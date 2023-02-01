//#![deny(warnings, clippy::all)]
// TODO
//#![forbid(unsafe_code)]
#![no_main]
#![no_std]

//use panic_abort as _; // panic handler
//use panic_rtt_target as _; // panic handler

mod logger;
mod panic_handler;
mod phy;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::net_clock::NetClock;
    use crate::phy::EthernetPhy;
    use ieee802_3_miim::{phy::PhySpeed, Phy};
    use log::{debug, info, warn};
    use smoltcp::{
        iface::{
            Interface, InterfaceBuilder, Neighbor, NeighborCache, Route, Routes, SocketHandle,
            SocketStorage,
        },
        socket::{UdpPacketMetadata, UdpSocket, UdpSocketBuffer},
        wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv4Cidr},
    };
    use stm32_eth::{
        dma::{EthernetDMA, RxRingEntry, TxRingEntry},
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
    //use rtt_logger::RTTLogger;
    //use rtt_target::rtt_init_print;

    type LedGreenPin = PB0<Output<PushPull>>;
    type LedBluePin = PB7<Output<PushPull>>;
    type LedRedPin = PB14<Output<PushPull>>;

    type MdioPin = PA2<AF11>;
    type MdcPin = PC1<AF11>;

    const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x05, 0x06, 0x07, 0x08];
    const SRC_IP: [u8; 4] = [192, 168, 1, 39];
    // TODO - for renode stuff: 192.0.2.29
    //const SRC_IP: [u8; 4] = [192, 0, 2, 29];
    const SRC_IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(SRC_IP), 24);
    const UDP_PORT: u16 = 12345;

    const SOCKET_BUFFER_SIZE: usize = 256;
    const NEIGHBOR_CACHE_SIZE: usize = 16;
    const ROUTING_TABLE_SIZE: usize = 16;
    const RX_DESC_RING_SIZE: usize = 16;
    const TX_DESC_RING_SIZE: usize = 8;

    //static mut LOGGER: Logger<USART3> = Logger::new();
    //static LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Trace);
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
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    const RX_DESC_INIT: RxRingEntry = RxRingEntry::new();
    const TX_DESC_INIT: TxRingEntry = TxRingEntry::new();

    // TODO - move the network storage things into a type to clean this up: struct NetworkStorage
    #[init(local = [
        rx_ring: [RxRingEntry; RX_DESC_RING_SIZE] = [RX_DESC_INIT; RX_DESC_RING_SIZE],
        tx_ring: [TxRingEntry; TX_DESC_RING_SIZE] = [TX_DESC_INIT; TX_DESC_RING_SIZE],
        neighbor_storage: [Option<(IpAddress, Neighbor)>; NEIGHBOR_CACHE_SIZE] = [None; NEIGHBOR_CACHE_SIZE],
        ip_addrs: [IpCidr; 1] = [IpCidr::Ipv4(SRC_IP_CIDR); 1],
        routes_storage: [Option<(IpCidr, Route)>; ROUTING_TABLE_SIZE] = [None; ROUTING_TABLE_SIZE],
        sockets: [SocketStorage<'static>; 1] = [SocketStorage::EMPTY; 1],
        eth_dma: core::mem::MaybeUninit<EthernetDMA<'static, 'static>> = core::mem::MaybeUninit::uninit(),
        socket_rx_buffer: [u8; SOCKET_BUFFER_SIZE] = [0; SOCKET_BUFFER_SIZE],
        socket_rx_metadata: [UdpPacketMetadata; 1] = [UdpPacketMetadata::EMPTY; 1],
        socket_tx_buffer: [u8; SOCKET_BUFFER_SIZE] = [0; SOCKET_BUFFER_SIZE],
        socket_tx_metadata: [UdpPacketMetadata; 1] = [UdpPacketMetadata::EMPTY; 1],
    ])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        //rtt_init_print!();
        /*
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Trace))
            .unwrap();
        */
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
        let log_tx_pin = gpiod.pd8.into_alternate();
        let log_tx = ctx
            .device
            .USART3
            .tx(log_tx_pin, 115_200.bps(), &clocks)
            .unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

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
            &mut ctx.local.rx_ring[..],
            &mut ctx.local.tx_ring[..],
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
        let neighbor_cache = NeighborCache::new(&mut ctx.local.neighbor_storage[..]);
        let routes = Routes::new(&mut ctx.local.routes_storage[..]);
        let mut eth_iface = InterfaceBuilder::new(eth_dma, &mut ctx.local.sockets[..])
            .hardware_addr(mac.into())
            .ip_addrs(&mut ctx.local.ip_addrs[..])
            .neighbor_cache(neighbor_cache)
            .routes(routes)
            .finalize();

        let udp_rx_buf = UdpSocketBuffer::new(
            &mut ctx.local.socket_rx_metadata[..],
            &mut ctx.local.socket_rx_buffer[..],
        );
        let udp_tx_buf = UdpSocketBuffer::new(
            &mut ctx.local.socket_tx_metadata[..],
            &mut ctx.local.socket_tx_buffer[..],
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
        net_poll_timer.start(20.Hz()).unwrap();
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

    #[task(binds=TIM4, local = [link_led, phy, net_link_check_timer], priority = 3)]
    fn on_net_link_check_timer(ctx: on_net_link_check_timer::Context) {
        let link_led = ctx.local.link_led;
        let phy = ctx.local.phy;
        let timer = ctx.local.net_link_check_timer;

        let _ = timer.wait();

        // Poll link status
        if phy.phy_link_up() {
            link_led.set_high();
        } else {
            link_led.set_low();
        }
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
