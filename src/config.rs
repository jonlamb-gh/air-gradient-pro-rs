use smoltcp::wire::{Ipv4Address, Ipv4Cidr};

// TODO - use env vars + gen build-time for these configs
// or put them in a flash section for configs
// use renode script to setup flash config as needed
pub const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x05, 0x06, 0x07, 0x08];
//const SRC_IP: [u8; 4] = [192, 168, 1, 39];
// TODO - for renode stuff: 192.0.2.29 02:00:05:06:07:08
pub const SRC_IP: [u8; 4] = [192, 0, 2, 29];
pub const SRC_IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(SRC_IP), 24);

pub const UDP_PORT: u16 = 12345;

// TODO - maybe put behind a mod like net
pub const SOCKET_BUFFER_SIZE: usize = 256;
pub const NEIGHBOR_CACHE_LEN: usize = 16;
pub const ROUTING_TABLE_LEN: usize = 16;
pub const RX_RING_LEN: usize = 16;
pub const TX_RING_LEN: usize = 8;
