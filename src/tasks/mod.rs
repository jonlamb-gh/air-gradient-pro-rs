//pub mod data_manager;
pub mod net;
//pub mod sgp41;
//pub mod sht31;

//pub(crate) use self::data_manager::{data_manager_task, SpawnArg};
pub(crate) use self::net::{
    eth_gpio_interrupt_handler_task, ipstack_clock_timer_task, ipstack_poll_task,
    ipstack_poll_timer_task,
};
//pub(crate) use self::sgp41::sgp41_task;
//pub(crate) use self::sht31::sht31_task;
