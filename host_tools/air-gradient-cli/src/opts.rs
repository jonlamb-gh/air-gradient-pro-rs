use clap::Parser;
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
    #[arg(long, default_value = "http://localhost:8086", env = "INFLUXDB_HOST")]
    pub host: String,

    /// InfluxDB organization
    #[arg(long, short = 'g', env = "INFLUXDB_ORG")]
    pub org: String,

    /// InfluxDB bucket
    #[arg(long, short = 'b', env = "INFLUXDB_BUCKET")]
    pub bucket: String,

    /// InfluxDB auth token
    #[arg(long, short = 't', env = "INFLUXDB_TOKEN")]
    pub token: String,
}
