use core::ptr;

/// The UCS (Update Configuration and Status) in-memory state.
pub struct UpdateConfigAndStatus {}

impl UpdateConfigAndStatus {
    /// Address in RAM where the UCS lives.
    /// word0 == updating_pending
    /// word1 = update_valid
    const RAM_ADDRESS: u32 = 0x2000_0000;
    const MAGIC_TRUE: u32 = 0xACAD_B0FC;

    /// Clears teh UCS.update_pending and UCS.update_valid flags.
    pub fn clear() {
        cortex_m::interrupt::free(|_cs| unsafe {
            ptr::write_volatile(Self::base_ptr_mut(), 0);
            ptr::write_volatile(Self::base_ptr_mut().offset(1), 0);
        });
    }

    /// Retrieves and clears the UCS.updating_pending flag.
    pub fn update_pending() -> bool {
        cortex_m::interrupt::free(|_cs| unsafe {
            let word = ptr::read_volatile(Self::base_ptr());
            ptr::write_volatile(Self::base_ptr_mut(), 0);
            word == Self::MAGIC_TRUE
        })
    }

    /// Sets the UCS.updating_pending flag.
    pub fn set_update_pending() {
        cortex_m::interrupt::free(|_cs| unsafe {
            ptr::write_volatile(Self::base_ptr_mut(), Self::MAGIC_TRUE);
        });
    }

    /// Retrieves and clears the UCS.update_valid flag.
    pub fn update_valid() -> bool {
        cortex_m::interrupt::free(|_cs| unsafe {
            let word = ptr::read_volatile(Self::base_ptr().offset(1));
            ptr::write_volatile(Self::base_ptr_mut().offset(1), 0);
            word == Self::MAGIC_TRUE
        })
    }

    /// Sets the UCS.update_valid flag.
    pub fn set_update_valid() {
        cortex_m::interrupt::free(|_cs| unsafe {
            ptr::write_volatile(Self::base_ptr_mut().offset(1), Self::MAGIC_TRUE);
        });
    }

    const fn base_ptr() -> *const u32 {
        Self::RAM_ADDRESS as *const _
    }

    const fn base_ptr_mut() -> *mut u32 {
        Self::RAM_ADDRESS as *mut _
    }
}
