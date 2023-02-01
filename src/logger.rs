// TODO - this is bad, should probably dedicate a task to this, put a bbq producer in here
// instead

// TODO - feature flag serial vs rtt

use core::fmt::Write as FmtWrite;
use core::{cell::RefCell, mem::MaybeUninit};
use cortex_m::interrupt::{self, Mutex};
use log::{Metadata, Record};
use stm32f4xx_hal::{
    hal::blocking::serial::Write,
    pac::USART3,
    serial::{Instance, Tx},
};

type Inner<T> = Mutex<RefCell<Tx<T>>>;
pub struct Logger<T: Instance>(Inner<T>);

static mut LOGGER: MaybeUninit<Logger<USART3>> = MaybeUninit::uninit();

pub(crate) unsafe fn init_logging(tx: Tx<USART3>) {
    LOGGER.write(Logger(Mutex::new(RefCell::new(tx))));
    log::set_logger(&*LOGGER.as_ptr())
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
}

// TODO
//impl<T: Instance + Sync + Send> log::Log for Logger<T> {
impl log::Log for Logger<USART3> {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            interrupt::free(|cs| {
                writeln!(
                    self.0.borrow(cs).borrow_mut(),
                    "[{}] {}",
                    record.level(),
                    record.args()
                )
                .ok();
            });
        }
    }

    fn flush(&self) {
        interrupt::free(|cs| {
            self.0.borrow(cs).borrow_mut().bflush().ok();
        });
    }
}
