//#![deny(warnings, clippy::all)]
// TODO
//#![forbid(unsafe_code)]
#![no_main]
#![no_std]

mod config;
mod display;
mod logger;
mod net;
mod panic_handler;
mod rtc;
mod sensors;
mod shared_i2c;
mod tasks;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::config::{self, SOCKET_BUFFER_LEN};
    use crate::display::Display;
    use crate::net::{Eth, EthernetStorage, NetworkStorage, UdpSocketStorage};
    use crate::rtc::Rtc;
    use crate::sensors::{Sgp41, Sht31};
    use crate::shared_i2c::I2cDevices;
    use crate::tasks::data_manager::default_bcast_message;
    use crate::tasks::{
        data_manager_task, ipstack_clock_timer_task, ipstack_poll_task, ipstack_poll_timer_task,
        sgp41_task, sht31_task, SpawnArg,
    };
    use log::{debug, info, warn};
    use smoltcp::{
        iface::{Config, Interface, SocketHandle, SocketSet},
        socket::udp::{PacketBuffer as UdpPacketBuffer, Socket as UdpSocket},
        wire::EthernetAddress,
    };
    use stm32f4xx_hal::{
        gpio::{PushPull, Speed as GpioSpeed, AF7, PA10, PA2, PA3, PA9},
        pac::{self, TIM10, TIM11, TIM3, USART1, USART2},
        prelude::*,
        serial::Serial,
        spi::Spi,
        timer::counter::CounterHz,
        timer::{DelayUs, Event, MonoTimerUs, SysCounterUs, SysEvent},
    };
    use wire_protocols::broadcast::Repr as Message;

    //type LedGreenPin = PB0<Output<PushPull>>;

    #[shared]
    struct Shared {
        #[lock_free]
        eth: Eth<'static>,
        #[lock_free]
        net: Interface,
        #[lock_free]
        sockets: SocketSet<'static>,
        #[lock_free]
        udp_socket: SocketHandle,
        #[lock_free]
        i2c_devices: I2cDevices<DelayUs<TIM10>, DelayUs<TIM11>>,
    }

    #[local]
    struct Local {
        net_clock_timer: SysCounterUs,
        ipstack_poll_timer: CounterHz<TIM3>,

        rtc: Rtc,

        // TODO
        s8_serial: Serial<USART1, (PA9<AF7<PushPull>>, PA10<AF7<PushPull>>), u8>,
        // TODO
        pms_serial: Serial<USART2, (PA2<AF7<PushPull>>, PA3<AF7<PushPull>>), u8>,
    }

    // TODO
    //#[monotonic(binds = TIM2, priority = 3, default = true)]
    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init(local = [
        eth_storage: EthernetStorage<{Eth::MTU}> = EthernetStorage::new(),
        net_storage: NetworkStorage<1> = NetworkStorage::new(),
        udp_socket_storage: UdpSocketStorage<SOCKET_BUFFER_LEN> = UdpSocketStorage::new(),
    ])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("Starting");

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();

        // Setup logging impl via USART6, Rx on PA12, Tx on PA11
        // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
        let log_tx_pin = gpioa.pa11.into_alternate();
        let log_tx = ctx
            .device
            .USART6
            .tx(log_tx_pin, 115_200.bps(), &clocks)
            .unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

        info!("############################################################");
        info!(
            "{} {} ({})",
            crate::built_info::PKG_NAME,
            crate::built_info::PKG_VERSION,
            crate::built_info::PROFILE
        );
        info!("Build date: {}", crate::built_info::BUILT_TIME_UTC);
        info!("{}", crate::built_info::RUSTC_VERSION);
        if let Some(gc) = crate::built_info::GIT_COMMIT_HASH {
            info!("git commit: {}", gc);
        }
        info!("############################################################");

        let mut common_delay = ctx.device.TIM4.delay_ms(&clocks);

        // TODO
        info!("Setup: S8 LP");
        let tx = gpioa.pa9.into_alternate();
        let rx = gpioa.pa10.into_alternate();
        let s8_serial = ctx
            .device
            .USART1
            .serial((tx, rx), 9600.bps(), &clocks)
            .unwrap();

        info!("Setup: PMS5003");
        let tx = gpioa.pa2.into_alternate();
        let rx = gpioa.pa3.into_alternate();
        let pms_serial = ctx
            .device
            .USART2
            .serial((tx, rx), 9600.bps(), &clocks)
            .unwrap();

        // DS3231 RTC on I2C1
        // TODO - does it have an on-board pull-up?
        info!("Setup: I2C1");
        let scl = gpiob.pb6.into_alternate().set_open_drain();
        let sda = gpiob.pb7.into_alternate().set_open_drain();
        let i2c1 = ctx.device.I2C1.i2c((scl, sda), 100.kHz(), &clocks);
        info!("Setup: DS3231 RTC");
        let rtc = Rtc::new(i2c1).unwrap();

        // Shared I2C2 bus
        info!("Setup: I2C2");
        let bus_manager: &'static _ = {
            use crate::shared_i2c::I2c;
            let scl = gpiob.pb10.into_alternate().set_open_drain();
            let sda = gpiob.pb3.into_alternate().set_open_drain();
            let i2c2 = ctx.device.I2C2.i2c((scl, sda), 100.kHz(), &clocks);
            shared_bus::new_atomic_check!(I2c = i2c2).unwrap()
        };

        // TODO - renode STM32_Timer (LTimer) issues
        // the hal timer/delay.rs impl is blocked on
        // CEN bit to clear
        // timer10 enableRequested False
        //
        // could also adjust timer11 frequency in script to make it finish faster
        let sht31_delay = ctx.device.TIM10.delay_us(&clocks);
        let sgp41_delay = ctx.device.TIM11.delay_us(&clocks);

        let i2c_devices = {
            info!("Setup: SH1106");
            let display = Display::new(bus_manager.acquire_i2c()).unwrap();
            info!("Setup: SHT31");
            let sht31 = Sht31::new(bus_manager.acquire_i2c(), sht31_delay).unwrap();
            info!("Setup: SGP41");
            let sgp41 = Sgp41::new(bus_manager.acquire_i2c(), sgp41_delay).unwrap();

            I2cDevices {
                display,
                sht31,
                sgp41,
            }
        };

        info!("Setup: ETH");
        let eth_spi = {
            let sck = gpiob.pb13.into_alternate().speed(GpioSpeed::VeryHigh);
            let miso = gpiob.pb14.into_alternate().speed(GpioSpeed::VeryHigh);
            let mosi = gpiob
                .pb15
                .into_alternate()
                .speed(GpioSpeed::VeryHigh)
                .internal_pull_up(true);

            // TODO 3 MHz
            Spi::new(
                ctx.device.SPI2,
                (sck, miso, mosi),
                enc28j60::MODE,
                1.MHz(),
                &clocks,
            )
        };
        let mut eth = {
            // TODO GPIO interrupt

            let mut ncs = gpiob.pb12.into_push_pull_output();
            ncs.set_high();
            let int = gpioa.pa8.into_pull_up_input();
            let mut reset = gpiob.pb1.into_push_pull_output();
            reset.set_high();

            let mut enc = enc28j60::Enc28j60::new(
                eth_spi,
                ncs,
                int,
                reset,
                &mut common_delay,
                7168,
                config::SRC_MAC,
            )
            .unwrap();

            // TODO - renode issue with ints vs not-connected/polling?
            //enc.listen(enc28j60::Event::Pkt).unwrap();

            Eth::new(
                enc,
                &mut ctx.local.eth_storage.rx_buffer[..],
                &mut ctx.local.eth_storage.tx_buffer[..],
            )
        };

        info!("Setup: TCP/IP");
        let mac = EthernetAddress::from_bytes(&config::SRC_MAC);
        info!("IP: {} MAC: {}", config::SRC_IP_CIDR.address(), mac);
        let mut config = Config::new();
        config.hardware_addr = Some(mac.into());
        let mut eth_iface = Interface::new(config, &mut eth);
        eth_iface.update_ip_addrs(|addr| {
            addr.push(config::SRC_IP_CIDR.into()).unwrap();
        });
        let mut sockets = SocketSet::new(&mut ctx.local.net_storage.sockets[..]);
        let udp_rx_buf = UdpPacketBuffer::new(
            &mut ctx.local.udp_socket_storage.rx_metadata[..],
            &mut ctx.local.udp_socket_storage.rx_buffer[..],
        );
        let udp_tx_buf = UdpPacketBuffer::new(
            &mut ctx.local.udp_socket_storage.tx_metadata[..],
            &mut ctx.local.udp_socket_storage.tx_buffer[..],
        );
        let udp_socket = UdpSocket::new(udp_rx_buf, udp_tx_buf);
        let udp_handle = sockets.add(udp_socket);

        info!("Setup: net clock timer");
        let mut net_clock_timer = ctx.core.SYST.counter_us(&clocks);
        net_clock_timer.start(1.millis()).unwrap();
        net_clock_timer.listen(SysEvent::Update);

        info!("Setup: net poll timer");
        let mut ipstack_poll_timer = ctx.device.TIM3.counter_hz(&clocks);
        ipstack_poll_timer.start(25.Hz()).unwrap();
        ipstack_poll_timer.listen(Event::Update);

        let mono = ctx.device.TIM2.monotonic_us(&clocks);
        info!("Initialized");

        // TODO - move this to a wrapper task that schedules all the sensor tasks
        data_manager_task::spawn_after(2.secs(), SpawnArg::SendData).unwrap();

        sensor_measurements_task::spawn_after(config::MEASUREMENT_PERIOD_MS.millis()).unwrap();

        (
            Shared {
                eth,
                net: eth_iface,
                sockets,
                udp_socket: udp_handle,
                i2c_devices,
            },
            Local {
                net_clock_timer,
                ipstack_poll_timer,
                rtc,
                s8_serial,
                pms_serial,
            },
            init::Monotonics(mono),
        )
    }

    #[task]
    fn sensor_measurements_task(_ctx: sensor_measurements_task::Context) {
        sht31_task::spawn().ok();
        sgp41_task::spawn().ok();
        sensor_measurements_task::spawn_after(config::MEASUREMENT_PERIOD_MS.millis()).unwrap();
    }

    extern "Rust" {
        #[task(shared = [i2c_devices])]
        fn sht31_task(ctx: sht31_task::Context);
    }

    extern "Rust" {
        #[task(shared = [i2c_devices])]
        fn sgp41_task(ctx: sgp41_task::Context);
    }

    extern "Rust" {
        #[task(local = [rtc, msg: Message = default_bcast_message()], shared = [net, sockets, udp_socket], capacity = 6)]
        fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg);
    }

    extern "Rust" {
        #[task(binds = SysTick, local = [net_clock_timer])]
        fn ipstack_clock_timer_task(ctx: ipstack_clock_timer_task::Context);
    }

    extern "Rust" {
        #[task(shared = [eth, net, sockets], capacity = 2)]
        fn ipstack_poll_task(ctx: ipstack_poll_task::Context);
    }

    extern "Rust" {
        #[task(binds = TIM3, local = [ipstack_poll_timer])]
        fn ipstack_poll_timer_task(ctx: ipstack_poll_timer_task::Context);
    }
}
