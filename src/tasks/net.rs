use crate::firmware_main::app::{
    ipstack_clock_timer_task, ipstack_poll_task, ipstack_poll_timer_task,
};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use log::{debug, info};
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

pub(crate) fn ipstack_poll_task(ctx: ipstack_poll_task::Context) {
    let eth = ctx.shared.eth;
    let net = ctx.shared.net;
    let sockets = ctx.shared.sockets;
    let time = NET_CLOCK.get();
    if net.poll(time, eth, sockets) {
        // _something_happened
    }
}

pub(crate) fn ipstack_poll_timer_task(ctx: ipstack_poll_timer_task::Context) {
    let timer = ctx.local.ipstack_poll_timer;
    let _ = timer.wait();
    ipstack_poll_task::spawn().ok();
}
