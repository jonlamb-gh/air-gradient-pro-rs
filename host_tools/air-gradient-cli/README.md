# air-gradient CLI

Command line tool for interacting with the air-gradient-pro firmware and protocol data.

## listen

Listen for broadcast messages

```bash
$ air-gradient listen
```

```
Listening for UDP broadcast messages on 0.0.0.0:32100

Received 60 bytes from 192.168.1.38:16000
UTC: 2023-04-24 15:48:26.375294904 UTC
Protocol: broadcast
Protocol version: 1
Firmware version: 0.2.0
Device ID: 0x1 (1)
Device serial number: 303233313036517042018
Sequence number: 470
Uptime seconds: 2475 | 41m 15s
Status flags: 0x1FE2
  initialized: true
  datetime_valid: false
  temperature_valid: true
  humidity_valid: true
  voc_ticks_valid: true
  nox_ticks_valid: true
  voc_index_valid: true
  nox_index_valid: true
  pm2_5_valid: true
  co2_valid: true
Temperature: 2121 cC | 21.21 °C | 70.18 °F
Humidity: 4110 c% | 41.10 %
VOC ticks: 30990
NOx ticks: 14715
VOC index: 98
NOx index: 1
PM2.5: 0 | AQI: 0, Good
CO2: 820

^C
Summary
Total messages: 1
Missed messages 0
Devices: 1
  * Device SN: 303233313036517042018
    Device ID: 1
    Last message seqnum: 470
    Total messages: 1
    Missed messages 0
```

## influx-relay

Relay the broadcast messages to InfluxDB.

See the `--help` output for configuration.

```bash
$ air-gradient influx-relay
```

## extract-archive

Extract firmware ELF files from an archive file

```bash
$ air-gradient extract-archive agp_images.cpio
```

```
Extracting 'agp_images.cpio' to '.'
Writing ELF './agp0.elf'
Writing bin './agp0.bin'
Writing ELF './agp1.elf'
Writing bin './agp1.bin'
```

## device

Subcommands for interacting with a device over the network

### device info

Request and print device info

```bash
$ air-gradient device info --address 192.168.1.38
```

```
Requesting device info from 192.168.1.38:32101
Status: Success
DeviceInfo {
    protocol_version: "1",
    firmware_version: "0.2.0",
    device_id: 1,
    device_serial_number: "303233313036517042018",
    mac_address: [
        2,
        0,
        4,
        3,
        7,
        2,
    ],
    active_boot_slot: Slot0,
    reset_reason: "Power-on reset",
    built_time_utc: "Mon, 24 Apr 2023 15:06:18 +0000",
    git_commit: "0a358262c4cb7580d7b64f995675903f2be02a7d",
}
```

### device reboot

Reboot a device

```bash
$ air-gradient device reboot --address 192.168.1.38
```

```
Rebooting device 192.168.1.38:32101
Status: Success
```

### device update

Perform a firmware update

```bash
$ air-gradient device update --address 192.168.1.38 /tmp/agp_images.cpio
```

```
Updating system from 192.168.1.38:32101 with image archive '/tmp/agp_images.cpio'
DeviceInfo {
    protocol_version: "1",
    firmware_version: "0.2.0",
    device_id: 1,
    device_serial_number: "303233313036517042018",
    active_boot_slot: Slot0,
    reset_reason: "Power-on reset",
    built_time_utc: "Mon, 24 Apr 2023 15:06:18 +0000",
    git_commit: "0a358262c4cb7580d7b64f995675903f2be02a7d",
}
Erasing sectors for boot slot SLOT1
Erase status: Success
Wrting bin to boot slot SLOT1, 161888 bytes
Verifying image currently in SLOT1
Update complete, issue reboot command
```
