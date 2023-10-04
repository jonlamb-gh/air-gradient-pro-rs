pub mod eth;
pub mod storage;

pub use eth::{Eth, SpiPins};
pub use storage::{EthernetStorage, NetworkStorage, TcpSocketStorage, UdpSocketStorage};
