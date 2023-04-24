#![deny(warnings, clippy::all)]

use smoltcp::wire::EthernetAddress;
use std::{env, fs, io::Write, net::Ipv4Addr, path::PathBuf};
use wire_protocols::{broadcast, device, DeviceId};

const DEFAULT_IP_ADDRESS: &str = "192.168.1.38";
const DEFAULT_MAC_ADDRESS: &str = "02:00:04:03:07:02";
const DEFAULT_DEVICE_ID: u16 = DeviceId::DEFAULT.0;
const DEFAULT_BROADCAST_PORT: u16 = broadcast::DEFAULT_PORT;
const DEFAULT_BROADCAST_ADDRESS: &str = "255.255.255.255";
const DEFAULT_DEVICE_PORT: u16 = device::DEFAULT_PORT;

pub fn generate_env_config_constants() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let config_file_path = out_dir.join("env_config.rs");
    let mut config_file = fs::File::create(config_file_path).unwrap();

    writeln!(
        &mut config_file,
        "use wire_protocols::{{DeviceId, FirmwareVersion}};"
    )
    .unwrap();

    let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    let minor = env::var("CARGO_PKG_VERSION_MINOR").unwrap();
    let patch = env::var("CARGO_PKG_VERSION_PATCH").unwrap();
    writeln!(&mut config_file, "pub const FIRMWARE_VERSION: FirmwareVersion = FirmwareVersion::new({major}, {minor}, {patch});").unwrap();
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION_MAJOR");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION_MINOR");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION_PATCH");

    let ip_addr: Ipv4Addr = get_env_or_default("AIR_GRADIENT_IP_ADDRESS", DEFAULT_IP_ADDRESS)
        .parse()
        .unwrap();
    let ip_octets = ip_addr.octets();
    writeln!(
        &mut config_file,
        "pub const IP_ADDRESS: [u8; 4] = [{}, {}, {}, {}];",
        ip_octets[0], ip_octets[1], ip_octets[2], ip_octets[3]
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_IP_ADDRESS");

    let mac_addr: EthernetAddress =
        get_env_or_default("AIR_GRADIENT_MAC_ADDRESS", DEFAULT_MAC_ADDRESS)
            .parse()
            .unwrap();
    let mac_octets = mac_addr.as_bytes();
    writeln!(
        &mut config_file,
        "pub const MAC_ADDRESS: [u8; 6] = [0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}];",
        mac_octets[0], mac_octets[1], mac_octets[2], mac_octets[3], mac_octets[4], mac_octets[5]
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_MAC_ADDRESS");

    let device_id: u16 =
        get_env_or_default("AIR_GRADIENT_DEVICE_ID", DEFAULT_DEVICE_ID.to_string())
            .parse()
            .unwrap();
    writeln!(
        &mut config_file,
        "pub const DEVICE_ID: DeviceId = DeviceId::new({device_id});"
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_DEVICE_ID");

    let bcast_port: u16 = get_env_or_default(
        "AIR_GRADIENT_BROADCAST_PORT",
        DEFAULT_BROADCAST_PORT.to_string(),
    )
    .parse()
    .unwrap();
    writeln!(
        &mut config_file,
        "pub const BROADCAST_PORT: u16 = {bcast_port};"
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_BROADCAST_PORT");

    let ip_addr: Ipv4Addr =
        get_env_or_default("AIR_GRADIENT_BROADCAST_ADDRESS", DEFAULT_BROADCAST_ADDRESS)
            .parse()
            .unwrap();
    let ip_octets = ip_addr.octets();
    writeln!(
        &mut config_file,
        "pub const BROADCAST_ADDRESS: [u8; 4] = [{}, {}, {}, {}];",
        ip_octets[0], ip_octets[1], ip_octets[2], ip_octets[3]
    )
    .unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_BROADCAST_ADDRESS");

    let dev_port: u16 =
        get_env_or_default("AIR_GRADIENT_DEVICE_PORT", DEFAULT_DEVICE_PORT.to_string())
            .parse()
            .unwrap();
    writeln!(&mut config_file, "pub const DEVICE_PORT: u16 = {dev_port};").unwrap();
    println!("cargo:rerun-if-env-changed=AIR_GRADIENT_DEVICE_PORT");
}

fn get_env_or_default<S: AsRef<str>>(var: &str, default: S) -> String {
    match env::var(var) {
        Ok(val) => val,
        Err(e) => match e {
            env::VarError::NotPresent => {
                let default = default.as_ref();
                println!("cargo:warning=Using default environment config '{var}={default}'");
                default.to_owned()
            }
            env::VarError::NotUnicode(c) => panic!("Bad env var '{var}'. Got '{c:?}'"),
        },
    }
}
