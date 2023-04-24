# bootloader

Bootloader for the AirGradient Pro firmware

## Assumptions

* bootloader will fit in sections 0..=2 (48K)
* boot and application config will fit in section 3 (16K)
* application will fit in <= 194K 
  - sectors 4 + 5 == 194K, 6 + 7 = 256K

## Memory Map

NOTE: K = KiBi = 1024 bytes

| Sector | Address     | Size  | Function |
| :---:  | :---:       | :---: | :---:    |
| 0      | 0x0800_0000 | 16K   | bootloader firmware |
| 1      | 0x0800_4000 | 16K   | bootloader firmware |
| 2      | 0x0800_8000 | 16K   | bootloader firmware |
| 3      | 0x0800_C000 | 16K   | boot and application configuration |
| 4      | 0x0801_0000 | 64K   | application firmware slot 0 |
| 5      | 0x0802_0000 | 128K  | application firmware slot 0 |
| 6      | 0x0804_0000 | 128K  | application firmware slot 1 |
| 7      | 0x0806_0000 | 128K  | application firmware slot 1 |

## Update Sequence

![fw_update_sequence.png](../doc/fw_update_sequence.png)

## Design

Note that these are somewhat incomplete.

![bootloader_startup](../doc/bootloader_startup.png)

![application_update.png](../doc/application_update.png)

## Example Update Log

Initial boot:
```
************************************************************
agp-bootloader 0.1.0 (release)
Build date: Mon, 24 Apr 2023 19:32:59 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
Reset reason: Power-on reset
Boot config slot: SLOT0
Update pending: false
Update valid: false
************************************************************
############################################################
air-gradient-pro-rs 0.2.0 (release)
Build date: Mon, 24 Apr 2023 19:33:40 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
Serial number: 303233313036517042018
Device ID: 0x1 (1)
IP address: 192.168.1.38
MAC address: 02-00-04-03-07-02
Broadcast protocol port: 32100
Broadcast protocol address: 255.255.255.255
Device protocol port: 32101
Reset reason: Power-on reset
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

Update upload completed, CLI triggered a complete-and-reboot, new firmware ACK's the update:
```
[W] Update complete, rebooting now
************************************************************
agp-bootloader 0.1.0 (release)
Build date: Mon, 24 Apr 2023 19:32:59 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
Reset reason: Software reset
Boot config slot: SLOT0
Update pending: true
Update valid: false
************************************************************
############################################################
air-gradient-pro-rs 0.2.1 (release)
Build date: Mon, 24 Apr 2023 19:34:32 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
Serial number: 303233313036517042018
Device ID: 0x1 (1)
IP address: 192.168.1.38
MAC address: 02-00-04-03-07-02
Broadcast protocol port: 32100
Broadcast protocol address: 255.255.255.255
Device protocol port: 32101
Reset reason: Software reset
Update pending: true
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
New application update checks out, marking for BC flash and reseting
```

New firmware slot is update in FLASH configs:
```
************************************************************
agp-bootloader 0.1.0 (release)
Build date: Mon, 24 Apr 2023 19:32:59 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
Reset reason: Software reset
Boot config slot: SLOT0
Update pending: true
Update valid: true
************************************************************
############################################################
air-gradient-pro-rs 0.2.1 (release)
Build date: Mon, 24 Apr 2023 19:34:32 +0000
Compiler: rustc 1.69.0 (84c898d65 2023-04-16)
Commit: 0a358262c4cb7580d7b64f995675903f2be02a7d
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
