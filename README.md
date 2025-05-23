# ChronOS

A basic multi-arch hobby os held together by duct tape, written in rust.

Contributors are welcome.

:warning: This project is a mess, and will probably be so for a long time. 

## [LICENSE](LICENSE)

## Features

### x86_64
- Terminal Emulator
- Serial IO
- Interrupts
- PIT/TSC/KVM/HPET/LAPIC Timers
- Real Time Clock (RTC)
- Memory Management
- PS/2 Keyboard and Mouse
- Preemptive Scheduler (single core for now)
- ACPI
- Basic PCI
- Basic Shell
- Basic RAM FS

### aarch64 (arm64)

- Terminal Emulator
- Serial IO
- Generic Timer
- Memory Allocator (no pagemap yet)
- Cooperative Scheduler
- Basic Shell (input via serial)
- Basic RAM FS

## TODO:

- Proper build system insteaad of just `make`
- Utilize all cpu cores
- NVMe
- USB
- ELF loading
- Interrupts, MMU and preemptive scheduler on aarch64

## Known Bugs/Issues

- Address overflow in vmm on my laptop in debug builds.
- Crash on my laptop when setting the pagetable (this is fixed by hardsetting the pagesize to LARGE).
- No way to wake up from S3 sleep yet.
- Using the keyboard before `up and running` makes keyboard and mouse not work

## Building And Running

Make sure you have the following installed:
* `rust`
* `clang`
* `make`
* `qemu`
* `xorriso`

Follow these steps to build and run the OS:
1. Clone this repo with:\
``git clone --recursive --depth=1 https://github.com/BUGO07/chronos``

2. Go to the root directory of cloned repo and run:\
``make run`` For running\
``make test`` For tests\
``make uacpi-test`` For measuring [uACPI](https://github.com/uACPI/uACPI) score

Environment variables:\
RUST_PROFILE - changes the rust build profile - `dev`/`release`/`smol` - default=`dev`\
KARCH - changes the target architecture - `x86_64`/`aarch64` - default=`x86_64`