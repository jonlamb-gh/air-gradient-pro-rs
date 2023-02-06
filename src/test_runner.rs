use core::fmt::Write;
use stm32f4xx_hal::prelude::*;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        let w = unsafe { crate::logger::get_logger() };
        write!(w, "{}...\t", core::any::type_name::<T>()).unwrap();
        self();
        writeln!(w, "[ok]").unwrap();
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

    let w = unsafe { crate::logger::get_logger() };
    writeln!(w, "Running {} tests", tests.len()).unwrap();
    for test in tests {
        test.run();
    }
}
