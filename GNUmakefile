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
	qemu-system-x86_64 \
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

.PHONY: run-aarch64
run-aarch64: ovmf/ovmf-code-aarch64.fd ovmf/ovmf-vars-aarch64.fd $(IMAGE_NAME).iso
	qemu-system-aarch64 \
		-M virt \
		-cpu cortex-a72 \
		-device ramfb \
		-device qemu-xhci \
		-device usb-kbd \
		-device usb-mouse \
		-serial stdio \
		-drive if=pflash,unit=0,format=raw,file=ovmf/ovmf-code-aarch64.fd,readonly=on \
		-drive if=pflash,unit=1,format=raw,file=ovmf/ovmf-vars-aarch64.fd \
		-cdrom $(IMAGE_NAME).iso \
		$(QEMUFLAGS)

.PHONY: test
test: ovmf/OVMF_x86_64.fd chronos-test.iso
	qemu-system-x86_64 \
		-M q35 \
		-cpu host \
		-display none \
		-serial stdio \
		-bios ovmf/OVMF_x86_64.fd \
		-boot order=d,menu=on,splash-time=0 \
		-enable-kvm \
		-cdrom chronos-test.iso \
		$(QEMUFLAGS)

.PHONY: uacpi-test
uacpi-test: ovmf/OVMF_x86_64.fd chronos-uacpi-test.iso
	qemu-system-x86_64 \
		-M q35 \
		-cpu host \
		-display none \
		-serial stdio \
		-bios ovmf/OVMF_x86_64.fd \
		-enable-kvm \
		-cdrom chronos-uacpi-test.iso \
		$(QEMUFLAGS) | grep 'avg'

.PHONY: run-bios
run-bios: $(IMAGE_NAME).iso
	qemu-system-x86_64 \
		-M q35 \
		-serial stdio \
		-cdrom $(IMAGE_NAME).iso \
		-boot d \
		-enable-kvm \
		$(QEMUFLAGS)

ovmf/OVMF_x86_64.fd:
	mkdir -p ovmf
	curl -Lo $@ https://retrage.github.io/edk2-nightly/bin/RELEASEX64_OVMF.fd

ovmf/ovmf-code-aarch64.fd:
	mkdir -p ovmf
	curl -Lo $@ https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-code-aarch64.fd
	case "aarch64" in \
		aarch64) dd if=/dev/zero of=$@ bs=1 count=0 seek=67108864 2>/dev/null;; \
		loongarch64) dd if=/dev/zero of=$@ bs=1 count=0 seek=5242880 2>/dev/null;; \
		riscv64) dd if=/dev/zero of=$@ bs=1 count=0 seek=33554432 2>/dev/null;; \
	esac

ovmf/ovmf-vars-aarch64.fd:
	mkdir -p ovmf
	curl -Lo $@ https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-vars-aarch64.fd
	case "aarch64" in \
		aarch64) dd if=/dev/zero of=$@ bs=1 count=0 seek=67108864 2>/dev/null;; \
		loongarch64) dd if=/dev/zero of=$@ bs=1 count=0 seek=5242880 2>/dev/null;; \
		riscv64) dd if=/dev/zero of=$@ bs=1 count=0 seek=33554432 2>/dev/null;; \
	esac

limine/limine:
	rm -rf limine
	git clone https://github.com/limine-bootloader/limine.git --branch=v9.x-binary --depth=1
	$(MAKE) -C limine

.PHONY: kernel-$(KARCH)
kernel-$(KARCH):
	$(MAKE) -C kernel $(KARCH)

.PHONY: kernel-uacpi-test
kernel-uacpi-test:
	$(MAKE) -C kernel uacpi-test

.PHONY: kernel-test
kernel-test:
	$(MAKE) -C kernel test

chronos-x86_64.iso: limine/limine kernel-x86_64
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/chronos-x86_64 iso_root/boot/chronos
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

chronos-aarch64.iso: limine/limine kernel-aarch64
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/chronos-aarch64 iso_root/boot/chronos
	mkdir -p iso_root/boot/limine
	cp -v limine.conf iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
	cp -v limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTAA64.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
	rm -rf iso_root

chronos-uacpi-test.iso: limine/limine kernel-uacpi-test
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/chronos-uacpi-test iso_root/boot/chronos
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
		iso_root -o chronos-uacpi-test.iso
	./limine/limine bios-install chronos-uacpi-test.iso
	rm -rf iso_root

chronos-test.iso: limine/limine kernel-test
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/chronos-test iso_root/boot/chronos
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
		iso_root -o chronos-test.iso
	./limine/limine bios-install chronos-test.iso
	rm -rf iso_root

.PHONY: clean
clean:
	$(MAKE) -C kernel clean
	rm -rf iso_root *.iso

.PHONY: distclean
distclean: clean
	$(MAKE) -C kernel distclean
	rm -rf limine ovmf
