use crate::{interruptor::Interruptor, measurement::MessageExt, opts::Listen};
use anyhow::Result;
use chrono::prelude::*;
use std::{collections::BTreeMap, net::UdpSocket, time::Duration};
use wire_protocols::{
    broadcast::{Message as WireMessage, Repr as Message, MESSAGE_LEN},
    DeviceId, ProtocolIdentifier,
};

const TIMEOUT: Duration = Duration::from_millis(100);

pub async fn listen(cmd: Listen, intr: Interruptor) -> Result<()> {
    println!(
        "Listening for UDP broadcast messages on {}:{}",
        cmd.address, cmd.port
    );

    let socket = UdpSocket::bind((cmd.address.as_str(), cmd.port))?;
    socket.set_read_timeout(TIMEOUT.into())?;

    let mut buf = vec![0; MESSAGE_LEN * 10];

    let mut stats = BTreeMap::new();

    println!();
    loop {
        if intr.is_set() {
            break;
        }

        let (bytes_recvd, src_addr) = match socket.recv_from(&mut buf) {
            Ok(ret) => ret,
            Err(_e) => continue,
        };
        let recv_utc: DateTime<Utc> = Utc::now();

        println!("Received {bytes_recvd} bytes from {src_addr}");
        println!("UTC: {recv_utc}");

        // TODO - walk entire buffer for possible multiple messages

        let wire_msg = match WireMessage::new_checked(&buf) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to parse as broadcast wire message. {e}");
                continue;
            }
        };

        let msg = match Message::parse(&wire_msg) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to parse as broadcast message. {e}");
                continue;
            }
        };

        let device_stats = stats
            .entry(msg.device_serial_number)
            .or_insert(DeviceStats {
                device_id: msg.device_id,
                last_seqnum: msg.sequence_number,
                total_messages: 0,
                missed_messages: 0,
            });

        if device_stats.total_messages != 0 {
            if msg.sequence_number == device_stats.last_seqnum {
                eprintln!(
                    "** Duplicate message sequence number {}",
                    msg.sequence_number
                );
            }

            if msg.sequence_number != (device_stats.last_seqnum + 1) {
                eprintln!(
                    "** Missed message sequence number {} (current {})",
                    device_stats.last_seqnum + 1,
                    msg.sequence_number
                );
                device_stats.missed_messages += 1;
            }
        }

        println!("Protocol: {}", ProtocolIdentifier::Broadcast);
        println!("Protocol version: {}", msg.protocol_version);
        println!("Firmware version: {}", msg.firmware_version);
        println!("Device ID: 0x{:X} ({})", msg.device_id, msg.device_id);
        println!("Device serial number: {:X}", msg.device_serial_number);
        println!("Sequence number: {}", msg.sequence_number);
        println!("Uptime seconds: {} | {}", msg.uptime_seconds, msg.uptime());
        println!("Status flags: 0x{:X}", msg.status_flags.0);
        println!("  initialized: {}", msg.status_flags.initialized());
        println!("  datetime_valid: {}", msg.status_flags.datetime_valid());
        println!(
            "  temperature_valid: {}",
            msg.status_flags.temperature_valid()
        );
        println!("  humidity_valid: {}", msg.status_flags.humidity_valid());
        println!("  voc_ticks_valid: {}", msg.status_flags.voc_ticks_valid());
        println!("  nox_ticks_valid: {}", msg.status_flags.nox_ticks_valid());
        println!("  voc_index_valid: {}", msg.status_flags.voc_index_valid());
        println!("  nox_index_valid: {}", msg.status_flags.nox_index_valid());
        println!("  pm2_5_valid: {}", msg.status_flags.pm2_5_valid());
        println!("  co2_valid: {}", msg.status_flags.co2_valid());

        if msg.status_flags.datetime_valid() {
            println!("DateTime: {}", msg.datetime);
        }
        if msg.status_flags.temperature_valid() {
            println!(
                "Temperature: {} cC | {:.02} °C | {:.02} °F",
                msg.temperature,
                msg.temperature_c(),
                msg.temperature_f(),
            );
        }
        if msg.status_flags.humidity_valid() {
            println!(
                "Humidity: {} c% | {:.02} %",
                msg.humidity,
                msg.relative_humidity(),
            );
        }
        if msg.status_flags.voc_ticks_valid() {
            println!("VOC ticks: {}", msg.voc_ticks);
        }
        if msg.status_flags.nox_ticks_valid() {
            println!("NOx ticks: {}", msg.nox_ticks);
        }
        if msg.status_flags.voc_index_valid() {
            println!("VOC index: {}", msg.voc_index);
        }
        if msg.status_flags.nox_index_valid() {
            println!("NOx index: {}", msg.nox_index);
        }
        if msg.status_flags.pm2_5_valid() {
            println!("PM2.5: {} | {}", msg.pm2_5_atm, msg.pm2_5_us_aqi());
        }
        if msg.status_flags.co2_valid() {
            println!("CO2: {}", msg.co2);
        }

        println!();

        device_stats.total_messages += 1;
        device_stats.last_seqnum = msg.sequence_number;
    }

    let total_messages: u64 = stats.values().map(|v| v.total_messages).sum();
    let missed_messages: u64 = stats.values().map(|v| v.missed_messages).sum();

    println!();
    println!("Summary");
    println!("Total messages: {total_messages}");
    println!("Missed messages {missed_messages}");
    println!("Devices: {}", stats.len());
    for (dev_sn, dev_stats) in stats.into_iter() {
        println!("  * Device SN: {:X}", dev_sn);
        println!("    Device ID: {}", dev_stats.device_id);
        println!("    Last message seqnum: {}", dev_stats.last_seqnum);
        println!("    Total messages: {}", dev_stats.total_messages);
        println!("    Missed messages {}", dev_stats.missed_messages);
    }

    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct DeviceStats {
    device_id: DeviceId,
    last_seqnum: u32,
    total_messages: u64,
    missed_messages: u64,
}
