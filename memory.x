MEMORY
{
    /* NOTE K = KiBi = 1024 bytes */
    /* STM32F411CEUx */
    /* FLASH : ORIGIN = 0x08000000, LENGTH = 512K */
    /* RAM : ORIGIN = 0x20000000, LENGTH = 128K */

    /* Bootloader is given the first 3 sectors (16K * 3 = 48K) */
    /* Boot configuration is stored in sector 3 (16K) */
    /* Firmware starts at 0x08010000 (offset 0x10000, sector 4, 512K - 64K = 448K) */
    /* TODO - split into slots... */
    FLASH : ORIGIN = 0x08010000, LENGTH = 448K

    /* First 8 bytes are reserved for the bootloader sticky flag data */
    /* LENGTH = (128K - 8) = 131064 */
    RAM : ORIGIN = 0x20000008, LENGTH = 131064
}
