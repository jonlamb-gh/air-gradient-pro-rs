MEMORY
{
    /* NOTE K = KiBi = 1024 bytes */
    /* STM32F411CEUx */
    /* FLASH : ORIGIN = 0x08000000, LENGTH = 512K */
    /* RAM : ORIGIN = 0x20000000, LENGTH = 128K */

    /* Bootloader is given the first 3 sectors (16K * 3 = 48K) */
    FLASH : ORIGIN = 0x08000000, LENGTH = 48K
    
    /* First 8 bytes are reserved for the UCS words */
    /* LENGTH = (128K - 8) = 131064 */
    RAM : ORIGIN = 0x20000008, LENGTH = 131064
}
