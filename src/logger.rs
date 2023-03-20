use core::fmt::{self, Write as FmtWrite};
use core::{cell::RefCell, mem::MaybeUninit};
use cortex_m::interrupt::{self, Mutex};
use log::{Metadata, Record};
use stm32f4xx_hal::{
    hal::blocking::serial::Write,
    pac::USART2,
    serial::{Instance, Tx},
};

type Inner<T> = Mutex<RefCell<Tx<T>>>;
pub struct Logger<T: Instance>(Inner<T>);
pub type LoggerUsart2 = MaybeUninit<Logger<USART2>>;

static mut LOGGER: LoggerUsart2 = LoggerUsart2::uninit();

pub(crate) unsafe fn init_logging(tx: Tx<USART2>) {
    LOGGER.write(Logger(Mutex::new(RefCell::new(tx))));
    log::set_logger(&*LOGGER.as_ptr())
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
}

pub(crate) unsafe fn get_logger() -> &'static mut dyn fmt::Write {
    &mut *LOGGER.as_mut_ptr()
}

impl log::Log for Logger<USART2> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            interrupt::free(|cs| {
                writeln!(
                    self.0.borrow(cs).borrow_mut(),
                    "[{}] {}",
                    level_marker(record.level()),
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

impl fmt::Write for Logger<USART2> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        interrupt::free(|cs| {
            self.0.borrow(cs).borrow_mut().bwrite_all(s.as_bytes()).ok();
        });
        Ok(())
    }
}

const fn level_marker(level: log::Level) -> &'static str {
    use log::Level::*;
    match level {
        Error => "E",
        Warn => "W",
        Info => "I",
        Debug => "D",
        Trace => "T",
    }
}
