# ChronOS

A basic hobby os held together by duct tape, written in rust.

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

## TODO:

- NVMe
- USB
- Port libc
- Support other architectures

## Known Bugs/Issues

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
``make run``

Environment variables:\
RUST_PROFILE - changes the rust build profile - `dev`/`release`/`smol` - default=`dev`\