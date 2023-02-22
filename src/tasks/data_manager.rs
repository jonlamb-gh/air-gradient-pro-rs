use crate::firmware_main::app::data_manager_task;
use crate::sensors::{sgp41, sht31};
use log::{info, warn};
use smoltcp::{socket::UdpSocket, wire::Ipv4Address};
use stm32f4xx_hal::prelude::*;
use wire_protocols::{
    broadcast::{self, Message as WireMessage, Repr as Message},
    DateTime, DeviceId, DeviceSerialNumber, FirmwareVersion, ProtocolVersion, StatusFlags,
};

const LOCAL_EPHEMERAL_PORT: u16 = 16000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Temperature and humidity measurement from the SHT31 sensor
    Sht31Measurement(sht31::Measurement),
    /// VOC and NOx measurement from the SGP41 sensor
    Sgp41Measurement(sgp41::Measurement),
    /// Time to send the data
    SendData,
    // TODO
    // - udp send deadline reached/timer stuff
}

// TODO - takes an enum arg, each sensor task sends/spawns this task
pub(crate) fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg) {
    let rtc = ctx.local.rtc;
    let msg = ctx.local.msg;
    let net = ctx.shared.net;
    let udp_socket_handle = ctx.shared.udp_socket;

    info!("Data manager task updating reason={}", arg.as_reason());

    let socket = net.get_socket::<UdpSocket>(*udp_socket_handle);

    // TODO - state management, rtc, status bits, timeout/invalidate, etc

    let mut send_msg = false;
    match arg {
        SpawnArg::Sht31Measurement(m) => {
            msg.temperature = m.temperature;
            msg.humidity = m.humidity;
            msg.status_flags.set_temperature_valid(true);
            msg.status_flags.set_humidity_valid(true);
        }
        SpawnArg::Sgp41Measurement(m) => {
            msg.voc_ticks = m.voc_ticks;
            msg.nox_ticks = m.nox_ticks;
            msg.status_flags.set_voc_ticks_valid(true);
            msg.status_flags.set_nox_ticks_valid(true);
        }
        SpawnArg::SendData => {
            // TODO - grab rtc DateTime
            send_msg = true;
        }
    }

    if send_msg {
        if !socket.is_open() {
            socket.bind(LOCAL_EPHEMERAL_PORT).unwrap();
        }

        if socket.can_send() {
            match socket.send(
                msg.message_len(),
                (Ipv4Address::BROADCAST, broadcast::DEFAULT_PORT).into(),
            ) {
                Err(e) => warn!("Failed to send. {e}"),
                Ok(buf) => {
                    let mut wire = WireMessage::new_unchecked(buf);
                    msg.emit(&mut wire);
                    info!("Sent message");

                    // TODO
                    msg.uptime_seconds = msg.uptime_seconds.saturating_add(2);
                }
            }
        } else {
            warn!("Socket cannot send");
            socket.close();
        }

        // TODO - send data period
        data_manager_task::spawn_after(2.secs(), SpawnArg::SendData).unwrap();
    }
}

impl SpawnArg {
    fn as_reason(&self) -> &'static str {
        match self {
            SpawnArg::Sht31Measurement(_) => "sht31-measurement",
            SpawnArg::Sgp41Measurement(_) => "sgp41-measurement",
            SpawnArg::SendData => "send-data",
        }
    }
}

pub const fn default_bcast_message() -> Message {
    Message {
        protocol_version: ProtocolVersion::v1(),
        firmware_version: FirmwareVersion::new(0, 1, 0),
        device_id: DeviceId::new(0xAB),
        device_serial_number: DeviceSerialNumber::new(1, 2, 3),
        datetime: DateTime::zero(),
        uptime_seconds: 0,
        status_flags: StatusFlags::empty(),
        temperature: 0,
        humidity: 0,
        voc_ticks: 0,
        nox_ticks: 0,
    }
}
