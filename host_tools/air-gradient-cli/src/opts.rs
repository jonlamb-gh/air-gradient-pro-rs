use clap::Parser;
use std::{fmt, path::PathBuf, str::FromStr};
use wire_protocols::{broadcast as broadcast_proto, device as device_proto};

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

    /// Extract firmware ELF files from an archive file
    ExtractArchive(ExtractArchive),
}

#[derive(Parser, Debug, Clone)]
pub struct Listen {
    /// Address
    #[arg(long, short = 'a', default_value = "0.0.0.0")]
    pub address: String,

    /// UDP port number
    #[arg(long, short = 'p', default_value = broadcast_proto::DEFAULT_PORT.to_string())]
    pub port: u16,
}

#[derive(Parser, Debug, Clone)]
pub struct InfluxRelay {
    /// Address
    #[arg(long, short = 'a', default_value = "0.0.0.0")]
    pub address: String,

    /// UDP port number
    #[arg(long, short = 'p', default_value_t = broadcast_proto::DEFAULT_PORT)]
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
    Info(CommonDeviceOpts),

    /// Reboot a device
    Reboot(CommonDeviceOpts),

    /// Perform a firmware update
    Update(DeviceUpdate),
}

#[derive(Parser, Debug, Clone)]
pub struct DeviceUpdate {
    #[clap(flatten)]
    pub common: CommonDeviceOpts,

    /// Use the provided directory to store cached image files instead of
    /// a temporary directory
    #[arg(long = "cache")]
    pub cache_dir: Option<PathBuf>,

    /// Path to the 'agp_images.cpio' archive file
    pub agp_images_cpio_file: PathBuf,
}

#[derive(Parser, Debug, Clone)]
pub struct CommonDeviceOpts {
    /// Address
    #[arg(long, short = 'a')]
    pub address: String,

    /// Device protocol TCP port number
    #[arg(long, short = 'p', default_value_t = device_proto::DEFAULT_PORT)]
    pub port: u16,

    /// Output format
    #[arg(long, short = 'f', default_value_t = Format::Text)]
    pub format: Format,
}

#[derive(Parser, Debug, Clone)]
pub struct ExtractArchive {
    /// Output directory to extract to
    #[arg(long = "output", short = 'o', default_value = ".")]
    pub output_dir: PathBuf,

    /// Path to the 'agp_images.cpio' archive file
    pub agp_images_cpio_file: PathBuf,
}

#[derive(Parser, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Format {
    #[default]
    Text,
    Json,
}

impl Format {
    pub fn is_text(&self) -> bool {
        matches!(self, Format::Text)
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "text" => Format::Text,
            "json" => Format::Json,
            _ => return Err("Invalid format '{s}'".to_owned()),
        })
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::Text => f.write_str("text"),
            Format::Json => f.write_str("json"),
        }
    }
}
