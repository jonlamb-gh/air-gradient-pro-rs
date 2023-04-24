use bootloader_support::ResetReason;
use stm32f4xx_hal::pac::RCC;

pub trait ResetReasonExt {
    fn read(rcc: &RCC) -> Self;
    fn read_and_clear(rcc: &mut RCC) -> Self;
}

impl ResetReasonExt for ResetReason {
    fn read(rcc: &RCC) -> Self {
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

    fn read_and_clear(rcc: &mut RCC) -> Self {
        let reason = Self::read(rcc);
        rcc.csr.modify(|_, w| w.rmvf().set_bit());
        reason
    }
}
