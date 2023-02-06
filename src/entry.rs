//#![deny(warnings, clippy::all)]
// TODO
//#![forbid(unsafe_code)]
#![no_main]
#![no_std]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

mod logger;
mod net;
mod panic_handler;
mod rtc;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(test)]
mod test_runner;

#[cfg(not(test))]
mod firmware_main;
