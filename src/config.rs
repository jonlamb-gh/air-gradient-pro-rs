use smoltcp::wire::{Ipv4Address, Ipv4Cidr};

// TODO - use env vars + gen build-time for these configs
// or put them in a flash section for configs
pub const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x04, 0x03, 0x07, 0x02];
pub const SRC_IP: [u8; 4] = [192, 168, 1, 38];
pub const SRC_IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(SRC_IP), 24);

// TODO - maybe put behind a mod like net
pub const SOCKET_BUFFER_LEN: usize = 256;

// sensor mod
pub const MEASUREMENT_PERIOD_MS: u32 = 1000;
