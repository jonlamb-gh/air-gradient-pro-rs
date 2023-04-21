# air-gradient CLI

Command line tool for interacting with the air-gradient-pro firmware and protocol data.

TODO add all the subcommands and examples here

## listen

Listen for broadcast messages

```bash
$ air-gradient listen
```

```
Listening for UDP broadcast messages on 0.0.0.0:32100

Received 60 bytes from 192.168.1.38:16000
UTC: 2023-04-07 13:09:56.897257022 UTC
Protocol: broadcast
Protocol version: 1
Firmware version: 0.1.0
Device ID: 0x1 (1)
Device serial number: 303233313036517042018
Sequence number: 244
Uptime seconds: 1345 | 22m 25s
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
Temperature: 2259 cC | 22.59 °C | 72.66 °F
Humidity: 3806 c% | 38.06 %
VOC ticks: 30710
NOx ticks: 14983
VOC index: 102
NOx index: 1
PM2.5: 0 | AQI: 0, Good
CO2: 1316

^C
Summary
Total messages: 1
Missed messages 0
```

## influx-relay

Relay the broadcast messages to InfluxDB.

See the `--help` output for configuration.

```bash
$ air-gradient influx-relay
```
