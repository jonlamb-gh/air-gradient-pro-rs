use smoltcp::wire::{Ipv4Address, Ipv4Cidr};

// TODO - use env vars + gen build-time for these configs
// or put them in a flash section for configs
pub const SRC_MAC: [u8; 6] = [0x02, 0x00, 0x04, 0x03, 0x07, 0x02];
pub const SRC_IP: [u8; 4] = [192, 168, 1, 38];
pub const SRC_IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(SRC_IP), 24);

// TODO - maybe put behind a mod like net
pub const SOCKET_BUFFER_LEN: usize = 256;

pub const STARTUP_DELAY_SECONDS: u8 = 5;

pub const SGP41_MEASUREMENT_INTERVAL_MS: u32 = 1000;
pub const SHT31_MEASUREMENT_INTERVAL_MS: u32 = 2500;
pub const S8LP_MEASUREMENT_INTERVAL_MS: u32 = 5000;

/// PMS sensor is woken up for measurements every 5 minutes
/// to conserve lifespan, it also needs to warm up for at
/// least 30 seconds before taking a measurement.
///
/// The measurement task is run once a second to drive the
/// wake/measurement/sleep cycle.
pub const PMS5003_MEASUREMENT_INTERVAL_MS: u32 = 1000;
pub const PMS5003_WAKE_INTERVAL_MS: u32 = (5 * 60) * 1000;
pub const PMS5003_WARM_UP_PERIOD_MS: u32 = 45 * 1000;

/// Number of measurements to perform (one per measurement interval) before
/// going into standby mode.
pub const PMS5003_MEASUREMENT_COUNT: u8 = 10;
