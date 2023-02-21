// TODO
// on_net_clock_timer
// on_net_link_check_timer
// on_eth
// on_net_poll_timera
// poll_ip_stack
//
// maybe split some up into net/ and ip/

use crate::firmware_main::app::ipstack_clock_timer_task;
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use smoltcp::time::Instant;

/// 32-bit millisecond clock
#[derive(Debug)]
#[repr(transparent)]
pub struct GlobalMillisClock(AtomicU32);

impl GlobalMillisClock {
    pub const fn new() -> Self {
        GlobalMillisClock(AtomicU32::new(0))
    }

    pub fn inc_from_interrupt(&self) {
        self.0.fetch_add(1, Relaxed);
    }

    pub fn get_raw(&self) -> u32 {
        self.0.load(Relaxed)
    }

    pub fn get(&self) -> Instant {
        Instant::from_millis(self.get_raw() as i64)
    }
}

static NET_CLOCK: GlobalMillisClock = GlobalMillisClock::new();

pub(crate) fn ipstack_clock_timer_task(ctx: ipstack_clock_timer_task::Context) {
    let timer = ctx.local.net_clock_timer;
    let _ = timer.wait();
    NET_CLOCK.inc_from_interrupt();
}
