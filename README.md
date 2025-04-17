# ChronOS

A basic x86_64 os kernel made in rust.

Contributors are welcome.

:warning: This project is a mess, and will probably be so for a long time. 

## [LICENSE](LICENSE)

## Features

- Basic Logger
- Serial
- Interrupts
- Memory Management
- PS/2 Keyboard
- PIT + TSC + KVM + LAPIC
- RTC
- Async Task Executor
- Basic Shell (i know its not supposed to be there, its temporary)

## TODO:

- ACPI
- Scheduler
- VFS
- USB

## Known Bugs

- Sometimes address overflow in vmm on real hardware.

## Building And Running

Make sure you have the following installed:
* Rust
* Clang
* Make
* QEMU x86-64
* Xorriso

Follow these steps to build and run the os
1. Clone this repo with:\
``git clone --recursive --depth=1 https://github.com/BUGO07/chronos``

2. Go to the root directory of cloned repo and run:\
``make run`` For running debug mode\
``RUST_PROFILE="release" make run`` For running release mode\
``make test`` For running tests