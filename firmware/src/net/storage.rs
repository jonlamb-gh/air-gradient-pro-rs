use smoltcp::{iface::SocketStorage, socket::udp::PacketMetadata as UdpPacketMetadata};

pub struct EthernetStorage<const BL: usize> {
    pub rx_buffer: [u8; BL],
    pub tx_buffer: [u8; BL],
}

impl<const BL: usize> EthernetStorage<BL> {
    pub const fn new() -> Self {
        EthernetStorage {
            rx_buffer: [0; BL],
            tx_buffer: [0; BL],
        }
    }
}

pub struct NetworkStorage<const SL: usize> {
    pub sockets: [SocketStorage<'static>; SL],
}

impl<const SL: usize> NetworkStorage<SL> {
    pub const fn new() -> Self {
        NetworkStorage {
            sockets: [SocketStorage::EMPTY; SL],
        }
    }
}

pub struct UdpSocketStorage<const BL: usize> {
    pub rx_buffer: [u8; BL],
    pub rx_metadata: [UdpPacketMetadata; 1],
    pub tx_buffer: [u8; BL],
    pub tx_metadata: [UdpPacketMetadata; 1],
}

impl<const BL: usize> UdpSocketStorage<BL> {
    pub const fn new() -> Self {
        UdpSocketStorage {
            rx_buffer: [0; BL],
            rx_metadata: [UdpPacketMetadata::EMPTY; 1],
            tx_buffer: [0; BL],
            tx_metadata: [UdpPacketMetadata::EMPTY; 1],
        }
    }
}

pub struct TcpSocketStorage<const BL: usize> {
    pub rx_buffer: [u8; BL],
    pub tx_buffer: [u8; BL],
}

impl<const BL: usize> TcpSocketStorage<BL> {
    pub const fn new() -> Self {
        TcpSocketStorage {
            rx_buffer: [0; BL],
            tx_buffer: [0; BL],
        }
    }
}
