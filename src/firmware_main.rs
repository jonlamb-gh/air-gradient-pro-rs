// TODO - not being in the root module means some generated RTIC code is
// seen as dead_code
#![allow(dead_code)]

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::firmware_main::net_clock::NetClock;
    use crate::net::{EthernetDmaStorage, EthernetPhy, NetworkStorage, UdpSocketStorage};
    use crate::rtc::Rtc;
    use ieee802_3_miim::{phy::PhySpeed, Phy};
    use log::{debug, info, warn};
    use smoltcp::{
        iface::{Interface, InterfaceBuilder, NeighborCache, Routes, SocketHandle},
        socket::{UdpSocket, UdpSocketBuffer},
        wire::{EthernetAddress, Ipv4Address, Ipv4Cidr},
    };
    use stm32_eth::{
        dma::EthernetDMA,
        mac::{EthernetMACWithMii, Speed},
        EthPins,
    };
    use stm32f4xx_hal::{
        gpio::{Output, PushPull, Speed as GpioSpeed, AF11, PA2, PB0, PB14, PB7, PC1},
        pac::{self, TIM3, TIM4, TIM5},
        prelude::*,
        timer::counter::{CounterHz, CounterUs},
        timer::{Event, MonoTimerUs},
    };

    type LedGreenPin = PB0<Output<PushPull>>;
    type LedBluePin = PB7<Output<PushPull>>;
    type LedRedPin = PB14<Output<PushPull>>;

    type MdioPin = PA2<AF11>;
    type MdcPin = PC1<AF11>;

    // TODO - use env vars + gen build-time for these configs
    // or put them in a flash section for configs
    // use renode script to setup flash config as needed
    const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x05, 0x06, 0x07, 0x08];
    //const SRC_IP: [u8; 4] = [192, 168, 1, 39];
    // TODO - for renode stuff: 192.0.2.29 02:00:05:06:07:08
    const SRC_IP: [u8; 4] = [192, 0, 2, 29];
    const SRC_IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(SRC_IP), 24);
    const UDP_PORT: u16 = 12345;

    // TODO move consts to config.rs so tests can use them
    const SOCKET_BUFFER_SIZE: usize = 256;
    const NEIGHBOR_CACHE_LEN: usize = 16;
    const ROUTING_TABLE_LEN: usize = 16;
    const RX_RING_LEN: usize = 16;
    const TX_RING_LEN: usize = 8;

    static NET_CLOCK: NetClock = NetClock::new();

    #[shared]
    struct Shared {
        #[lock_free]
        net: Interface<'static, &'static mut EthernetDMA<'static, 'static>>,
        #[lock_free]
        udp_socket: SocketHandle,
    }

    #[local]
    struct Local {
        led_b: LedBluePin,
        led_r: LedRedPin,

        link_led: LedGreenPin,
        phy: EthernetPhy<EthernetMACWithMii<MdioPin, MdcPin>>,

        net_clock_timer: CounterUs<TIM3>,
        net_link_check_timer: CounterHz<TIM4>,
        net_poll_timer: CounterHz<TIM5>,

        rtc: Rtc,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init(local = [
        eth_dma_storage: EthernetDmaStorage<RX_RING_LEN, TX_RING_LEN> = EthernetDmaStorage::new(),
        net_storage: NetworkStorage<NEIGHBOR_CACHE_LEN, ROUTING_TABLE_LEN, 1> = NetworkStorage::new(SRC_IP_CIDR),
        udp_socket_storage: UdpSocketStorage<SOCKET_BUFFER_SIZE> = UdpSocketStorage::new(),
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
        let mac = EthernetAddress::from_bytes(&SRC_MAC);
        info!("IP: {} MAC: {}", SRC_IP_CIDR.address(), mac);
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
        let mut net_clock_timer = ctx.device.TIM3.counter_us(&clocks);
        net_clock_timer.start(1.millis()).unwrap();
        net_clock_timer.listen(Event::Update);

        info!("Setup: net link check timer");
        let mut net_link_check_timer = ctx.device.TIM4.counter_hz(&clocks);
        net_link_check_timer.start(1.Hz()).unwrap();
        net_link_check_timer.listen(Event::Update);

        info!("Setup: net poll timer");
        let mut net_poll_timer = ctx.device.TIM5.counter_hz(&clocks);
        net_poll_timer.start(25.Hz()).unwrap();
        net_poll_timer.listen(Event::Update);

        let mono = ctx.device.TIM2.monotonic_us(&clocks);
        info!("Initialized");

        (
            Shared {
                net: eth_iface,
                udp_socket: udp_handle,
            },
            Local {
                led_b,
                led_r,
                link_led,
                phy,
                net_clock_timer,
                net_link_check_timer,
                net_poll_timer,
                rtc,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [led_b, led_r], shared = [net, udp_socket], priority = 3)]
    fn do_udp_stuff(ctx: do_udp_stuff::Context) {
        let led_b = ctx.local.led_b;
        let led_r = ctx.local.led_r;
        let net = ctx.shared.net;
        let udp_socket_handle = ctx.shared.udp_socket;
        let socket = net.get_socket::<UdpSocket>(*udp_socket_handle);
        if !socket.is_open() {
            info!("Binding to UDP port {UDP_PORT}");
            socket.bind(UDP_PORT).unwrap();
            led_b.set_high();
        }

        let mut endpoint = None;
        if let Ok((data, remote)) = socket.recv() {
            led_r.toggle();
            info!("Got {} bytes from {}", data.len(), remote);
            endpoint.replace(remote);
        }
        if let Some(remote) = endpoint.take() {
            socket.send_slice(b"hello", remote).unwrap();
        }
    }

    #[task(binds=TIM3, local = [net_clock_timer], priority = 4)]
    fn on_net_clock_timer(ctx: on_net_clock_timer::Context) {
        let timer = ctx.local.net_clock_timer;
        let _ = timer.wait();
        NET_CLOCK.inc_from_interrupt();
    }

    #[task(binds=TIM4, local = [link_led, phy, net_link_check_timer, rtc], priority = 3)]
    fn on_net_link_check_timer(ctx: on_net_link_check_timer::Context) {
        let link_led = ctx.local.link_led;
        let phy = ctx.local.phy;
        let timer = ctx.local.net_link_check_timer;
        let rtc = ctx.local.rtc;

        let _ = timer.wait();

        // Poll link status
        let link_status = if phy.phy_link_up() {
            link_led.set_high();
            true
        } else {
            link_led.set_low();
            false
        };

        let dt = rtc.datetime().unwrap();
        info!("link={}, dt={}", link_status, dt);
    }

    #[task(binds=TIM5, local = [net_poll_timer], priority = 1)]
    fn on_net_poll_timer(ctx: on_net_poll_timer::Context) {
        let timer = ctx.local.net_poll_timer;
        let _ = timer.wait();
        poll_ip_stack::spawn().ok();
        do_udp_stuff::spawn().ok();
    }

    #[task(binds = ETH, shared = [net], priority = 3)]
    fn on_eth(ctx: on_eth::Context) {
        let net = ctx.shared.net;
        net.device_mut().interrupt_handler();
        poll_ip_stack::spawn().ok();
    }

    #[task(shared = [net], capacity = 2, priority = 3)]
    fn poll_ip_stack(ctx: poll_ip_stack::Context) {
        let net = ctx.shared.net;
        let time = NET_CLOCK.get();
        match net.poll(time) {
            Ok(_something_happened) => (),
            Err(e) => debug!("{:?}", e),
        }
    }
}

// TODO
mod net_clock {
    use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use smoltcp::time::Instant;

    /// 32-bit millisecond clock
    #[derive(Debug)]
    #[repr(transparent)]
    pub struct NetClock(AtomicU32);

    impl NetClock {
        pub const fn new() -> Self {
            NetClock(AtomicU32::new(0))
        }

        pub fn inc_from_interrupt(&self) {
            self.0.fetch_add(1, Relaxed);
        }

        pub fn get_raw(&self) -> u32 {
            self.0.load(Relaxed)
        }

        pub fn get(&self) -> Instant {
            Instant::from_millis(self.get_raw() as i64)
        }
    }
}
