# air-gradient-pro-rs

do all the `rg -i todo .`

make gh issues for things

bring back renode emulation support

try out https://github.com/rtic-scope

hook up a console for enable/disable debug logs or whatever
https://github.com/rust-embedded-community/menu
https://crates.io/crates/ushell

maybe add a bootloader w/FOTA update stuff and CLI for it

add AQI on-device for display and in the protocol
https://github.com/kelnos/aqi-rs

reset happened:
```
2023-04-04T19:08:00Z,27,sequence_number
2023-04-05T09:12:00Z,10074,sequence_number
**
2023-04-05T09:16:00Z,8,sequence_number
2023-04-05T09:20:00Z,55
```

```
********************************
PANIC
panicked at 'assertion failed: buffer.as_slice().len() >= usize(self.len())', ~/.cargo/git/checkouts/enc28j60-53056b08aeec0867/b006635/src/lib.rs:764:9
********************************

NextPacket::read
assert!(buffer.as_slice().len() >= usize(self.len()));
```


```
# send-huge-packet.sh 
# this repro's it

dd if=/dev/zero bs=1024 count=32 | perl -MIO::Socket::INET -e \
   'IO::Socket::INET->new(PeerAddr => q[192.168.1.38:1234], Proto => q[udp])->send(do { local $/; <STDIN> })'
```

```
config section
env vars
AIR_GRADIENT_IP_ADDRESS
AIR_GRADIENT_MAC_ADDRESS
AIR_GRADIENT_DEVICE_ID
AIR_GRADIENT_BROADCAST_PORT
AIR_GRADIENT_BROADCAST_ADDRESS
```


instructions for build+flash via `cargo embed --release`
and download+flash via `probe-rs ...`


https://raw.githubusercontent.com/WeActStudio/WeActStudio.MiniSTM32F4x1/master/images/STM32F4x1_PinoutDiagram_RichardBalint.png

https://www.waveshare.com/wiki/ENC28J60_Ethernet_Board

https://www.airgradient.com/open-airgradient/instructions/diy-pro-v37/
https://www.airgradient.com/images/diy/schematicpro37.png
https://github.com/airgradienthq/arduino
https://github.com/airgradienthq/arduino/blob/master/examples/DIY_PRO_V3_7/DIY_PRO_V3_7.ino

use their defaults:
https://github.com/airgradienthq/arduino/blob/master/examples/DIY_PRO_V3_7/DIY_PRO_V3_7.ino#L267

128x64 display U8G2 SH1106
https://github.com/olikraus/u8g2
https://crates.io/crates/sh1106
https://www.velleman.eu/downloads/29/infosheets/sh1106_datasheet.pdf

SHT31
https://crates.io/crates/sht3x
https://www.mouser.com/datasheet/2/682/Sensirion_Humidity_Sensors_SHT3x_Datasheet_digital-971521.pdf
https://github.com/renode/renode-infrastructure/blob/master/src/Emulator/Peripherals/Peripherals/I2C/SHT21.cs

senseAir S8 LP
CO2 sensor
serial modbus
https://senseair.com/products/size-counts/s8-lp/
https://rmtplusstoragesenseair.blob.core.windows.net/docs/publicerat/PSP126.pdf
https://rmtplusstoragesenseair.blob.core.windows.net/docs/Dev/publicerat/TDE2067.pdf
https://github.com/alttch/rmodbus
https://github.com/slowtec/modbus-core

PMS5003
https://crates.io/crates/pms-7003
https://crates.io/crates/pms700x
https://www.aqmd.gov/docs/default-source/aq-spec/resources-page/plantower-pms5003-manual_v2-3.pdf

SGP41 TVOC
https://crates.io/crates/sgp41
uses https://crates.io/crates/sensirion-i2c
https://sensirion.com/products/catalog/SGP41/
https://www.mouser.com/datasheet/2/682/Sensirion_Gas_Sensors_Datasheet_SGP41-2604356.pdf
https://github.com/Sensirion/arduino-i2c-sgp41
https://github.com/Sensirion/gas-index-algorithm
https://github.com/Sensirion/arduino-gas-index-algorithm

## Pins

| Pin   | Peripheral    | Description |
| :---  |    :---       |        ---: |
| PA11  | USART6 Tx     | Console/logger/panic-handler output |
| PA12  | USART6 Rx     | Console input |
| PA9   | USART1 Tx     | senseAir S8 Rx |
| PA10  | USART1 Rx     | senseAir S8 Tx |
| PA2   | USART2 Tx     | PMS5003 Rx |
| PA3   | USART2 Rx     | PMS5003 Tx |
| PB6   | I2C1 SCL      | DS3231 RTC SCL |
| PB7   | I2C1 SDA      | DS3231 RTC SDA |
| PB3   | I2C2 SDA      | Shared I2C SCL : SH1106, SHT31, SGP41 |
| PB10  | I2C2 SCL      | Shared I2C SDA : SH1106, SHT31, SGP41 |
| PB13  | SPI2 SCK      | ENC28J60 Eth SCK |
| PB14  | SPI2 MISO     | ENC28J60 Eth MISO |
| PB15  | SPI2 MOSI     | ENC28J60 Eth MOSI |
| PB12  | GPIO Output   | ENC28J60 Eth CS |
| PA8   | GPIO Input    | ENC28J60 Eth INT |
| PB1   | GPIO Output   | ENC28J60 Eth RESET |
| PC13  | GPIO Output   | On-board LED |
