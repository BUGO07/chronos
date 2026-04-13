# Nuke built-in rules and variables.
MAKEFLAGS += -rR
.SUFFIXES:

# Convenience macro to reliably declare user overridable variables.
override USER_VARIABLE = $(if $(filter $(origin $(1)),default undefined),$(eval override $(1) := $(2)))

# Default user QEMU flags. These are appended to the QEMU command calls.
$(call USER_VARIABLE,QEMUFLAGS,-smp 4 -m 2G -d int -D int.txt)
$(call USER_VARIABLE,KARCH,x86_64)

override IMAGE_NAME := chronos-$(KARCH)

.PHONY: all
all: $(IMAGE_NAME).iso

.PHONY: run
run: run-$(KARCH)

.PHONY: run-x86_64
run-x86_64: ovmf/OVMF_x86_64.fd $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M q35 \
		-cpu host \
		-serial stdio \
		-device secondary-vga \
		-drive file=kernel_disk.qcow2,format=qcow2,if=none,id=nvme0 \
		-device nvme,drive=nvme0,serial=deadbeef \
		-bios ovmf/OVMF_x86_64.fd \
		-boot order=d,menu=on,splash-time=0 \
		-enable-kvm \
		-cdrom $(IMAGE_NAME).iso \
		$(QEMUFLAGS)

.PHONY: run-bios
run-bios: $(IMAGE_NAME).iso
	qemu-system-$(KARCH) \
		-M q35 \
		-serial stdio \
		-cdrom $(IMAGE_NAME).iso \
		-boot d \
		-enable-kvm \
		$(QEMUFLAGS)

initramfs.tar: initramfs/src
	$(MAKE) testelf initramfs/src
	rm -rf initramfs/target
	tar --format=ustar -cf $@ -C initramfs ./

ovmf/OVMF_x86_64.fd:
	mkdir -p ovmf
	curl -Lo $@ https://retrage.github.io/edk2-nightly/bin/RELEASEX64_OVMF.fd

limine/limine:
	rm -rf limine
	git clone https://github.com/limine-bootloader/limine.git --branch=v11.x-binary --depth=1
	$(MAKE) -C limine

.PHONY: kernel
kernel:
	$(MAKE) -C kernel

$(IMAGE_NAME).iso: limine/limine initramfs.tar kernel
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/$(IMAGE_NAME) iso_root/boot/chronos
	cp -v initramfs.tar iso_root/boot
	mkdir -p iso_root/boot/limine
	cp -v limine.conf iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
	cp -v limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTX64.EFI iso_root/EFI/BOOT/
	cp -v limine/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
	./limine/limine bios-install $(IMAGE_NAME).iso
	rm -rf iso_root

# Allow: make testelf path/to/file.rs
# Collects everything after `testelf` as the source path and suppresses make's
# attempt to build those extra words as targets.
ifeq (testelf,$(firstword $(MAKECMDGOALS)))
  TESTELF_SRC := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(TESTELF_SRC):;@:)
endif

# Derive project dir (parent of src folder) and binary name
TESTELF_PROJ   := $(abspath $(TESTELF_SRC)/..)
TESTELF_BIN    := $(notdir $(TESTELF_PROJ))
TESTELF_TARGET := x86_64-unknown-none

.PHONY: testelf
testelf:
	@if [ -z "$(TESTELF_SRC)" ]; then \
		echo "usage: make testelf <path/to/srcfolder>"; exit 1; \
	fi
	mkdir -p $(TESTELF_PROJ)/bin
	RUSTFLAGS="-C relocation-model=static -C link-arg=-Ttext=0x400000" \
		cargo build --release --target $(TESTELF_TARGET) \
		--manifest-path $(TESTELF_PROJ)/Cargo.toml
	cp $(TESTELF_PROJ)/target/$(TESTELF_TARGET)/release/$(TESTELF_BIN) \
		$(TESTELF_PROJ)/bin/$(TESTELF_BIN).elf

.PHONY: clean
clean:
	$(MAKE) -C kernel clean
	rm -rf iso_root *.iso

.PHONY: distclean
distclean: clean
	$(MAKE) -C kernel distclean
	rm -rf limine ovmf
