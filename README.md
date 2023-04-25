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
* Add a ENC28J60 Ethernet board
* Custom bootloader and firmware, written in Rust

## Features

* Bootloader with firmware update and failover rollback mechanisms
  - see the [bootloader README](bootloader/README.md)
* TCP/IP stack, comes with these protocols:
  - a lightweight [broadcast protocol](libraries/wire-protocols/src/broadcast.rs) for influx/etc integration
  - a [device protocol](libraries/wire-protocols/src/device.rs) for FOTA updates, device info, and device control
* CLI with command-line tools and InfluxDB relaying, see the [air-gradient-cli README](host_tools/air-gradient-cli/README.md)
* Configuration for network and device settings
* OLED display

## Configuration

The [build.rs](./build.rs) file handles generating build-time configuration values based
on the github repository and host environment variables.

The following environment variables can be set:
* `AIR_GRADIENT_IP_ADDRESS` : The device's IP address, default is `192.168.1.38`
* `AIR_GRADIENT_MAC_ADDRESS` : The device's MAC address, default is `02:00:04:03:07:02`
* `AIR_GRADIENT_DEVICE_ID` : An arbitrary 16-bit identifier, default is `0xFFFF` (`DeviceId::DEFAULT`)
* `AIR_GRADIENT_BROADCAST_PORT` : The port number to send the broadcast protocol data on, default is `32100`
* `AIR_GRADIENT_BROADCAST_ADDRESS` : The IP address to send the broadcast protocol data to, default is `255.255.255.255`
* `AIR_GRADIENT_DEVICE_PORT` : The port number the device protocol socket listens on, default is `32101`

## FOTA Updating

Update files (`agp_images.cpio`) are generated by a custom linker (see its [README](host_tools/agp-linker/README.md))
as part of the build process (`cargo build --release`).

* See the [device update](host_tools/air-gradient-cli/README.md#device-update) section of the
  CLI for more information on using the CLI.
* See the [Firmware Update Sequence](bootloader/README.md#update-sequence) section of the
  bootloader for more information on performing FOTA updates.
* See the [Design](bootloader/README.md#design) section of the bootloader for more
  information on how the update protocol and failover mechanism works.
* See the [Example Update Log](bootloader/README.md#example-update-log) section of the bootloader
  for example output from the bootloader and firmware throughout the update process.

```bash
air-gradient device update --address 192.168.1.38 path/to/agp_images.cpio
```

## Flashing

Initial flashing of the bootloader and firmware onto the board is currently done via SWD and an st-link.

The default [memory.x](memory.x) file is setup to use firmware slot 0 in flash, which is also
the default slot picked by the bootloader on initial setup.

You can use the [Development Artifacts](https://github.com/jonlamb-gh/air-gradient-pro-rs/actions/workflows/dev_artifacts.yml)
github action to build a custom-configured bootloader and firmware image in CI (click "Run workflow" and set the configuration fields)
or grab the latest release with the default configuration from the [Releases page](https://github.com/jonlamb-gh/air-gradient-pro-rs/releases).

### Using a github release artifact

1. Install [probe-rs-cli](https://crates.io/crates/probe-rs-cli)
  ```bash
  cargo install probe-rs-cli
  ```
2. Flash the target using the ELF files
  ```bash
  probe-rs-cli run --chip STM32F411CEUx --protocol swd path/to/bootloader
  probe-rs-cli run --chip STM32F411CEUx --protocol swd path/to/air-gradient-pro
  ```

### Building from source

1. Install [cargo-embed](https://crates.io/crates/cargo-embed) and [flip-link](https://crates.io/crates/flip-link)
  ```bash
  cargo install cargo-embed flip-link
  ```
2. Build the bootloader and flash the target
  ```bash
  cd bootloader/
  cargo embed --release
  ```
2. Build the firmware and flash the target
  ```bash
  cd firmware/
  cargo embed --release
  ```

Log messages are available on pin PA11 (USART6 Tx), you should see output like the following:

```
************************************************************
agp-bootloader 0.1.0 (release)
Build date: Mon, 24 Apr 2023 14:28:38 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 3023a001f2ab011406a3e58dd8e328cb4502737a
Reset reason: Software reset
Boot config slot: SLOT0
Update pending: false
Update valid: false
************************************************************
############################################################
air-gradient-pro-rs 0.2.0 (release)
Build date: Mon, 24 Apr 2023 14:35:08 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 3023a001f2ab011406a3e58dd8e328cb4502737a
Serial number: 303233313036517042018
Device ID: 0x1 (1)
IP address: 192.168.1.38
MAC address: 02-00-04-03-07-02
Broadcast protocol port: 32100
Broadcast protocol address: 255.255.255.255
Device protocol port: 32101
Reset reason: Software reset
Update pending: false
############################################################
Setup: startup delay 5 seconds
Setup: boot config
Setup: S8 LP
Setup: PMS5003
Setup: I2C2
Setup: SH1106
Setup: SHT31
Setup: SGP41
Setup: ETH
Setup: TCP/IP
Setup: net clock timer
Setup: net poll timer
>>> Initialized <<<
```

## Hardware

* STM32F411 "black pill"
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
* AMS1117-3.3 regulator
  - [Amazon link](https://www.amazon.com/gp/product/B07CP4P5XJ/ref=ppx_yo_dt_b_asin_title_o00_s00?ie=UTF8&psc=1)

### Pins

| Pin   | Peripheral    | Board D1 Mini Header Pin | Description |
| :---  |    :---       |     :---                 |   :---      |
| PA11  | USART6 Tx     | TX | Console/logger/panic-handler output |
| PA12  | USART6 Rx     | RX | Console input (not used currenlty) |
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

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
