# air-gradient-pro-rs

NOTE: this isn't a real thing yet, just a place for hacking around in renode

* renode things to look at
  - can also emulate the peripherals in Rust: https://antmicro.com/blog/2021/07/rust-peripheral-support-in-renode/
  - https://github.com/antmicro/renode-rust-example
  - (recent I2C example) https://github.com/renode/renode-infrastructure/blob/master/src/Emulator/Peripherals/Peripherals/I2C/LC709205F.cs
  - https://renode.readthedocs.io/en/latest/tutorials/zephyr-ptp-testing.html#running-the-test
  - https://github.com/renode/renode-infrastructure/tree/master/src/Emulator/Peripherals/Peripherals/Sensors
  - https://github.com/renode/renode/blob/master/tests/platforms/QuarkC1000/QuarkC1000.robot (network tests)
  - https://github.com/renode/renode/blob/master/tests/platforms/zynq-7000.robot (linux)
  - more examples (log/auto-exit): https://www.bitcraze.io/2021/04/successful-emulation/
* add features for, compile_error! for conflicts
  - log-rtt
  - log-usart3
  - panic-usart3 (switch to abort instead of loop'n)
  - panic-rtt
* console: https://github.com/rust-embedded-community/menu

NOTE: uses my fork of renode with custom peripherals:
https://github.com/renode/renode-infrastructure/compare/master...jonlamb-gh:renode-infrastructure:add-sensors

TODO 
* move Sensors/DS3231.cs to I2C/DS3231.cs or Timers/

Renode is the default runner:
```
sudo ./renode/setup-network.sh

cargo run

# also works
RENODE_PATH="/path/to/forked/renode/renode" RENODE_OPTS="--hide-monitor" cargo +nightly test
```

Robot framework driver:
```
renode-test tests.robot
```

Example responds with "hello":
```
echo "hi" | socat -t 2 - udp:192.0.2.29:12345
```

https://www.st.com/en/evaluation-tools/nucleo-f429zi.html

https://os.mbed.com/platforms/ST-Nucleo-F429ZI/

USART3 is virtual comm, D8-tx, D9-rx

https://www.airgradient.com/open-airgradient/instructions/diy-pro-v37/
https://www.airgradient.com/images/diy/schematicpro37.png
https://github.com/airgradienthq/arduino

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

SGP41 TVOC
https://crates.io/crates/sgp41
uses https://crates.io/crates/sensirion-i2c
https://sensirion.com/products/catalog/SGP41/
https://www.mouser.com/datasheet/2/682/Sensirion_Gas_Sensors_Datasheet_SGP41-2604356.pdf


## Peripheral Support

TODO - add table here with hw and renode support columns



