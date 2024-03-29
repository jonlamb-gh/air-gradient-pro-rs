use crate::{
    app::{data_manager_task, display_task},
    config,
    display::SystemStatus,
    sensors::{pms5003, s8lp, sgp41, sht31},
    tasks::{display::SpawnArg as DisplaySpawnArg, sgp41::GasIndices},
    util,
};
use log::{debug, warn};
use smoltcp::{
    socket::udp::{Socket as UdpSocket, UdpMetadata},
    wire::Ipv4Address,
};
use stm32f4xx_hal::prelude::*;
use wire_protocols::{
    broadcast::{Message as WireMessage, Repr as Message},
    DateTime, DeviceSerialNumber, ProtocolVersion, StatusFlags,
};

const LOCAL_EPHEMERAL_PORT: u16 = 16000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Temperature and humidity measurement from the SHT31 sensor
    Sht31Measurement(sht31::Measurement),
    /// VOC and NOx measurement from the SGP41 sensor
    Sgp41Measurement(sgp41::Measurement),
    /// VOC and NOx computed indices
    GasIndices(GasIndices),
    /// PM2.5 measurement from the PMS5003 sensor
    Pms5003Measurement(pms5003::Measurement),
    /// CO2 measurement from the S8 LP sensor
    S8LpMeasurement(s8lp::Measurement),
    /// Time to send the broadcast protocol data
    SendBroadcastMessage,
}

pub struct TaskState {
    msg: Message,
    cycles_till_warmed_up: u32,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            msg: default_bcast_message(),
            cycles_till_warmed_up: config::DATA_MANAGER_WARM_UP_PERIOD_CYCLES,
        }
    }
}

// TODO - state management, rtc, status bits, timeout/invalidate, etc
// make SystemStatus msg sn Option to indicate it on display too
pub(crate) fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg) {
    let state = ctx.local.state;
    let sockets = ctx.shared.sockets;
    let udp_socket_handle = ctx.shared.bcast_socket;

    let socket = sockets.get_mut::<UdpSocket>(*udp_socket_handle);

    if !state.msg.status_flags.initialized() {
        debug!("DM: initializing data manager state");
        state.msg.device_serial_number = util::read_device_serial_number();
        state.msg.status_flags.set_initialized(true);
    }

    let mut send_msg = false;
    match arg {
        SpawnArg::Sht31Measurement(m) => {
            state.msg.temperature = m.temperature;
            state.msg.humidity = m.humidity;
            state.msg.status_flags.set_temperature_valid(true);
            state.msg.status_flags.set_humidity_valid(true);
        }
        SpawnArg::Sgp41Measurement(m) => {
            state.msg.voc_ticks = m.voc_ticks;
            state.msg.nox_ticks = m.nox_ticks;
            state.msg.status_flags.set_voc_ticks_valid(true);
            state.msg.status_flags.set_nox_ticks_valid(true);
        }
        SpawnArg::GasIndices(m) => {
            // The gas indices are valid once they are non-zero
            if let Some(i) = m.voc_index {
                state.msg.voc_index = i.get();
                state.msg.status_flags.set_voc_index_valid(true);
            }

            if let Some(i) = m.nox_index {
                state.msg.nox_index = i.get();
                state.msg.status_flags.set_nox_index_valid(true);
            }
        }
        SpawnArg::Pms5003Measurement(m) => {
            state.msg.pm2_5_atm = m.pm2_5_atm;
            state.msg.status_flags.set_pm2_5_valid(true);
        }
        SpawnArg::S8LpMeasurement(m) => {
            state.msg.co2 = m.co2;
            state.msg.status_flags.set_co2_valid(true);
        }
        SpawnArg::SendBroadcastMessage => {
            // TODO invalidate stale fields on timer or keep valid?

            if state.cycles_till_warmed_up != 0 {
                state.cycles_till_warmed_up = state.cycles_till_warmed_up.saturating_sub(1);

                if state.cycles_till_warmed_up == 0 {
                    debug!("DM: warm up period complete");
                }
            } else {
                send_msg = true;
            }

            state.msg.uptime_seconds += config::BCAST_INTERVAL_SEC;

            data_manager_task::spawn_after(
                config::BCAST_INTERVAL_SEC.secs(),
                SpawnArg::SendBroadcastMessage,
            )
            .unwrap();
        }
    }

    if send_msg {
        if !socket.is_open() {
            socket.bind(LOCAL_EPHEMERAL_PORT).unwrap();
        }

        if socket.can_send() {
            let endpoint = (
                Ipv4Address(config::BROADCAST_ADDRESS),
                config::BROADCAST_PORT,
            )
                .into();
            let meta = Default::default();
            match socket.send(state.msg.message_len(), UdpMetadata { endpoint, meta }) {
                Err(e) => warn!("Failed to send. {e:?}"),
                Ok(buf) => {
                    let mut wire = WireMessage::new_unchecked(buf);
                    state.msg.emit(&mut wire);
                    debug!("DM: Sent message sn {}", state.msg.sequence_number);
                    state.msg.sequence_number = state.msg.sequence_number.wrapping_add(1);
                }
            }
        } else {
            warn!("Socket cannot send");
            socket.close();
        }

        let sys_status = SystemStatus {
            pm2_5: state
                .msg
                .status_flags
                .pm2_5_valid()
                .then_some(state.msg.pm2_5_atm),
            temp: state
                .msg
                .status_flags
                .temperature_valid()
                .then_some(state.msg.temperature),
            humidity: state
                .msg
                .status_flags
                .humidity_valid()
                .then_some(state.msg.humidity),
            co2: state.msg.status_flags.co2_valid().then_some(state.msg.co2),
            voc_index: state
                .msg
                .status_flags
                .voc_index_valid()
                .then_some(state.msg.voc_index),
            nox_index: state
                .msg
                .status_flags
                .nox_index_valid()
                .then_some(state.msg.nox_index),
            msg_seqnum: state.msg.sequence_number,
        };
        display_task::spawn(DisplaySpawnArg::SystemStatus(sys_status)).unwrap();
    }
}

const fn default_bcast_message() -> Message {
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
