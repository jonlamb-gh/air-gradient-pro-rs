use clap::Parser;
use std::path::PathBuf;
use wire_protocols::broadcast;

/// Command line tool for interacting with the air-gradient-pro firmware
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about, disable_help_subcommand(true))]
pub struct Opts {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug, Clone)]
pub enum Command {
    /// Listen for broadcast messages
    Listen(Listen),

    /// Relay the broadcast messages to InfluxDB
    InfluxRelay(InfluxRelay),

    /// Subcommands for interacting with a device over the network
    #[command(subcommand)]
    Device(Device),
}

#[derive(Parser, Debug, Clone)]
pub struct Listen {
    /// Address
    #[arg(long, short = 'a', default_value = "0.0.0.0")]
    pub address: String,

    /// UDP port number
    #[arg(long, short = 'p', default_value = broadcast::DEFAULT_PORT.to_string())]
    pub port: u16,
}

#[derive(Parser, Debug, Clone)]
pub struct InfluxRelay {
    /// Address
    #[arg(long, short = 'a', default_value = "0.0.0.0")]
    pub address: String,

    /// UDP port number
    #[arg(long, short = 'p', default_value = broadcast::DEFAULT_PORT.to_string())]
    pub port: u16,

    /// InfluxDB host
    #[arg(long, default_value = "http://localhost:8086", env = "INFLUX_HOST")]
    pub host: String,

    /// InfluxDB organization
    #[arg(long, short = 'g', env = "INFLUX_ORG")]
    pub org: String,

    /// InfluxDB bucket
    #[arg(long, short = 'b', env = "INFLUX_BUCKET_NAME")]
    pub bucket: String,

    /// InfluxDB auth token
    #[arg(long, short = 't', env = "INFLUX_TOKEN")]
    pub token: String,

    /// InfluxDB measurement name
    #[arg(long, short = 'm', default_value = "measurement")]
    pub measurement_name: String,
}

#[derive(Parser, Debug, Clone)]
pub enum Device {
    /// Request and print device info
    Info,

    /// Perform a firmware update
    Update(DeviceUpdate),
}

#[derive(Parser, Debug, Clone)]
pub struct DeviceUpdate {
    /// Use the provided directory to store cached image files instead of
    /// a temporary directory
    #[arg(long = "cache")]
    pub cache_dir: Option<PathBuf>,

    /// Path to the 'agp_images.cpio' archive file
    pub agp_images_cpio_file: PathBuf,
}

// TODO
//#[derive(Parser, Debug, Clone)]
//pub struct CommonDeviceOpts {
