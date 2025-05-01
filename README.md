# ChronOS

A basic multi-arch hobby os held together by duct tape, written in rust.

Contributors are welcome.

:warning: This project is a mess, and will probably be so for a long time. 

## [LICENSE](LICENSE)

## Features (most of these are x86_64 only)

- Basic Logger
- Serial
- Interrupts
- PIT + TSC + KVM + HPET + LAPIC
- RTC
- Memory Management
- PS/2 Keyboard
- PS/2 Mouse
- Cooperative Scheduler
- ACPI
- PCI
- Basic Shell (i know its not supposed to be there, its temporary)

## TODO:

- Preemptive Scheduler
- VFS
- USB
- Userspace
- Expand aarch64 support

## Known Bugs/Issues

- Sometimes address overflow in vmm on real hardware in debug builds.
- Double fault on my laptop when booting for unknown reason.
- No way to wake up from sleep yet.
- Opt-level 2 and 3 cause a bootloop.
- Using the keyboard before `up and running` makes keyboard and mouse not work

## Building And Running

Make sure you have the following installed:
* Rust
* Clang
* Make
* QEMU x86_64 | aarch64
* Xorriso

You will also need `aarch64-linux-gnu-gcc` to build for cross-compile c libraries

Follow these steps to build and run the os
1. Clone this repo with:\
``git clone --recursive --depth=1 https://github.com/BUGO07/chronos``

2. Go to the root directory of cloned repo and run:\
``make run`` For running debug mode\
``RUST_PROFILE="release" make run`` For running release mode\
``RUST_PROFILE="smol" make run`` For optimizing for size\
``make test`` For running tests

Change the KARCH environment variable to run `x86_64` (default) or `aarch64` (arm64) architectures (no tests for it yet)