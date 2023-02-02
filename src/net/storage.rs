use smoltcp::{
    iface::{Neighbor, Route, SocketStorage},
    socket::UdpPacketMetadata,
    wire::{IpAddress, IpCidr, Ipv4Cidr},
};
use stm32_eth::dma::{RxRingEntry, TxRingEntry};

const RX_RING_INIT: RxRingEntry = RxRingEntry::new();
const TX_RING_INIT: TxRingEntry = TxRingEntry::new();

pub struct EthernetDmaStorage<const RX: usize, const TX: usize> {
    pub rx_ring: [RxRingEntry; RX],
    pub tx_ring: [TxRingEntry; TX],
}

impl<const RX: usize, const TX: usize> EthernetDmaStorage<RX, TX> {
    pub const fn new() -> Self {
        EthernetDmaStorage {
            rx_ring: [RX_RING_INIT; RX],
            tx_ring: [TX_RING_INIT; TX],
        }
    }
}

pub struct NetworkStorage<const NCL: usize, const RTL: usize, const SL: usize> {
    pub neighbor_storage: [Option<(IpAddress, Neighbor)>; NCL],
    pub routes_storage: [Option<(IpCidr, Route)>; RTL],
    pub sockets: [SocketStorage<'static>; SL],
    pub ip_addrs: [IpCidr; 1],
}

impl<const NCL: usize, const RTL: usize, const SL: usize> NetworkStorage<NCL, RTL, SL> {
    pub const fn new(ip_addr: Ipv4Cidr) -> Self {
        NetworkStorage {
            neighbor_storage: [None; NCL],
            routes_storage: [None; RTL],
            sockets: [SocketStorage::EMPTY; SL],
            ip_addrs: [IpCidr::Ipv4(ip_addr)],
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
