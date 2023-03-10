use cortex_m::Peripherals as CorePeripherals;
use stm32f4xx_hal::{pac::Peripherals, prelude::*, rcc::Clocks};

/// SAFETY:
/// * Don't mess with the clocks, USART3, or PD8
/// * Unit tests should do cleanup/de-init if needed
/// * TODO
///   - write down constraints
///   - add a per-test/global timeout (reserved TIM or systick, SYST is used by some tests atm)
pub struct TestResources {
    pub dp: Peripherals,
    pub cp: CorePeripherals,
    pub clocks: Clocks,
}

pub trait Testable {
    fn run(&self, res: TestResources) -> ();
}

impl<T> Testable for T
where
    T: Fn(TestResources),
{
    fn run(&self, res: TestResources) {
        let w = unsafe { crate::logger::get_logger() };
        write!(w, "{}...\t", core::any::type_name::<T>()).unwrap();
        self(res);
        writeln!(w, "[ok]").unwrap();
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    crate::test_main();
    loop {}
}

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

    writeln!(
        w,
        "############################################################"
    )
    .unwrap();
    writeln!(
        w,
        "{} {} ({})",
        crate::built_info::PKG_NAME,
        crate::built_info::PKG_VERSION,
        crate::built_info::PROFILE
    )
    .unwrap();
    writeln!(w, "Build date: {}", crate::built_info::BUILT_TIME_UTC).unwrap();
    writeln!(w, "{}", crate::built_info::RUSTC_VERSION).unwrap();
    if let Some(gc) = crate::built_info::GIT_COMMIT_HASH {
        writeln!(w, "git commit: {}", gc).unwrap();
    }
    writeln!(
        w,
        "############################################################"
    )
    .unwrap();

    writeln!(w, "running {} tests", tests.len()).unwrap();
    for test in tests {
        let res = unsafe {
            TestResources {
                dp: Peripherals::steal(),
                cp: CorePeripherals::steal(),
                clocks: clocks.clone(),
            }
        };
        test.run(res);
    }
    writeln!(w, "test result: ok. {} passed; 0 failed", tests.len()).unwrap();
}
