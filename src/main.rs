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
mod sensors;
mod shared_i2c;
mod tasks;
mod util;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::display::Display;
    use crate::net::{Eth, EthernetStorage, NetworkStorage, UdpSocketStorage};
    use crate::sensors::{Pms5003, S8Lp, Sgp41, Sht31};
    use crate::shared_i2c::I2cDevices;
    use crate::tasks::{
        data_manager::default_bcast_message,
        data_manager::SpawnArg as DataManagerSpawnArg,
        data_manager_task, eth_gpio_interrupt_handler_task, ipstack_clock_timer_task,
        ipstack_poll_task, ipstack_poll_timer_task,
        pms5003::TaskState as Pms5003TaskState,
        pms5003_task, s8lp_task,
        sgp41::{SpawnArg as Sgp41SpawnArg, TaskState as Sgp41TaskState},
        sgp41_task, sht31_task,
    };
    use crate::{config, util};
    use log::info;
    use smoltcp::{
        iface::{Config, Interface, SocketHandle, SocketSet},
        socket::udp::{PacketBuffer as UdpPacketBuffer, Socket as UdpSocket},
        wire::EthernetAddress,
    };
    use stm32f4xx_hal::{
        gpio::{Edge, Output, PushPull, Speed as GpioSpeed, PC13},
        pac::{self, TIM10, TIM11, TIM3},
        prelude::*,
        spi::Spi,
        timer::counter::CounterHz,
        timer::{DelayUs, Event, MonoTimerUs, SysCounterUs, SysEvent},
    };
    use wire_protocols::broadcast::{self, Repr as Message};

    // TODO - blink LED every udp tx or something
    // maybe on the watchdog task instead
    type LedPin = PC13<Output<PushPull>>;

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
        pms: Pms5003,
        s8lp: S8Lp,
    }

    // TODO
    //#[monotonic(binds = TIM2, priority = 3, default = true)]
    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init(local = [
        eth_storage: EthernetStorage<{Eth::MTU}> = EthernetStorage::new(),
        net_storage: NetworkStorage<1> = NetworkStorage::new(),
        udp_socket_storage: UdpSocketStorage<{config::SOCKET_BUFFER_LEN}> = UdpSocketStorage::new(),
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        // TODO hook up the watchdog

        let mut syscfg = ctx.device.SYSCFG.constrain();
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();

        // Turn it off, active-low
        let _led: LedPin = gpioc.pc13.into_push_pull_output_in_state(true.into());

        // Setup logging impl via USART6, Rx on PA12, Tx on PA11
        // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
        let log_tx_pin = gpioa.pa11.into_alternate();
        let log_tx = ctx
            .device
            .USART6
            .tx(log_tx_pin, 115_200.bps(), &clocks)
            .unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

        // TODO add project stuff, gen in build.rs
        // default_bcast_message() things
        // ProtocolVersion
        // FirmwareVersion
        // device id
        // ...

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
        info!("Serial number: {:X}", util::read_device_serial_number());
        info!("Device ID: 0x{:X}", config::DEVICE_ID.as_u16());
        info!("IP address: {}", config::SRC_IP_CIDR.address());
        info!(
            "MAC address: {}",
            EthernetAddress::from_bytes(&config::SRC_MAC)
        );
        info!("Broadcast protocol port: {}", broadcast::DEFAULT_PORT);
        info!("############################################################");

        let mut common_delay = ctx.device.TIM4.delay_ms(&clocks);

        info!(
            "Setup: startup delay {} seconds",
            config::STARTUP_DELAY_SECONDS
        );
        for _ in 0..config::STARTUP_DELAY_SECONDS {
            for _ in 0..10 {
                common_delay.delay_ms(100_u8);
            }
        }

        info!("Setup: S8 LP");
        let tx = gpioa.pa9.into_alternate();
        let rx = gpioa.pa10.into_alternate();
        let s8_serial = ctx
            .device
            .USART1
            .serial((tx, rx), 9600.bps(), &clocks)
            .unwrap();
        let s8lp = S8Lp::new(s8_serial);

        info!("Setup: PMS5003");
        let tx = gpioa.pa2.into_alternate();
        let rx = gpioa.pa3.into_alternate();
        let pms_serial = ctx
            .device
            .USART2
            .serial((tx, rx), 9600.bps(), &clocks)
            .unwrap();
        let pms = Pms5003::new(pms_serial, &mut common_delay).unwrap();

        // Shared I2C2 bus
        info!("Setup: I2C2");
        let bus_manager: &'static _ = {
            use crate::shared_i2c::I2c;
            let scl = gpiob.pb10.into_alternate().set_open_drain();
            let sda = gpiob.pb3.into_alternate().set_open_drain();
            let i2c2 = ctx.device.I2C2.i2c((scl, sda), 100.kHz(), &clocks);
            shared_bus::new_atomic_check!(I2c = i2c2).unwrap()
        };

        let sht31_delay = ctx.device.TIM10.delay_us(&clocks);
        let sgp41_delay = ctx.device.TIM11.delay_us(&clocks);

        let i2c_devices = {
            info!("Setup: SH1106");
            let display = Display::new(bus_manager.acquire_i2c()).unwrap();
            info!("Setup: SHT31");
            let sht31 = Sht31::new(bus_manager.acquire_i2c(), sht31_delay).unwrap();
            info!("SHT31: serial number {}", sht31.serial_number());
            info!("Setup: SGP41");
            let sgp41 = Sgp41::new(bus_manager.acquire_i2c(), sgp41_delay).unwrap();
            info!("SGP41: serial number {}", sgp41.serial_number());

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
            let ncs = gpiob.pb12.into_push_pull_output_in_state(true.into());

            let mut int = gpioa.pa8.into_pull_up_input();
            int.make_interrupt_source(&mut syscfg);
            int.enable_interrupt(&mut ctx.device.EXTI);
            int.trigger_on_edge(&mut ctx.device.EXTI, Edge::Falling);

            let mut reset = gpiob.pb1.into_push_pull_output_in_state(true.into());
            // Hard reset
            reset.set_low();
            common_delay.delay_ms(5_u8);
            reset.set_high();
            common_delay.delay_ms(5_u8);

            let mut enc = enc28j60::Enc28j60::new(
                eth_spi,
                ncs,
                int,
                // Provide Unconnected so that a soft-reset also happens
                // TODO - still debugging why it occasionally gets into
                // CorruptRxBuffer and no more data
                // Probably just panic/reset when it happens until I
                // look into it more
                enc28j60::Unconnected,
                &mut common_delay,
                7168,
                config::SRC_MAC,
            )
            .unwrap();

            enc.listen(enc28j60::Event::Pkt).unwrap();

            Eth::new(
                enc,
                &mut ctx.local.eth_storage.rx_buffer[..],
                &mut ctx.local.eth_storage.tx_buffer[..],
            )
        };

        info!("Setup: TCP/IP");
        let mac = EthernetAddress::from_bytes(&config::SRC_MAC);
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

        sht31_task::spawn().unwrap();
        sgp41_task::spawn(Sgp41SpawnArg::Measurement).unwrap();
        pms5003_task::spawn().unwrap();
        s8lp_task::spawn().unwrap();

        data_manager_task::spawn_after(
            config::BCAST_INTERVAL_SEC.secs(),
            DataManagerSpawnArg::SendData,
        )
        .unwrap();

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
                pms,
                s8lp,
            },
            init::Monotonics(mono),
        )
    }

    extern "Rust" {
        #[task(shared = [i2c_devices])]
        fn sht31_task(ctx: sht31_task::Context);
    }

    extern "Rust" {
        #[task(local = [state: Sgp41TaskState = Sgp41TaskState::new()], shared = [i2c_devices], capacity = 4)]
        fn sgp41_task(ctx: sgp41_task::Context, arg: Sgp41SpawnArg);
    }

    extern "Rust" {
        #[task(local = [state: Pms5003TaskState = Pms5003TaskState::new(), pms])]
        fn pms5003_task(ctx: pms5003_task::Context);
    }

    extern "Rust" {
        #[task(local = [s8lp])]
        fn s8lp_task(ctx: s8lp_task::Context);
    }

    extern "Rust" {
        #[task(local = [msg: Message = default_bcast_message()], shared = [sockets, udp_socket], capacity = 8)]
        fn data_manager_task(ctx: data_manager_task::Context, arg: DataManagerSpawnArg);
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

    extern "Rust" {
        #[task(binds = EXTI9_5, shared = [eth])]
        fn eth_gpio_interrupt_handler_task(ctx: eth_gpio_interrupt_handler_task::Context);
    }
}
