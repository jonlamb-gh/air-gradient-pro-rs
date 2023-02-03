//! seperate file from main.rs due to rtic macro issues with cfg_attr

//#![deny(warnings, clippy::all)]
// TODO
//#![forbid(unsafe_code)]
#![no_main]
#![no_std]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_framework::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

// TODO - make a lib?
mod logger;
mod net;
mod panic_handler;
mod rtc;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

// TODO this appears to mess with rtic macros...
// TODO - shared device setup methods to reuse here
#[cfg(test)]
mod test_framework {
    use stm32f4xx_hal::prelude::*;

    pub trait Testable {
        fn run(&self) -> ();
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn run(&self) {
            log::info!("{}...\t", core::any::type_name::<T>());
            self();
            log::info!("[ok]");
        }
    }

    #[no_mangle]
    pub extern "C" fn main() -> ! {
        crate::test_main();
        loop {}
    }

    // TODO - maybe use another serial port instead of the one for Log...
    // need to update panic handler too or don't include it here
    // could impl fmt::Write on the global logger instance too
    pub(crate) fn test_runner(tests: &[&dyn Testable]) {
        let dp = stm32f4xx_hal::pac::Peripherals::take().unwrap();

        // Set up the system clock
        // HCLK must be at least 25MHz to use the ethernet peripheral
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.hclk(64.MHz()).sysclk(180.MHz()).freeze();

        let gpiod = dp.GPIOD.split();

        let log_tx_pin = gpiod.pd8.into_alternate();
        let log_tx = dp.USART3.tx(log_tx_pin, 115_200.bps(), &clocks).unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

        log::info!("Running {} tests", tests.len());
        for test in tests {
            test.run();
        }
    }
}
