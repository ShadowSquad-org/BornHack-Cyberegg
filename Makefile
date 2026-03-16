ELF     = target/thumbv7em-none-eabihf/release/embassy
BIN     = target/thumbv7em-none-eabihf/release/embassy.bin
UF2     = embassy.uf2

# nRF52840 flash base (from memory.x) and Adafruit UF2 family ID
FLASH_BASE  = 0x00027000
UF2_FAMILY  = 0xADA52840

.PHONY: uf2 fw sim flash monitor

uf2: $(UF2)

$(UF2): $(BIN)
	uf2conv --base $(FLASH_BASE) --family $(UF2_FAMILY) --output $(UF2) $(BIN)

$(BIN): $(ELF)
	arm-none-eabi-objcopy -O binary $(ELF) $(BIN)

$(ELF):
	cargo fw-uf2

fw:
	cargo fw

sim:
	cargo sim

flash:
	cargo fw-flash

monitor:
	probe-rs attach --chip nRF52840_xxAA target/thumbv7em-none-eabihf/debug/embassy
