//! Note most of this was taken from the stm32-eth examples

use ieee802_3_miim::{
    phy::{
        lan87xxa::{LAN8720A, LAN8742A},
        BarePhy, KSZ8081R,
    },
    Miim, Pause, Phy,
};

/// An ethernet PHY
pub enum EthernetPhy<M: Miim> {
    /// LAN8720A
    LAN8720A(LAN8720A<M>),
    /// LAN8742A
    LAN8742A(LAN8742A<M>),
    /// KSZ8081R
    KSZ8081R(KSZ8081R<M>),
}

impl<M: Miim> Phy<M> for EthernetPhy<M> {
    fn best_supported_advertisement(&self) -> ieee802_3_miim::AutoNegotiationAdvertisement {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.best_supported_advertisement(),
            EthernetPhy::LAN8742A(phy) => phy.best_supported_advertisement(),
            EthernetPhy::KSZ8081R(phy) => phy.best_supported_advertisement(),
        }
    }

    fn get_miim(&mut self) -> &mut M {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.get_miim(),
            EthernetPhy::LAN8742A(phy) => phy.get_miim(),
            EthernetPhy::KSZ8081R(phy) => phy.get_miim(),
        }
    }

    fn get_phy_addr(&self) -> u8 {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.get_phy_addr(),
            EthernetPhy::LAN8742A(phy) => phy.get_phy_addr(),
            EthernetPhy::KSZ8081R(phy) => phy.get_phy_addr(),
        }
    }
}

impl<M: Miim> EthernetPhy<M> {
    /// Attempt to create one of the known PHYs from the given
    /// MIIM.
    ///
    /// Returns an error if the PHY does not support the extended register
    /// set, or if the PHY's identifier does not correspond to a known PHY.
    pub fn from_miim(miim: M, phy_addr: u8) -> Result<Self, M> {
        let mut bare = BarePhy::new(miim, phy_addr, Pause::NoPause);
        let phy_ident = if let Some(id) = bare.phy_ident() {
            id.raw_u32()
        } else {
            return Err(bare.release());
        };
        let miim = bare.release();
        match phy_ident & 0xFFFFFFF0 {
            0x0007C0F0 => Ok(Self::LAN8720A(LAN8720A::new(miim, phy_addr))),
            0x0007C130 => Ok(Self::LAN8742A(LAN8742A::new(miim, phy_addr))),
            0x00221560 => Ok(Self::KSZ8081R(KSZ8081R::new(miim, phy_addr))),
            _ => Err(miim),
        }
    }

    /// Get a string describing the type of PHY
    pub const fn ident_string(&self) -> &'static str {
        match self {
            EthernetPhy::LAN8720A(_) => "LAN8720A",
            EthernetPhy::LAN8742A(_) => "LAN8742A",
            EthernetPhy::KSZ8081R(_) => "KSZ8081R",
        }
    }

    /// Initialize the PHY
    pub fn phy_init(&mut self) {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.phy_init(),
            EthernetPhy::LAN8742A(phy) => phy.phy_init(),
            EthernetPhy::KSZ8081R(phy) => {
                phy.set_autonegotiation_advertisement(phy.best_supported_advertisement());
            }
        }
    }

    pub fn speed(&mut self) -> Option<ieee802_3_miim::phy::PhySpeed> {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.link_speed(),
            EthernetPhy::LAN8742A(phy) => phy.link_speed(),
            EthernetPhy::KSZ8081R(phy) => phy.link_speed(),
        }
    }

    #[allow(dead_code)]
    pub fn release(self) -> M {
        match self {
            EthernetPhy::LAN8720A(phy) => phy.release(),
            EthernetPhy::LAN8742A(phy) => phy.release(),
            EthernetPhy::KSZ8081R(phy) => phy.release(),
        }
    }
}