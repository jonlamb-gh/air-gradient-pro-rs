pub mod data_manager;
pub mod display;
pub mod net;
pub mod pms5003;
pub mod s8lp;
pub mod sgp41;
pub mod sht31;
pub mod watchdog;

pub(crate) use self::data_manager::data_manager_task;
pub(crate) use self::display::display_task;
pub(crate) use self::net::{
    eth_gpio_interrupt_handler_task, ipstack_clock_timer_task, ipstack_poll_task,
    ipstack_poll_timer_task,
};
pub(crate) use self::pms5003::pms5003_task;
pub(crate) use self::s8lp::s8lp_task;
pub(crate) use self::sgp41::sgp41_task;
pub(crate) use self::sht31::sht31_task;
pub(crate) use self::watchdog::watchdog_task;
