use crate::{
    interruptor::Interruptor,
    measurement::{Measurement, MeasurementFields, MeasurementTags, MessageExt},
    opts::InfluxRelay,
};
use anyhow::Result;
use chrono::prelude::*;
use futures::prelude::*;
use influxdb2::Client;
use tokio::net::UdpSocket;
use wire_protocols::broadcast::{Message as WireMessage, Repr as Message, MESSAGE_LEN};

pub async fn influx_relay(cmd: InfluxRelay, intr: Interruptor) -> Result<()> {
    tracing::info!(
        address = cmd.address,
        port = cmd.port,
        host = cmd.host,
        org = cmd.org,
        "Relaying UDP broadcast messages to influx",
    );

    let mut buf = vec![0; MESSAGE_LEN * 10];

    let s = std::net::UdpSocket::bind((cmd.address.as_str(), cmd.port))?;
    s.set_nonblocking(true)?;
    let socket = UdpSocket::from_std(s)?;

    let client = Client::new(&cmd.host, &cmd.org, &cmd.token);

    loop {
        if intr.is_set() {
            break;
        }

        let (bytes_recvd, src_addr) = socket.recv_from(&mut buf).await?;
        let recv_utc: DateTime<Utc> = Utc::now();

        tracing::debug!(
            src = %src_addr,
            bytes_recvd = bytes_recvd,
            "Received message data"
        );

        // TODO - walk entire buffer for possible multiple messages
        let wire_msg = match WireMessage::new_checked(&buf) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!(e = %e, "Failed to parse as broadcast wire message");
                continue;
            }
        };

        let msg = match Message::parse(&wire_msg) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!(e = %e, "Failed to parse as broadcast message");
                continue;
            }
        };

        let (pm25, aqi, aqi_level) = if msg.status_flags.pm2_5_valid() {
            let aqi = msg.pm2_5_us_aqi();
            (
                i64::from(msg.pm2_5_atm).into(),
                i64::from(aqi.aqi()).into(),
                aqi.level().to_string().into(),
            )
        } else {
            (None, None, None)
        };

        let m = Measurement {
            recv_time_utc_ns: recv_utc.timestamp_nanos(),
            tags: MeasurementTags {
                device_id: msg.device_id.to_string(),
                device_serial_number: format!("{:X}", msg.device_serial_number),
                firmware_version: msg.firmware_version.to_string(),
            },
            fields: MeasurementFields {
                sequence_number: msg.sequence_number.into(),
                temperature: if msg.status_flags.temperature_valid() {
                    msg.temperature_f().into()
                } else {
                    None
                },
                humidity: if msg.status_flags.humidity_valid() {
                    msg.relative_humidity().into()
                } else {
                    None
                },
                voc_ticks: if msg.status_flags.voc_ticks_valid() {
                    i64::from(msg.voc_ticks).into()
                } else {
                    None
                },
                nox_ticks: if msg.status_flags.nox_ticks_valid() {
                    i64::from(msg.nox_ticks).into()
                } else {
                    None
                },
                voc_index: if msg.status_flags.voc_index_valid() {
                    i64::from(msg.voc_index).into()
                } else {
                    None
                },
                nox_index: if msg.status_flags.nox_index_valid() {
                    i64::from(msg.nox_index).into()
                } else {
                    None
                },
                pm25,
                aqi,
                aqi_level,
                co2: if msg.status_flags.co2_valid() {
                    i64::from(msg.co2).into()
                } else {
                    None
                },
            },
        }
        .into_data_point(&cmd.measurement_name)?;

        if let Err(e) = client
            .write(&cmd.bucket, stream::iter(std::iter::once(m)))
            .await
        {
            tracing::error!(error = %e, "Failed to write measurement");
        }
    }

    tracing::debug!("Exiting relay loop");

    Ok(())
}
