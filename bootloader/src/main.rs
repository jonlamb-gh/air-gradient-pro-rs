// TODO lints

#![no_std]
#![no_main]

mod config;
mod logger;
mod panic_handler;

use bootloader_lib::{BootConfig, ResetReason, UpdateConfigAndStatus, DEFAULT_CONFIG};
use core::sync::atomic::{compiler_fence, Ordering::SeqCst};
use cortex_m_rt::entry;
use log::{debug, error, info, warn};
use stm32f4xx_hal::rcc::Enable;
use stm32f4xx_hal::{
    crc32::Crc32,
    gpio::{Output, PushPull, PC13},
    pac,
    prelude::*,
    watchdog::IndependentWatchdog,
};

type LedPin = PC13<Output<PushPull>>;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let reset_reason = ResetReason::read(&dp.RCC);

    // NOTE: keep this consistent with the application config
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

    let mut watchdog = IndependentWatchdog::new(dp.IWDG);
    watchdog.start(config::WATCHDOG_RESET_PERIOD_MS.millis());
    watchdog.feed();

    let gpioa = dp.GPIOA.split();
    let gpioc = dp.GPIOC.split();

    // Turn it off, active-low
    let _led: LedPin = gpioc.pc13.into_push_pull_output_in_state(true.into());

    // Setup logging impl via USART6, Rx on PA12, Tx on PA11
    // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
    let log_tx_pin = gpioa.pa11.into_alternate();
    let log_tx = dp.USART6.tx(log_tx_pin, 115_200.bps(), &clocks).unwrap();
    unsafe { crate::logger::init_logging(log_tx) };

    let mut flash = dp.FLASH;
    let mut crc = Crc32::new(dp.CRC);
    let mut boot_cfg = match BootConfig::read(&flash, &mut crc) {
        Some(cfg) => {
            debug!("Valid boot config");
            cfg
        }
        None => {
            debug!("Invalid boot config, using default");

            // TODO clear the UCS RAM words too?
            UpdateConfigAndStatus::clear();

            debug!("Writing config to flash");
            let mut cfg = DEFAULT_CONFIG;
            cfg.write(&mut flash, &mut crc);
            cfg
        }
    };

    debug!("Watchdog: inerval {}", watchdog.interval());

    info!("############################################################");
    info!(
        "{} {} ({})",
        crate::built_info::PKG_NAME,
        crate::built_info::PKG_VERSION,
        crate::built_info::PROFILE
    );
    info!("Build date: {}", crate::built_info::BUILT_TIME_UTC);
    info!("{}", crate::built_info::RUSTC_VERSION);
    if let Some(gc) = crate::built_info::GIT_COMMIT_HASH {
        info!("git commit: {}", gc);
    }
    info!("Reset reason: {reset_reason}");
    info!("Boot config slot: {}", boot_cfg.firmware_boot_slot());
    info!("############################################################");

    let update_pending = UpdateConfigAndStatus::update_pending();
    let update_valid = UpdateConfigAndStatus::update_valid();

    const NOT_PENDING: bool = false;
    const IS_PENDING: bool = true;
    const NOT_VALID: bool = false;
    const IS_VALID: bool = true;
    let boot_slot = match (update_pending, update_valid, reset_reason) {
        (IS_PENDING, IS_VALID, ResetReason::SoftwareReset) => {
            // The newly booted updated application marked the update
            // as valid
            debug!("Pending update now complete");
            UpdateConfigAndStatus::clear();
            boot_cfg.swap_firmware_boot_slot();
            debug!("Writing new config slot: {}", boot_cfg.firmware_boot_slot());
            boot_cfg.write(&mut flash, &mut crc);
            boot_cfg.firmware_boot_slot()
        }
        (IS_PENDING, IS_VALID, _) => {
            warn!("The application marked the pending update as valid, but wrong reset reason, aborting");
            UpdateConfigAndStatus::clear();
            boot_cfg.firmware_boot_slot()
        }
        (IS_PENDING, NOT_VALID, ResetReason::SoftwareReset) => {
            // TODO - do the application_flash_address() checks first
            debug!("The application has a pending update, selecting it for boot");
            let current_slot = boot_cfg.firmware_boot_slot();
            current_slot.other()
        }
        (IS_PENDING, NOT_VALID, _) => {
            warn!("The application has a pending update, but wrong reset reason, aborting");
            UpdateConfigAndStatus::clear();
            boot_cfg.firmware_boot_slot()
        }
        (NOT_PENDING, IS_VALID, _) => {
            warn!("UCS.update_valid is true but UCS.update_pending is not, aborting and pending update");
            UpdateConfigAndStatus::clear();
            boot_cfg.firmware_boot_slot()
        }
        (NOT_PENDING, NOT_VALID, _) => {
            debug!("Normal boot");
            UpdateConfigAndStatus::clear();
            boot_cfg.firmware_boot_slot()
        }
    };

    if let Some(valid_app_address) = boot_slot.application_flash_address() {
        debug!("Booting firmware at slot {boot_slot} address 0x{valid_app_address:X}");

        watchdog.feed();

        // de-init
        // SAFETY: don't use logger/peripherals beyond this point
        unsafe {
            logger::flush_logger();

            let rcc = &(*pac::RCC::ptr());
            let _ = crc;
            pac::CRC::disable(rcc);
            pac::USART6::disable(rcc);
        }

        compiler_fence(SeqCst);
        unsafe {
            cp.SCB.vtor.write(valid_app_address);
            cortex_m::asm::bootload(valid_app_address as *const u32);
        }
    } else {
        // TODO do something else? just boot loop...
        error!("The application at boot slot {boot_slot} is invalid!");
        loop {
            cortex_m::asm::nop();
        }
    }
}
