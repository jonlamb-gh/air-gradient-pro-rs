use crate::{
    app::data_manager_task,
    config,
    sensors::{pms5003, s8lp, sgp41, sht31},
    util,
};
use log::{info, warn};
use smoltcp::{socket::udp::Socket as UdpSocket, wire::Ipv4Address};
use stm32f4xx_hal::prelude::*;
use wire_protocols::{
    broadcast::{self, Message as WireMessage, Repr as Message},
    DateTime, DeviceSerialNumber, ProtocolVersion, StatusFlags,
};

const LOCAL_EPHEMERAL_PORT: u16 = 16000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Temperature and humidity measurement from the SHT31 sensor
    Sht31Measurement(sht31::Measurement),
    /// VOC and NOx measurement from the SGP41 sensor
    Sgp41Measurement(sgp41::Measurement),
    /// PM2.5 measurement from the PMS5003 sensor
    Pms5003Measurement(pms5003::Measurement),
    /// CO2 measurement from the S8 LP sensor
    S8LpMeasurement(s8lp::Measurement),
    /// Time to send the data
    SendData,
}

pub(crate) fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg) {
    let msg = ctx.local.msg;
    let sockets = ctx.shared.sockets;
    let udp_socket_handle = ctx.shared.udp_socket;

    let socket = sockets.get_mut::<UdpSocket>(*udp_socket_handle);

    // TODO - state management, rtc, status bits, timeout/invalidate, etc
    if !msg.status_flags.initialized() {
        info!("Initializing data manager state");
        msg.device_serial_number = util::read_device_serial_number();
        msg.status_flags.set_initialized(true);
    }

    let mut send_msg = false;
    match arg {
        SpawnArg::Sht31Measurement(m) => {
            msg.temperature = m.temperature;
            msg.humidity = m.humidity;
            msg.status_flags.set_temperature_valid(true);
            msg.status_flags.set_humidity_valid(true);
        }
        // TODO - use the indices once the gas algorithm is impl'd
        SpawnArg::Sgp41Measurement(m) => {
            msg.voc_ticks = m.voc_ticks;
            msg.nox_ticks = m.nox_ticks;
            msg.status_flags.set_voc_ticks_valid(true);
            msg.status_flags.set_nox_ticks_valid(true);
        }
        SpawnArg::Pms5003Measurement(m) => {
            msg.pm2_5_atm = m.pm2_5_atm;
            msg.status_flags.set_pm2_5_valid(true);
        }
        SpawnArg::S8LpMeasurement(m) => {
            msg.co2 = m.co2;
            msg.status_flags.set_co2_valid(true);
        }
        SpawnArg::SendData => {
            // TODO
            // invalidate stale fields
            send_msg = true;
            msg.uptime_seconds += config::BCAST_INTERVAL_SEC;
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
                Err(e) => warn!("Failed to send. {e:?}"),
                Ok(buf) => {
                    let mut wire = WireMessage::new_unchecked(buf);
                    msg.emit(&mut wire);
                    info!("DM: Sent message");

                    // TODO
                    msg.sequence_number = msg.sequence_number.wrapping_add(1);
                }
            }
        } else {
            warn!("Socket cannot send");
            socket.close();
        }

        data_manager_task::spawn_after(config::BCAST_INTERVAL_SEC.secs(), SpawnArg::SendData)
            .unwrap();
    }
}

// TODO - build.rs should generate some of these
pub const fn default_bcast_message() -> Message {
    Message {
        protocol_version: ProtocolVersion::v1(),
        firmware_version: config::FIRMWARE_VERSION,
        device_id: config::DEVICE_ID,
        device_serial_number: DeviceSerialNumber::zero(),
        sequence_number: 0,
        uptime_seconds: 0,
        status_flags: StatusFlags::empty(),
        datetime: DateTime::zero(),
        temperature: 0,
        humidity: 0,
        voc_ticks: 0,
        nox_ticks: 0,
        voc_index: 0,
        nox_index: 0,
        pm2_5_atm: 0,
        co2: 0,
    }
}
