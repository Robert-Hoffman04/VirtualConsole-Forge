# Forwarder build.
#
#   make            build/main.dol   the generic loader (needs devkitPPC + libogc + libfat)
#   make booter     build/booter.bin the freestanding second stage (any powerpc gcc)
#   make test       layout/DOL tests, cross-built for big-endian PPC and run under qemu-ppc
#   make install    copy build/main.dol to prebuilt/main.dol (what vc-cli --forwarder ships)
#
# Toolchain overrides:
#   CROSS=powerpc-linux-gnu-     use a different PPC toolchain for `make booter` / `make test`
#   DEVKITPRO / DEVKITPPC        as installed by devkitPro

DEVKITPRO ?= /opt/devkitpro
DEVKITPPC ?= $(DEVKITPRO)/devkitPPC
CROSS     ?= $(DEVKITPPC)/bin/powerpc-eabi-
ELF2DOL   ?= $(DEVKITPRO)/tools/bin/elf2dol
LIBOGC    ?= $(DEVKITPRO)/libogc

TESTCROSS ?= powerpc-linux-gnu-
QEMU      ?= qemu-ppc

BUILD := build
INC   := include

# --- booter: freestanding, no libc, linked at VCF_BOOTER_ADDR ------------------
BOOTER_CFLAGS := -Os -Wall -Wextra -Werror -mcpu=750 -msoft-float -msdata=none \
                 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector \
                 -fno-asynchronous-unwind-tables -fno-tree-loop-distribute-patterns -I $(INC)

# --- loader: normal libogc program ---------------------------------------------
MACHDEP := -DGEKKO -mrvl -mcpu=750 -meabi -mhard-float
CFLAGS  := -O2 -Wall -Wextra $(MACHDEP) -I $(INC) -I $(LIBOGC)/include
LDFLAGS := $(MACHDEP) -Wl,-Map,$(BUILD)/main.map -L$(LIBOGC)/lib/wii
LIBS    := -lfat -logc -lm

.PHONY: all booter test check-cfg install clean
all: $(BUILD)/main.dol
booter: $(BUILD)/booter.bin

$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/booter.ld: booter/booter.ld.in $(INC)/vcf_memmap.h | $(BUILD)
	$(CROSS)gcc -E -P -x c -I $(INC) $< -o $@

$(BUILD)/crt0.o: booter/crt0.S $(INC)/vcf_memmap.h | $(BUILD)
	$(CROSS)gcc $(BOOTER_CFLAGS) -c $< -o $@

$(BUILD)/booter.o: booter/booter.c $(INC)/vcf_dol.h $(INC)/vcf_memmap.h | $(BUILD)
	$(CROSS)gcc $(BOOTER_CFLAGS) -c $< -o $@

$(BUILD)/booter.elf: $(BUILD)/crt0.o $(BUILD)/booter.o $(BUILD)/booter.ld
	$(CROSS)gcc -nostdlib -nostartfiles -static -no-pie -Wl,--build-id=none -Wl,-T,$(BUILD)/booter.ld \
	    $(BUILD)/crt0.o $(BUILD)/booter.o -o $@

$(BUILD)/booter.bin: $(BUILD)/booter.elf
	$(CROSS)objcopy -O binary $< $@

# Run from inside build/ so the symbols are _binary_booter_bin_{start,end}.
$(BUILD)/booter_bin.o: $(BUILD)/booter.bin
	cd $(BUILD) && $(CROSS)objcopy -I binary -O elf32-powerpc -B powerpc:common booter.bin booter_bin.o

$(BUILD)/main.o: loader/main.c $(INC)/vcf_launch.h $(INC)/vcf_dol.h $(INC)/vcf_memmap.h | $(BUILD)
	$(CROSS)gcc $(CFLAGS) -c $< -o $@

$(BUILD)/main.elf: $(BUILD)/main.o $(BUILD)/booter_bin.o
	$(CROSS)gcc $(LDFLAGS) $^ $(LIBS) -o $@

$(BUILD)/main.dol: $(BUILD)/main.elf
	$(ELF2DOL) $< $@

install: $(BUILD)/main.dol
	mkdir -p prebuilt && cp $< prebuilt/main.dol

# Validate a launch.cfg from `vc-cli --launch-cfg-out` with the loader's own C checks.
check-cfg: $(BUILD)/check_launch_cfg
	$(QEMU) $< $(CFG)

# --- tests ---------------------------------------------------------------------
test: $(BUILD)/test_launch $(BUILD)/test_dol
	$(QEMU) $(BUILD)/test_launch
	$(QEMU) $(BUILD)/test_launch --dump | diff - tests/golden_launch.txt
	$(QEMU) $(BUILD)/test_dol

$(BUILD)/test_%: tests/test_%.c $(INC)/*.h | $(BUILD)
	$(TESTCROSS)gcc -std=gnu17 -O1 -Wall -Wextra -Werror -static -I $(INC) $< -o $@

$(BUILD)/check_%: tests/check_%.c $(INC)/*.h | $(BUILD)
	$(TESTCROSS)gcc -std=gnu17 -O1 -Wall -Wextra -Werror -static -I $(INC) $< -o $@

clean:
	rm -rf $(BUILD)
