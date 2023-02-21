// TODO - not being in the root module means some generated RTIC code is
// seen as dead_code
#![allow(dead_code)]

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::config::{
        self, NEIGHBOR_CACHE_LEN, ROUTING_TABLE_LEN, RX_RING_LEN, SOCKET_BUFFER_LEN, TX_RING_LEN,
    };
    use crate::display::Display;
    use crate::net::{EthernetDmaStorage, EthernetPhy, NetworkStorage, UdpSocketStorage};
    use crate::rtc::Rtc;
    use crate::sensors::{Sgp41, Sht31};
    use crate::shared_i2c::I2cDevices;
    use crate::tasks::{
        data_manager_task, eth_interrupt_handler_task, eth_link_status_timer_task,
        ipstack_clock_timer_task, ipstack_poll_task, ipstack_poll_timer_task, sgp41_task,
        sht31_task, SpawnArg,
    };
    use ieee802_3_miim::{phy::PhySpeed, Phy};
    use log::{debug, info, warn};
    use smoltcp::{
        iface::{Interface, InterfaceBuilder, NeighborCache, Routes, SocketHandle},
        socket::{UdpSocket, UdpSocketBuffer},
        wire::EthernetAddress,
    };
    use stm32_eth::{
        dma::EthernetDMA,
        mac::{EthernetMACWithMii, Speed},
        EthPins,
    };
    use stm32f4xx_hal::{
        gpio::{Output, PushPull, Speed as GpioSpeed, AF11, PA2, PB0, PB14, PB7, PC1},
        pac::{self, TIM10, TIM11, TIM3, TIM4},
        prelude::*,
        timer::counter::CounterHz,
        timer::{DelayUs, Event, MonoTimerUs, SysCounterUs, SysEvent},
    };

    type LedGreenPin = PB0<Output<PushPull>>;
    type LedBluePin = PB7<Output<PushPull>>;
    type LedRedPin = PB14<Output<PushPull>>;

    type MdioPin = PA2<AF11>;
    type MdcPin = PC1<AF11>;

    #[shared]
    struct Shared {
        #[lock_free]
        net: Interface<'static, &'static mut EthernetDMA<'static, 'static>>,

        #[lock_free]
        udp_socket: SocketHandle,

        #[lock_free]
        i2c_devices: I2cDevices<DelayUs<TIM10>, DelayUs<TIM11>>,
    }

    #[local]
    struct Local {
        // TODO - move these LEDs around
        led_b: LedBluePin,
        led_r: LedRedPin,

        link_led: LedGreenPin,
        phy: EthernetPhy<EthernetMACWithMii<MdioPin, MdcPin>>,

        net_clock_timer: SysCounterUs,
        ipstack_poll_timer: CounterHz<TIM3>,
        eth_link_status_timer: CounterHz<TIM4>,

        rtc: Rtc,
    }

    // TODO
    //#[monotonic(binds = TIM2, priority = 3, default = true)]
    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    // TODO - local type aliases with defaults to clean this up a bit
    #[init(local = [
        eth_dma_storage: EthernetDmaStorage<RX_RING_LEN, TX_RING_LEN> = EthernetDmaStorage::new(),
        net_storage: NetworkStorage<NEIGHBOR_CACHE_LEN, ROUTING_TABLE_LEN, 1> = NetworkStorage::new(config::SRC_IP_CIDR),
        udp_socket_storage: UdpSocketStorage<SOCKET_BUFFER_LEN> = UdpSocketStorage::new(),
        eth_dma: core::mem::MaybeUninit<EthernetDMA<'static, 'static>> = core::mem::MaybeUninit::uninit(),
    ])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("Starting");

        // Set up the system clock
        // HCLK must be at least 25MHz to use the ethernet peripheral
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.hclk(64.MHz()).sysclk(180.MHz()).freeze();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();
        let gpiod = ctx.device.GPIOD.split();
        let gpiof = ctx.device.GPIOF.split();
        let gpiog = ctx.device.GPIOG.split();

        let mut link_led = gpiob.pb0.into_push_pull_output();
        let mut led_b = gpiob.pb7.into_push_pull_output();
        let mut led_r = gpiob.pb14.into_push_pull_output();
        link_led.set_low();
        led_b.set_low();
        led_r.set_low();

        // Setup logging impl via USART3, Rx on PD9, Tx on PD8
        // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
        let log_tx_pin = gpiod.pd8.into_alternate();
        let log_tx = ctx
            .device
            .USART3
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

        // DS3231 RTC on I2C1
        // TODO - does it have an on-board pull-up?
        info!("Setup: I2C1");
        let scl = gpiob.pb8.into_alternate().set_open_drain();
        let sda = gpiob.pb9.into_alternate().set_open_drain();
        let i2c1 = ctx.device.I2C1.i2c((scl, sda), 100.kHz(), &clocks);
        info!("Setup: DS3231 RTC");
        let rtc = Rtc::new(i2c1).unwrap();

        // Shared I2C2 bus
        // - SSH1106
        let bus_manager: &'static _ = {
            use crate::shared_i2c::I2c;
            info!("Setup: I2C2");
            let scl = gpiof.pf1.into_alternate().set_open_drain();
            let sda = gpiof.pf0.into_alternate().set_open_drain();
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
        let mdio_pin = gpioa.pa2.into_alternate().speed(GpioSpeed::VeryHigh);
        let mdc_pin = gpioc.pc1.into_alternate().speed(GpioSpeed::VeryHigh);

        let eth_pins = EthPins {
            ref_clk: gpioa.pa1,
            crs: gpioa.pa7,
            tx_en: gpiog.pg11,
            tx_d0: gpiog.pg13,
            tx_d1: gpiob.pb13,
            rx_d0: gpioc.pc4,
            rx_d1: gpioc.pc5,
        };
        let eth_periph_parts = (
            ctx.device.ETHERNET_MAC,
            ctx.device.ETHERNET_MMC,
            ctx.device.ETHERNET_DMA,
        );

        let stm32_eth::Parts { dma, mac } = stm32_eth::new_with_mii(
            eth_periph_parts.into(),
            &mut ctx.local.eth_dma_storage.rx_ring[..],
            &mut ctx.local.eth_dma_storage.tx_ring[..],
            clocks,
            eth_pins,
            mdio_pin,
            mdc_pin,
        )
        .unwrap();

        let eth_dma = ctx.local.eth_dma.write(dma);

        let mut phy = if let Ok(phy) = EthernetPhy::from_miim(mac, 0) {
            phy
        } else {
            panic!("Unsupported PHY. Cannot detect link speed.");
        };
        info!("Setup: PHY, type {}", phy.ident_string());

        phy.phy_init();

        // TODO - don't fail if link is down on init
        info!("Setup: waiting for link");
        while !phy.phy_link_up() {
            cortex_m::asm::delay(100000);
        }

        link_led.set_high();
        info!("Setup: link up");

        if let Some(speed) = phy.speed().map(|s| match s {
            PhySpeed::HalfDuplexBase10T => Speed::HalfDuplexBase10T,
            PhySpeed::FullDuplexBase10T => Speed::FullDuplexBase10T,
            PhySpeed::HalfDuplexBase100Tx => Speed::HalfDuplexBase100Tx,
            PhySpeed::FullDuplexBase100Tx => Speed::FullDuplexBase100Tx,
        }) {
            phy.get_miim().set_speed(speed);
            info!("Setup: detected link speed: {:?}", speed);
        } else {
            warn!("Setup: failed to detect link speed.");
        }

        eth_dma.enable_interrupt();

        info!("Setup: TCP/IP");
        let mac = EthernetAddress::from_bytes(&config::SRC_MAC);
        info!("IP: {} MAC: {}", config::SRC_IP_CIDR.address(), mac);
        let neighbor_cache = NeighborCache::new(&mut ctx.local.net_storage.neighbor_storage[..]);
        let routes = Routes::new(&mut ctx.local.net_storage.routes_storage[..]);
        let mut eth_iface = InterfaceBuilder::new(eth_dma, &mut ctx.local.net_storage.sockets[..])
            .hardware_addr(mac.into())
            .ip_addrs(&mut ctx.local.net_storage.ip_addrs[..])
            .neighbor_cache(neighbor_cache)
            .routes(routes)
            .finalize();

        let udp_rx_buf = UdpSocketBuffer::new(
            &mut ctx.local.udp_socket_storage.rx_metadata[..],
            &mut ctx.local.udp_socket_storage.rx_buffer[..],
        );
        let udp_tx_buf = UdpSocketBuffer::new(
            &mut ctx.local.udp_socket_storage.tx_metadata[..],
            &mut ctx.local.udp_socket_storage.tx_buffer[..],
        );
        let udp_socket = UdpSocket::new(udp_rx_buf, udp_tx_buf);
        let udp_handle = eth_iface.add_socket(udp_socket);

        info!("Setup: net clock timer");
        let mut net_clock_timer = ctx.core.SYST.counter_us(&clocks);
        net_clock_timer.start(1.millis()).unwrap();
        net_clock_timer.listen(SysEvent::Update);

        info!("Setup: net poll timer");
        let mut ipstack_poll_timer = ctx.device.TIM3.counter_hz(&clocks);
        ipstack_poll_timer.start(20.Hz()).unwrap();
        ipstack_poll_timer.listen(Event::Update);

        info!("Setup: net link check timer");
        let mut eth_link_status_timer = ctx.device.TIM4.counter_hz(&clocks);
        eth_link_status_timer.start(1.Hz()).unwrap();
        eth_link_status_timer.listen(Event::Update);

        let mono = ctx.device.TIM2.monotonic_us(&clocks);
        info!("Initialized");

        // TODO - move this to a wrapper task that schedules all the sensor tasks
        sht31_task::spawn_after(Sht31::<(), ()>::MEASUREMENT_PERIOD_MS.millis()).unwrap();
        sgp41_task::spawn_after(Sgp41::<(), ()>::MEASUREMENT_PERIOD_MS.millis()).unwrap();
        data_manager_task::spawn_after(2.secs(), SpawnArg::SendData).unwrap();

        (
            Shared {
                net: eth_iface,
                udp_socket: udp_handle,
                i2c_devices,
            },
            Local {
                led_b,
                led_r,
                link_led,
                phy,
                net_clock_timer,
                eth_link_status_timer,
                ipstack_poll_timer,
                rtc,
            },
            init::Monotonics(mono),
        )
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
        #[task(local = [rtc], shared = [net, udp_socket], capacity = 6)]
        fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg);
    }

    extern "Rust" {
        #[task(binds = SysTick, local = [net_clock_timer])]
        fn ipstack_clock_timer_task(ctx: ipstack_clock_timer_task::Context);
    }

    extern "Rust" {
        #[task(shared = [net], capacity = 2)]
        fn ipstack_poll_task(ctx: ipstack_poll_task::Context);
    }

    extern "Rust" {
        #[task(binds = TIM3, local = [ipstack_poll_timer])]
        fn ipstack_poll_timer_task(ctx: ipstack_poll_timer_task::Context);
    }

    extern "Rust" {
        #[task(binds = ETH, shared = [net])]
        fn eth_interrupt_handler_task(ctx: eth_interrupt_handler_task::Context);
    }

    extern "Rust" {
        #[task(binds = TIM4, local = [link_led, phy, eth_link_status_timer, prev_link_status: bool = false])]
        fn eth_link_status_timer_task(ctx: eth_link_status_timer_task::Context);
    }
}
