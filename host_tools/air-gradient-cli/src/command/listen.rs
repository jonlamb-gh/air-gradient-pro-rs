#![forbid(unsafe_code)]
#![deny(warnings, clippy::all)]

use crate::{interruptor::Interruptor, opts::Listen};
use anyhow::Result;
use chrono::prelude::*;
use std::{net::UdpSocket, time::Duration};
use wire_protocols::{
    broadcast::{Message as WireMessage, Repr as Message, MESSAGE_LEN},
    ProtocolIdentifier,
};

const TIMEOUT: Duration = Duration::from_millis(100);

pub fn listen(cmd: Listen, intr: Interruptor) -> Result<()> {
    println!(
        "Listening for UDP broadcast messages on {}:{}",
        cmd.address, cmd.port
    );

    let socket = UdpSocket::bind((cmd.address.as_str(), cmd.port))?;
    socket.set_read_timeout(TIMEOUT.into())?;

    let mut missed_messages = 0_u64;
    let mut total_messages = 0_u64;
    let mut prev_sn = None;
    let mut buf = vec![0; MESSAGE_LEN * 10];

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

        println!("Received {bytes_recvd} from {src_addr}");
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

        if let Some(psn) = prev_sn {
            if msg.sequence_number != (psn + 1) {
                eprintln!("** Missed message sequence number {}", psn + 1);
                missed_messages += 1;
            }
        }
        prev_sn = Some(msg.sequence_number);

        println!("Protocol: {}", ProtocolIdentifier::Broadcast);
        println!("Protocol version: {}", msg.protocol_version);
        println!("Firmware version: {}", msg.firmware_version);
        println!("Device ID: 0x{:X}", msg.device_id);
        println!("Device serial number: {:X}", msg.device_serial_number);
        println!("Sequence number: {}", msg.sequence_number);
        println!("Uptime seconds: {}", msg.uptime_seconds);
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
                "Temperature: {} mC, {:.02} °C, {:.02} °F",
                msg.temperature,
                raw_c_to_c(msg.temperature),
                deg_c_to_f(raw_c_to_c(msg.temperature))
            );
        }
        if msg.status_flags.humidity_valid() {
            println!(
                "Humidity: {} m%, {:.02} %",
                msg.humidity,
                raw_humidity_to_percent(msg.humidity)
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
            println!("PM2.5: {}", msg.pm2_5_atm);
        }
        if msg.status_flags.co2_valid() {
            println!("CO2: {}", msg.co2);
        }

        println!();

        total_messages += 1;
    }

    println!();
    println!("Summary");
    println!("Total messages: {total_messages}");
    println!("Missed messages {missed_messages}");

    Ok(())
}

fn raw_c_to_c(mc: i32) -> f64 {
    f64::from(mc) / 100.0
}

fn deg_c_to_f(c: f64) -> f64 {
    (c * 1.8) + 32.0
}

fn raw_humidity_to_percent(mp: u16) -> f64 {
    f64::from(mp) / 100.0
}
