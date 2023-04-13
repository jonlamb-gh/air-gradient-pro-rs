# bootloader

Assumptions:
* bootloader will fit in sections 0..=2 (48K)
* boot (and application?) config will fit in section 3 (16K)
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

## Prototype Design

![bootloader_startup](doc/bootloader_startup.png)

![application_update.png](doc/application_update.png)
