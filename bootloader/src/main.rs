// TODO lints

#![no_std]
#![no_main]

mod config;
mod logger;
mod panic_handler;

use bootloader_lib::{BootConfig, ResetReason, DEFAULT_CONFIG};
use core::sync::atomic::{compiler_fence, Ordering::SeqCst};
use cortex_m_rt::entry;
use log::{debug, info};
use stm32f4xx_hal::rcc::Enable;
use stm32f4xx_hal::{
    crc32::Crc32,
    flash::{flash_sectors, FlashExt},
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
    let mut dp = pac::Peripherals::take().unwrap();

    let reset_reason = ResetReason::read_and_clear(&mut dp.RCC);

    // NOTE: keep this consistent with the application config
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

    // TODO add watchdog back in
    //    let mut watchdog = IndependentWatchdog::new(dp.IWDG);
    //    watchdog.start(config::WATCHDOG_RESET_PERIOD_MS.millis());
    //    watchdog.feed();

    let gpioa = dp.GPIOA.split();
    let gpioc = dp.GPIOC.split();

    // Turn it off, active-low
    let _led: LedPin = gpioc.pc13.into_push_pull_output_in_state(true.into());

    // Setup logging impl via USART6, Rx on PA12, Tx on PA11
    // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
    let log_tx_pin = gpioa.pa11.into_alternate();
    let log_tx = dp.USART6.tx(log_tx_pin, 115_200.bps(), &clocks).unwrap();
    unsafe { crate::logger::init_logging(log_tx) };

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
    info!("############################################################");

    //debug!("Watchdog: inerval {}", watchdog.interval());

    let mut flash = dp.FLASH;
    debug!("Flash addr: 0x{:X}", flash.address());
    debug!("Flash size: {} ({}K)", flash.len(), flash.len() / 1024);
    debug!("Flash dual-bank: {}", flash.dual_bank());
    for sector in flash_sectors(flash.len(), flash.dual_bank()) {
        debug!(
            "  sector {} @ 0x{:X}, LEN = {} ({}K)",
            sector.number,
            sector.offset,
            sector.size,
            sector.size / 1024
        );
    }

    let mut crc = Crc32::new(dp.CRC);
    let boot_cfg = match BootConfig::read(&flash, &mut crc) {
        Some(cfg) => {
            debug!("Valid boot config");
            cfg
        }
        None => {
            debug!("Invalid boot config, using default");
            // TODO only if reset_reason is sw??
            // clear the RAM words too?
            debug!("Writing config to flash");

            let mut cfg = DEFAULT_CONFIG;
            cfg.write(&mut flash, &mut crc);
            cfg
        }
    };

    info!("BC.firmware_boot_slot = {}", boot_cfg.firmware_boot_slot());

    if reset_reason == ResetReason::PowerOnReset {
        if let Some(fw_address) = boot_cfg.firmware_boot_slot().application_flash_address() {
            info!("Booting firmware at address 0x{fw_address:X}");

            //watchdog.feed();

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
                cp.SCB.vtor.write(fw_address);
                cortex_m::asm::bootload(fw_address as *const u32);
            }
        }
    }

    // TODO
    loop {
        cortex_m::asm::nop();
        //watchdog.feed();
    }
}
