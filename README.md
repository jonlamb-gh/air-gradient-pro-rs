# air-gradient-pro-rs

Firmware and tools for the [AirGradient PRO](https://www.airgradient.com/open-airgradient/kits/) kit
with some modifications.

The firmware is written in Rust and uses the [RTIC](https://rtic.rs/1/book/en/) framework.

![dashboard](resources/dashboard.png)
![startup](resources/startup.jpg)
![display.jpg](resources/display.jpg)
![prototype.jpg](resources/prototype.jpg)

## Overview

I've modified an AirGradient PRO kit ([PCB Version 3.7](https://www.airgradient.com/open-airgradient/instructions/diy-pro-v37/)) mainly so I can have a wired ethernet connection.

Significant differences from stock:
* Replace the Wemos D1 Mini v4 with an stm32f411 "black pill" board
* Add a ENC28J60 Ethernet boarda
* Custom Rust firmware

## Features

* TCP/IP stack, comes with a lightweight [broadcast protocol](libraries/wire-protocols/src/broadcast.rs)
* CLI with command-line tools and InfluxDB relaying, see the [air-gradient-cli README](host_tools/air-gradient-cli/README.md)
* Configuration for network and device settings
* OLED display

TODO maybe some pictures here

## Configuration

The [build.rs](./build.rs) file handles generating build-time configuration values based
on the github repository and host environment variables.

The following environment variables can be set:
* `AIR_GRADIENT_IP_ADDRESS` : The deivce's IP address, default is `192.168.1.38`
* `AIR_GRADIENT_MAC_ADDRESS` : The device's MAC address, default is `02:00:04:03:07:02`
* `AIR_GRADIENT_DEVICE_ID` : An arbitrary 16-bit identifier, default is `0xFFFF` (`DeviceId::DEFAULT`)
* `AIR_GRADIENT_BROADCAST_PORT` : The port number to send the broadcast protocol data on, default is `32100`
* `AIR_GRADIENT_BROADCAST_ADDRESS` : The IP address to send the broadcast protocol data to, default is `255.255.255.255`

## Flashing

You can flash the board is currently done via SWD and an st-link.

You can use the [Development Artifacts](https://github.com/jonlamb-gh/air-gradient-pro-rs/actions/workflows/dev_artifacts.yml)
github action to build a custom-configurated firmware image in CI too (click "Run workflow" and set the configuration fields).

### Using a github release artifact

1. Install [probe-rs-cli](https://crates.io/crates/probe-rs-cli)
  ```bash
  cargo install probe-rs-cli
  ```
2. Flash the target
  ```bash
  probe-rs-cli run --chip STM32F411CEUx --protocol swd path/to/air-gradient-pro
  ```

### Building from source

1. Install [cargo-embed](https://crates.io/crates/cargo-embed) and [flip-link](https://crates.io/crates/flip-link)
  ```bash
  cargo install cargo-embed flip-link
  ```
2. Build the firmware and flash the target
  ```bash
  cargo embed --release
  ```

Log messages are available on pin PA11 (USART6 Tx), you should see output like the following:

```
[I] ############################################################
[I] air-gradient-pro-rs 0.1.0 (release)
[I] Build date: Fri, 07 Apr 2023 10:00:13 +0000
[I] rustc 1.68.2 (9eb3afe9e 2023-03-27)
[I] git commit: 709ce69ea2a86585e58f07684f0def66e5f79010
[I] Serial number: 303233313036517042018
[I] Device ID: 0x1 (1)
[I] IP address: 192.168.1.38
[I] MAC address: 02-00-04-03-07-02
[I] Broadcast protocol port: 32100
[I] Broadcast protocol address: 255.255.255.255
[I] ############################################################
[I] Setup: startup delay 5 seconds
[I] Setup: S8 LP
[I] Setup: PMS5003
[I] Setup: I2C2
[I] Setup: SH1106
[I] Setup: SHT31
[I] Setup: SGP41
[I] Setup: ETH
[I] Setup: TCP/IP
[I] Setup: net clock timer
[I] Setup: net poll timer
[I] >>> Initialized <<<
```

## Hardware

* STM32F411 "back pill"
  - [WeActStudio github](https://github.com/WeActStudio/WeActStudio.MiniSTM32F4x1#stm32f411ceu6-core-board)
  - [pinout diagram](https://raw.githubusercontent.com/WeActStudio/WeActStudio.MiniSTM32F4x1/master/images/STM32F4x1_PinoutDiagram_RichardBalint.png)
  - [refman](https://www.st.com/resource/en/reference_manual/dm00119316-stm32f411xc-e-advanced-arm-based-32-bit-mcus-stmicroelectronics.pdf)
  - [datasheet](https://www.st.com/resource/en/datasheet/stm32f411ce.pdf)
* ENC28J60 Ethernet board
  - [Waveshare wiki](https://www.waveshare.com/wiki/ENC28J60_Ethernet_Board)
  - [datasheet](https://www.waveshare.com/w/upload/7/7f/ENC28J60.pdf)
* AirGradient PRO V3.7 kit
  - [kit page](https://www.airgradient.com/open-airgradient/instructions/diy-pro-v37/)
  - [schematic](https://www.airgradient.com/images/diy/schematicpro37.png)
  - [Arduino code github](https://github.com/airgradienthq/arduino)
  - [Factory firmware](https://github.com/airgradienthq/arduino/blob/master/examples/DIY_PRO_V3_7/DIY_PRO_V3_7.ino)
* SH1106 OLED
  - [datasheet](https://www.velleman.eu/downloads/29/infosheets/sh1106_datasheet.pdf)
* Sensirion SHT31 (temperature/humidity sensor)
  - [datasheet](https://www.mouser.com/datasheet/2/682/Sensirion_Humidity_Sensors_SHT3x_Datasheet_digital-971521.pdf)
* Senseair S8 LP (CO2 sensor)
  - [product page](https://senseair.com/products/size-counts/s8-lp/)
  - [doc](https://rmtplusstoragesenseair.blob.core.windows.net/docs/publicerat/PSP126.pdf)
  - [doc](https://rmtplusstoragesenseair.blob.core.windows.net/docs/Dev/publicerat/TDE2067.pdf)
* PMS5003 (particle concentration sensor)
  - [manual](https://www.aqmd.gov/docs/default-source/aq-spec/resources-page/plantower-pms5003-manual_v2-3.pdf)
* Sensirion SGP41 (TVOC/NOx sensor)
  - [product page](https://sensirion.com/products/catalog/SGP41/)
  - [datasheet](https://www.mouser.com/datasheet/2/682/Sensirion_Gas_Sensors_Datasheet_SGP41-2604356.pdf)

### Pins

| Pin   | Peripheral    | Board D1 Mini Header Pin | Description |
| :---  |    :---       |     :---                 |   :---      |
| PA11  | USART6 Tx     | TX | Console/logger/panic-handler output |
| PA12  | USART6 Rx     | RX | Console input |
| PA9   | USART1 Tx     | D3 | senseAir S8 Rx |
| PA10  | USART1 Rx     | D4 | senseAir S8 Tx |
| PA2   | USART2 Tx     | D6 | PMS5003 Rx |
| PA3   | USART2 Rx     | D5 | PMS5003 Tx |
| PB3   | I2C2 SDA      | D1 | Shared I2C SCL : SH1106, SHT31, SGP41 |
| PB10  | I2C2 SCL      | D2 | Shared I2C SDA : SH1106, SHT31, SGP41 |
| PB13  | SPI2 SCK      | NC | ENC28J60 Eth SCK |
| PB14  | SPI2 MISO     | NC | ENC28J60 Eth MISO |
| PB15  | SPI2 MOSI     | NC | ENC28J60 Eth MOSI |
| PB12  | GPIO Output   | NC | ENC28J60 Eth CS |
| PA8   | GPIO Input    | NC | ENC28J60 Eth INT |
| PB1   | GPIO Output   | NC | ENC28J60 Eth RESET |
| PC13  | GPIO Output   | NC | On-board LED |
