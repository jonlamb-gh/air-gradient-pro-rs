pub mod phy;
pub mod storage;

pub use phy::EthernetPhy;
pub use storage::{EthernetDmaStorage, NetworkStorage, UdpSocketStorage};
