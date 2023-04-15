use core::fmt;
use stm32f4xx_hal::pac::RCC;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ResetReason {
    /// Low-power management reset
    LowPowerReset,
    /// The window watchdog triggered
    WindowWatchdogReset,
    /// The independent watchdog triggered
    IndependentWatchdogReset,
    /// The software did a soft reset
    SoftwareReset,
    /// The mcu went from not having power to having power and resetting
    PowerOnReset,
    /// The reset pin was asserted
    PinReset,
    /// The brownout detector triggered
    BrownoutReset,
    /// The reason could not be determined, contains the raw CSR register value
    Unknown(u32),
}

impl fmt::Display for ResetReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResetReason::LowPowerReset => f.write_str("Low-power management reset"),
            ResetReason::WindowWatchdogReset => f.write_str("WWDG reset"),
            ResetReason::IndependentWatchdogReset => f.write_str("IWDG reset"),
            ResetReason::SoftwareReset => f.write_str("Software reset"),
            ResetReason::PowerOnReset => f.write_str("Power-on reset"),
            ResetReason::PinReset => f.write_str("Pin reset (NRST)"),
            ResetReason::BrownoutReset => f.write_str("Brownout reset"),
            ResetReason::Unknown(rcc_csr) => write!(
                f,
                "Could not determine the cause. RCC CSR bits were 0x{:X}",
                rcc_csr
            ),
        }
    }
}

impl ResetReason {
    pub fn read(rcc: &RCC) -> Self {
        let reason = rcc.csr.read();
        if reason.lpwrrstf().bit_is_set() {
            ResetReason::LowPowerReset
        } else if reason.wwdgrstf().bit_is_set() {
            ResetReason::WindowWatchdogReset
        } else if reason.wdgrstf().bit_is_set() {
            ResetReason::IndependentWatchdogReset
        } else if reason.sftrstf().bit_is_set() {
            ResetReason::SoftwareReset
        } else if reason.porrstf().bit_is_set() {
            ResetReason::PowerOnReset
        } else if reason.padrstf().bit_is_set() {
            ResetReason::PinReset
        } else if reason.borrstf().bit_is_set() {
            ResetReason::BrownoutReset
        } else {
            ResetReason::Unknown(reason.bits())
        }
    }

    pub fn read_and_clear(rcc: &mut RCC) -> Self {
        let reason = Self::read(rcc);
        rcc.csr.modify(|_, w| w.rmvf().set_bit());
        reason
    }
}
