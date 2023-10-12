use enc28j60::Enc28j60;
use log::{debug, error, warn};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use stm32f4xx_hal::{
    gpio::{Input, Output, PushPull, AF5, PA8, PB12, PB13, PB14, PB15},
    pac::SPI2,
    spi::Spi,
};

type CsPin = PB12<Output<PushPull>>;
type IntPin = PA8<Input>;
//type ResetPin = PB1<Output<PushPull>>;
type ResetPin = enc28j60::Unconnected;

pub type SpiSckPin = PB13<AF5>;
pub type SpiMisoPin = PB14<AF5>;
pub type SpiMosiPin = PB15<AF5>;
pub type SpiPins = (SpiSckPin, SpiMisoPin, SpiMosiPin);
pub type EthSpi = Spi<SPI2>;

type Drv = Enc28j60<EthSpi, CsPin, IntPin, ResetPin>;

// TODO - add some rx/tx error counters
/// An ENC28J60 connected to SPI2
pub struct Eth<'buf> {
    drv: Drv,
    rx_buffer: &'buf mut [u8],
    tx_buffer: &'buf mut [u8],
}

impl<'buf> Eth<'buf> {
    pub const MTU: usize = 1514;

    pub fn new(drv: Drv, rx_buffer: &'buf mut [u8], tx_buffer: &'buf mut [u8]) -> Self {
        let eth = Eth {
            drv,
            rx_buffer,
            tx_buffer,
        };
        debug!(
            "ENC28J60: buffer length, rx {}, tx {}, mtu {}",
            eth.rx_buffer.len(),
            eth.tx_buffer.len(),
            eth.mtu(),
        );
        eth
    }

    pub fn driver(&mut self) -> &mut Drv {
        &mut self.drv
    }

    fn mtu(&self) -> usize {
        // TODO - fixup the MTU logic
        // 1514, the maximum frame length allowed by the interface
        // 1024, buffer sizes
        let min_buf = core::cmp::min(self.rx_buffer.len(), self.tx_buffer.len());
        let min_iface = core::cmp::min(self.drv.mtu() as usize, Self::MTU);
        core::cmp::min(min_buf, min_iface)
    }
}

impl<'buf> Device for Eth<'buf> {
    type RxToken<'a> = RxToken<'a> where Self: 'a;
    type TxToken<'a> = TxToken<'a> where Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self.drv.next_packet() {
            Ok(Some(packet)) => {
                if packet.len() as usize > self.rx_buffer.len() {
                    warn!(
                        "Dropping rx packet, too big, len {}, cap {}",
                        packet.len(),
                        self.rx_buffer.len()
                    );
                    packet.ignore().unwrap();
                    None
                } else if let Err(e) = packet.read(&mut self.rx_buffer[..]) {
                    error!("Failed to read next packet. {e:?}");
                    None
                } else {
                    Some((
                        RxToken(&mut self.rx_buffer[..]),
                        TxToken {
                            phy: &mut self.drv,
                            buf: self.tx_buffer,
                        },
                    ))
                }
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to receive next packet. {e:?}");
                None
            }
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(TxToken {
            phy: &mut self.drv,
            buf: self.tx_buffer,
        })
    }

    // TODO - double check CRC behavior, it's done in the hw
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu();
        caps.max_burst_size = Some(1);
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct RxToken<'a>(&'a mut [u8]);

impl<'a> phy::RxToken for RxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(self.0)
    }
}

pub struct TxToken<'a> {
    phy: &'a mut Drv,
    buf: &'a mut [u8],
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let result = f(&mut self.buf[..len]);
        if let Err(e) = self.phy.transmit(&self.buf[..len]) {
            error!("Failed to transmit packet. {e:?}");
        }
        result
    }
}
