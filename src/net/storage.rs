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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        NEIGHBOR_CACHE_LEN, ROUTING_TABLE_LEN, RX_RING_LEN, SOCKET_BUFFER_LEN, SRC_IP, TX_RING_LEN,
    };
    use crate::test_runner::TestResources;
    use smoltcp::wire::Ipv4Address;

    #[test_case]
    fn net_storage(_res: TestResources) {
        let _eth_dma_storage: EthernetDmaStorage<RX_RING_LEN, TX_RING_LEN> =
            EthernetDmaStorage::new();
        let _net_storage: NetworkStorage<NEIGHBOR_CACHE_LEN, ROUTING_TABLE_LEN, 1> =
            NetworkStorage::new(Ipv4Cidr::new(Ipv4Address(SRC_IP), 24));
        let _udp_socket_storage: UdpSocketStorage<SOCKET_BUFFER_LEN> = UdpSocketStorage::new();
    }
}
