use enc28j60::Enc28j60;
use stm32f4xx_hal::{
    gpio::{Alternate, Input, Output, PushPull, AF0, PA8, PA9, PB12, PB13, PB14, PB15},
    pac::SPI2,
    spi::Spi,
};

type CsPin = PB12<Output<PushPull>>;
type IntPin = PA8<Input>;
type ResetPin = PA9<Output<PushPull>>;

type SpiSckPin = PB13<AF0>;
type SpiMisoPin = PB14<AF0>;
type SpiMosiPin = PB15<AF0>;
type SpiPins = (SpiSckPin, SpiMisoPin, SpiMosiPin);
type EthSpi = Spi<SPI2, SpiPins>;

type Drv = Enc28j60<SPI2, CsPin, IntPin, ResetPin>;

/// An ENC28J60 connected to SPI2
pub struct Eth {
    drv: Drv,
    // TODO - probably some error counters, tx and rx, ticked at the smoltcp_phy level
    //rx_buffer: [u8; Eth::MTU],
    //tx_buffer: [u8; Eth::MTU],
}

impl Eth {
    pub const MTU: usize = 1536;

    // TODO - constructor that takes static refs to bufs so they're in bss
    // probably do enc28j60 constructor within as well
    pub fn new(drv: Drv) -> Self {
        Eth { drv }
    }
}
