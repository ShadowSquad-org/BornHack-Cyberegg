MEMORY
{
  /* Standalone factory test — no bootloader, runs from reset vector. */
  FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 1024K
  RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 256K
}
