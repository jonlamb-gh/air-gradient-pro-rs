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
    use crate::net::{
        Eth, EthernetStorage, NetworkStorage, SpiPins as EthSpiPins, TcpSocketStorage,
        UdpSocketStorage,
    };
    use crate::sensors::{Pms5003, Pms5003SerialPins, S8Lp, S8LpSerialPins, Sgp41, Sht31};
    use crate::shared_i2c::{I2cDevices, I2cPins};
    use crate::tasks::{
        data_manager::{SpawnArg as DataManagerSpawnArg, TaskState as DataManagerTaskState},
        data_manager_task,
        display::{SpawnArg as DisplaySpawnArg, TaskState as DisplayTaskState},
        display_task, eth_gpio_interrupt_handler_task, ipstack_clock_timer_task, ipstack_poll_task,
        ipstack_poll_timer_task,
        pms5003::TaskState as Pms5003TaskState,
        pms5003_task, s8lp_task,
        sgp41::{SpawnArg as Sgp41SpawnArg, TaskState as Sgp41TaskState},
        sgp41_task, sht31_task,
        update_manager::TaskState as UpdateManagerTaskState,
        update_manager_task, watchdog_task,
    };
    use crate::{config, util};
    use bootloader_lib::{BootConfig, ResetReasonExt, UpdateConfigAndStatus};
    use bootloader_support::ResetReason;
    use log::{debug, error, info};
    use smoltcp::{
        iface::{Config, Interface, SocketHandle, SocketSet},
        socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer},
        socket::udp::{PacketBuffer as UdpPacketBuffer, Socket as UdpSocket},
        wire::{EthernetAddress, Ipv4Address},
    };
    use stm32f4xx_hal::{
        crc32::Crc32,
        gpio::{Edge, Output, PushPull, Speed as GpioSpeed, PC13},
        pac::{self, FLASH, TIM10, TIM11, TIM3},
        prelude::*,
        rcc::Enable,
        spi::Spi,
        timer::counter::CounterHz,
        timer::{DelayUs, Event, MonoTimerUs, SysCounterUs, SysEvent},
        watchdog::IndependentWatchdog,
    };
    use update_manager::DeviceInfo;

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
        bcast_socket: SocketHandle,
        #[lock_free]
        device_socket: SocketHandle,
        #[lock_free]
        i2c_devices: I2cDevices<DelayUs<TIM10>, DelayUs<TIM11>>,
    }

    #[local]
    struct Local {
        net_clock_timer: SysCounterUs,
        ipstack_poll_timer: CounterHz<TIM3>,
        pms: Pms5003,
        s8lp: S8Lp,
        led: LedPin,
        watchdog: IndependentWatchdog,
        device_info: DeviceInfo,
        flash: FLASH,
    }

    // TODO use MonoTimer64Us with 64 bit timer
    /// TIM2 is a 32-bit timer, defaults to having the highest interrupt priority
    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init(local = [
        eth_storage: EthernetStorage<{Eth::MTU}> = EthernetStorage::new(),
        net_storage: NetworkStorage<2> = NetworkStorage::new(),
        udp_socket_storage: UdpSocketStorage<{config::BCAST_PROTO_SOCKET_BUFFER_LEN}> = UdpSocketStorage::new(),
        tcp_socket_storage: TcpSocketStorage<{config::DEVICE_PROTO_SOCKET_BUFFER_LEN}> = TcpSocketStorage::new(),
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let reset_reason: ResetReason = ResetReason::read_and_clear(&mut ctx.device.RCC);

        let mut syscfg = ctx.device.SYSCFG.constrain();
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

        let mut watchdog = IndependentWatchdog::new(ctx.device.IWDG);
        watchdog.start(config::WATCHDOG_RESET_PERIOD_MS.millis());
        watchdog.feed();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();

        // Turn it off, active-low
        let led: LedPin = gpioc.pc13.into_push_pull_output_in_state(true.into());

        // Setup logging impl via USART6, Rx on PA12, Tx on PA11
        // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
        let log_tx_pin = gpioa.pa11.into_alternate();
        let log_tx = ctx
            .device
            .USART6
            .tx(log_tx_pin, 115_200.bps(), &clocks)
            .unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

        // Read and clear UCS flags
        let update_pending = UpdateConfigAndStatus::update_pending();

        debug!("Watchdog: inerval {}", watchdog.interval());

        info!("############################################################");
        info!(
            "{} {} ({})",
            crate::built_info::PKG_NAME,
            config::FIRMWARE_VERSION,
            crate::built_info::PROFILE
        );
        info!("Build date: {}", crate::built_info::BUILT_TIME_UTC);
        info!("Compiler: {}", crate::built_info::RUSTC_VERSION);
        if let Some(gc) = crate::built_info::GIT_COMMIT_HASH {
            info!("Commit: {}", gc);
        }
        info!("Serial number: {:X}", util::read_device_serial_number());
        info!(
            "Device ID: 0x{:X} ({})",
            config::DEVICE_ID,
            config::DEVICE_ID
        );
        info!("IP address: {}", config::IP_CIDR.address());
        info!(
            "MAC address: {}",
            EthernetAddress::from_bytes(&config::MAC_ADDRESS)
        );
        info!("Broadcast protocol port: {}", config::BROADCAST_PORT);
        info!(
            "Broadcast protocol address: {}",
            Ipv4Address(config::BROADCAST_ADDRESS)
        );
        info!("Device protocol port: {}", config::DEVICE_PORT);
        info!("Reset reason: {reset_reason}");
        info!("Update pending: {update_pending}");
        info!("############################################################");

        if update_pending && reset_reason != ResetReason::SoftwareReset {
            error!("Aborting application update due to wrong reset reason");
            UpdateConfigAndStatus::clear();
            unsafe {
                crate::logger::flush_logger();
                let rcc = &(*pac::RCC::ptr());
                pac::USART6::disable(rcc);
            }
            // Let the watchdog reset to indicate a failure to the bootloader
            let _ = watchdog;
            loop {
                cortex_m::asm::nop();
            }
        }

        let mut common_delay = ctx.device.TIM4.delay_ms(&clocks);

        info!(
            "Setup: startup delay {} seconds",
            config::STARTUP_DELAY_SECONDS
        );
        for _ in 0..config::STARTUP_DELAY_SECONDS {
            for _ in 0..10 {
                watchdog.feed();
                common_delay.delay_ms(100_u8);
            }
        }
        watchdog.feed();

        info!("Setup: boot config");
        let flash = ctx.device.FLASH;
        let mut crc = Crc32::new(ctx.device.CRC);
        let boot_cfg = BootConfig::read(&flash, &mut crc).unwrap();

        info!("Setup: S8 LP");
        let tx = gpioa.pa9.into_alternate();
        let rx = gpioa.pa10.into_alternate();
        let pins: S8LpSerialPins = (tx, rx);
        let s8_serial = ctx.device.USART1.serial(pins, 9600.bps(), &clocks).unwrap();
        let s8lp = S8Lp::new(s8_serial);

        info!("Setup: PMS5003");
        let tx = gpioa.pa2.into_alternate();
        let rx = gpioa.pa3.into_alternate();
        let pins: Pms5003SerialPins = (tx, rx);
        let pms_serial = ctx.device.USART2.serial(pins, 9600.bps(), &clocks).unwrap();
        let pms = Pms5003::new(pms_serial, &mut common_delay).unwrap();

        // Shared I2C2 bus
        info!("Setup: I2C2");
        let bus_manager: &'static _ = {
            use crate::shared_i2c::I2c;
            let scl = gpiob.pb10.into_alternate().set_open_drain();
            let sda = gpiob.pb3.into_alternate().set_open_drain();
            let pins: I2cPins = (scl, sda);
            let i2c2 = ctx.device.I2C2.i2c(pins, 100.kHz(), &clocks);
            shared_bus::new_atomic_check!(I2c = i2c2).unwrap()
        };

        let sht31_delay = ctx.device.TIM10.delay_us(&clocks);
        let sgp41_delay = ctx.device.TIM11.delay_us(&clocks);

        let i2c_devices = {
            info!("Setup: SH1106");
            let display = Display::new(bus_manager.acquire_i2c()).unwrap();
            info!("Setup: SHT31");
            let sht31 = Sht31::new(bus_manager.acquire_i2c(), sht31_delay).unwrap();
            debug!("SHT31: serial number {}", sht31.serial_number());
            info!("Setup: SGP41");
            let sgp41 = Sgp41::new(bus_manager.acquire_i2c(), sgp41_delay).unwrap();
            debug!("SGP41: serial number {}", sgp41.serial_number());

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
            let pins: EthSpiPins = (sck, miso, mosi);

            // TODO 3 MHz
            Spi::new(ctx.device.SPI2, pins, enc28j60::MODE, 1.MHz(), &clocks)
        };
        let mut eth = {
            let ncs = gpiob.pb12.into_push_pull_output_in_state(true.into());

            let mut int = gpioa.pa8.into_pull_up_input();
            int.make_interrupt_source(&mut syscfg);
            int.enable_interrupt(&mut ctx.device.EXTI);
            int.trigger_on_edge(&mut ctx.device.EXTI, Edge::Falling);

            let mut reset = gpiob.pb1.into_push_pull_output_in_state(true.into());

            // Perform a hard reset first, then let the driver
            // perform a soft reset by provided enc28j60::Unconnected
            // instead of the actual reset pin
            reset.set_low();
            common_delay.delay_ms(5_u8);
            reset.set_high();
            common_delay.delay_ms(5_u8);

            let mut enc = enc28j60::Enc28j60::new(
                eth_spi,
                ncs,
                int,
                enc28j60::Unconnected,
                &mut common_delay,
                7168,
                config::MAC_ADDRESS,
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
        let mac = EthernetAddress::from_bytes(&config::MAC_ADDRESS);
        let config = Config::new(mac.into());
        let mut eth_iface = Interface::new(config, &mut eth, smoltcp::time::Instant::ZERO);
        eth_iface.update_ip_addrs(|addr| {
            addr.push(config::IP_CIDR.into()).unwrap();
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
        let bcast_socket = sockets.add(udp_socket);

        let tcp_rx_buf = TcpSocketBuffer::new(&mut ctx.local.tcp_socket_storage.rx_buffer[..]);
        let tcp_tx_buf = TcpSocketBuffer::new(&mut ctx.local.tcp_socket_storage.tx_buffer[..]);
        let tcp_socket = TcpSocket::new(tcp_rx_buf, tcp_tx_buf);
        let device_socket = sockets.add(tcp_socket);

        info!("Setup: net clock timer");
        let mut net_clock_timer = ctx.core.SYST.counter_us(&clocks);
        net_clock_timer.start(1.millis()).unwrap();
        net_clock_timer.listen(SysEvent::Update);

        info!("Setup: net poll timer");
        let mut ipstack_poll_timer = ctx.device.TIM3.counter_hz(&clocks);
        ipstack_poll_timer.start(25.Hz()).unwrap();
        ipstack_poll_timer.listen(Event::Update);

        let mono = ctx.device.TIM2.monotonic_us(&clocks);
        info!(">>> Initialized <<<");
        watchdog.feed();

        // If we've made it this far, assume it's ok to mark this firmware image slot as
        // valid
        // NOTE: could move this to a task and perform the op later to give things a chance
        // to gain more coverage
        if update_pending && reset_reason == ResetReason::SoftwareReset {
            info!("New application update checks out, marking for BC flash and reseting");
            UpdateConfigAndStatus::set_update_pending();
            UpdateConfigAndStatus::set_update_valid();
            watchdog.feed();
            unsafe {
                crate::logger::flush_logger();
                let rcc = &(*pac::RCC::ptr());
                pac::USART6::disable(rcc);

                bootloader_lib::sw_reset();
            }
        }

        let device_info = util::device_info(boot_cfg.firmware_boot_slot(), reset_reason);

        watchdog_task::spawn().unwrap();
        display_task::spawn(DisplaySpawnArg::Startup).unwrap();
        sht31_task::spawn().unwrap();
        sgp41_task::spawn(Sgp41SpawnArg::Measurement).unwrap();
        pms5003_task::spawn().unwrap();
        s8lp_task::spawn().unwrap();

        data_manager_task::spawn_after(
            config::BCAST_INTERVAL_SEC.secs(),
            DataManagerSpawnArg::SendBroadcastMessage,
        )
        .unwrap();

        update_manager_task::spawn_after(config::UPDATE_MANAGER_POLL_INTERVAL_MS.millis()).unwrap();

        (
            Shared {
                eth,
                net: eth_iface,
                sockets,
                bcast_socket,
                device_socket,
                i2c_devices,
            },
            Local {
                net_clock_timer,
                ipstack_poll_timer,
                pms,
                s8lp,
                led,
                watchdog,
                device_info,
                flash,
            },
            init::Monotonics(mono),
        )
    }

    extern "Rust" {
        #[task(local = [watchdog, led])]
        fn watchdog_task(ctx: watchdog_task::Context);
    }

    extern "Rust" {
        #[task(local = [state: DisplayTaskState = DisplayTaskState::new()], shared = [i2c_devices], capacity = 4)]
        fn display_task(ctx: display_task::Context, arg: DisplaySpawnArg);
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
        #[task(local = [state: DataManagerTaskState = DataManagerTaskState::new()], shared = [sockets, bcast_socket], capacity = 8)]
        fn data_manager_task(ctx: data_manager_task::Context, arg: DataManagerSpawnArg);
    }

    extern "Rust" {
        #[task(
              local = [state: UpdateManagerTaskState = UpdateManagerTaskState::new(), device_info, flash],
              shared = [sockets, device_socket])
          ]
        fn update_manager_task(ctx: update_manager_task::Context);
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
