MEMORY
{
    /* NOTE K = KiBi = 1024 bytes */
    /* STM32F411CEUx */
    /* FLASH : ORIGIN = 0x08000000, LENGTH = 512K */
    /* RAM : ORIGIN = 0x20000000, LENGTH = 128K */

    /* Bootloader is given the first 3 sectors (16K * 3 = 48K) */
    /* Boot configuration is stored in sector 3 (16K) */
    /* Firmware slot 0 starts at 0x08010000 (offset 0x10000, sectors 4 and 5, 64K + 128K = 194K) */
    /* Slot 1 is bigger (sectors 6 and 7, but we use the min of the two) */
    /* Use slot 0 here since initial programming must write to slot 0 */
    FLASH : ORIGIN = 0x08010000, LENGTH = 194K

    /* First 16 bytes are reserved for the UCS words */
    /* LENGTH = (128K - 16) = 131056 */
    RAM : ORIGIN = 0x20000010, LENGTH = 131056
}
