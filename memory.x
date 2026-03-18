MEMORY
{
  BOOTLOADER       (rx)  : ORIGIN = 0x00000000, LENGTH = 48K
  BOOTLOADER_STATE (rw)  : ORIGIN = 0x0000C000, LENGTH = 4K
  FLASH            (rx)  : ORIGIN = 0x0000D000, LENGTH = 480K
  DFU              (rw)  : ORIGIN = 0x00085000, LENGTH = 484K
  RAM              (rwx) : ORIGIN = 0x20000000, LENGTH = 256K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE);
__bootloader_state_end   = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE);
__bootloader_dfu_start = ORIGIN(DFU);
__bootloader_dfu_end   = ORIGIN(DFU) + LENGTH(DFU);
