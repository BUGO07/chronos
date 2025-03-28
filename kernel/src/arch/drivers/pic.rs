use core::cell::SyncUnsafeCell;

use x86_64::instructions::port::Port;

static MASTER: SyncUnsafeCell<Pic> = SyncUnsafeCell::new(Pic::new(0x20));
static SLAVE: SyncUnsafeCell<Pic> = SyncUnsafeCell::new(Pic::new(0xA0));

// SAFETY: must be main thread
pub unsafe fn master<'a>() -> &'a mut Pic {
    &mut *MASTER.get()
}
// SAFETY: must be main thread
pub unsafe fn slave<'a>() -> &'a mut Pic {
    &mut *SLAVE.get()
}

pub unsafe fn init() {
    let master = master();
    let slave = slave();
    // Start initialization
    master.cmd.write(0x11);
    slave.cmd.write(0x11);

    // Set offsets
    master.data.write(0x20);
    slave.data.write(0x28);

    // Set up cascade
    master.data.write(4);
    slave.data.write(2);

    // Set up interrupt mode (1 is 8086/88 mode, 2 is auto EOI)
    master.data.write(1);
    slave.data.write(1);

    // Unmask interrupts
    master.data.write(0);
    slave.data.write(0);

    // Ack remaining interrupts
    master.ack();
    slave.ack();

    // probably already set to PIC, but double-check
    // irq::set_irq_method(irq::IrqMethod::Pic);
}

pub unsafe fn disable() {
    master().data.write(0xFF);
    slave().data.write(0xFF);
}

pub struct Pic {
    cmd: Port<u8>,
    data: Port<u8>,
}

impl Pic {
    pub const fn new(port: u16) -> Pic {
        Pic {
            cmd: Port::new(port),
            data: Port::new(port + 1),
        }
    }

    pub unsafe fn ack(&mut self) {
        self.cmd.write(0x20);
    }

    pub unsafe fn mask_set(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask |= 1 << irq;
        self.data.write(mask);
    }

    pub unsafe fn mask_clear(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask &= !(1 << irq);
        self.data.write(mask);
    }
    /// A bitmap of all currently servicing IRQs. Spurious IRQs will not have this bit set
    pub unsafe fn isr(&mut self) -> u8 {
        self.cmd.write(0x0A);
        self.cmd.read() // note that cmd is read, rather than data
    }
}

pub unsafe fn send_eoi(irq: u8) {
    if irq >= 8 {
        slave().cmd.write(0x20);
    }
    master().cmd.write(0x20);
}
