// TODO
// on_net_clock_timer
// on_net_link_check_timer
// on_eth
// on_net_poll_timera
// poll_ip_stack
//
// maybe split some up into net/ and ip/

use crate::firmware_main::app::{
    eth_interrupt_handler_task, eth_link_status_timer_task, ipstack_clock_timer_task,
    ipstack_poll_task, ipstack_poll_timer_task,
};
use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
use ieee802_3_miim::Phy;
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
    let net = ctx.shared.net;
    let time = NET_CLOCK.get();
    match net.poll(time) {
        Ok(_something_happened) => (),
        Err(e) => debug!("{:?}", e),
    }
}

pub(crate) fn ipstack_poll_timer_task(ctx: ipstack_poll_timer_task::Context) {
    let timer = ctx.local.ipstack_poll_timer;
    let _ = timer.wait();
    ipstack_poll_task::spawn().ok();
}

pub(crate) fn eth_interrupt_handler_task(ctx: eth_interrupt_handler_task::Context) {
    let net = ctx.shared.net;
    net.device_mut().interrupt_handler();
    ipstack_poll_task::spawn().ok();
}

pub(crate) fn eth_link_status_timer_task(ctx: eth_link_status_timer_task::Context) {
    let link_led = ctx.local.link_led;
    let phy = ctx.local.phy;
    let timer = ctx.local.eth_link_status_timer;
    let prev_link_status = ctx.local.prev_link_status;
    let _ = timer.wait();
    let link_status = if phy.phy_link_up() {
        link_led.set_high();
        true
    } else {
        link_led.set_low();
        false
    };

    if link_status != *prev_link_status {
        info!("Link is {}", if link_status { "up" } else { "down" });
        *prev_link_status = link_status;
    }
}
