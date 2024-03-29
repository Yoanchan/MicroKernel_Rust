use spin::Mutex;
use x86_64::instructions::port::Port;

pub static MAIN: Mutex<Pic> = Mutex::new(Pic::new(0x20));
pub static WORKER: Mutex<Pic> = Mutex::new(Pic::new(0xA0));

pub fn init() {
    let mut wait_port: Port<u8> = Port::new(0x80);

    let mut write_then_wait = |port: &mut Port<u8>, data: u8| unsafe {
        port.write(data);
        wait_port.write(0);
    };

    let mut main = MAIN.lock();
    let mut worker = WORKER.lock();

    let (saved_mask1, saved_mask2) = unsafe { (main.data.read(), worker.data.read()) };

    let init_value: u8 = (ICW1::INIT as u8) + (ICW1::ICW4_NOT_NEEDED as u8);
    write_then_wait(&mut main.cmd, init_value);
    write_then_wait(&mut worker.cmd, init_value);

    write_then_wait(&mut main.data, 0x20);
    write_then_wait(&mut worker.data, 0x28);

    write_then_wait(&mut main.data, 0x4);
    write_then_wait(&mut worker.data, 0x2);

    write_then_wait(&mut main.data, ICW4::MODE_8086 as u8);
    write_then_wait(&mut worker.data, ICW4::MODE_8086 as u8);

    write_then_wait(&mut main.data, saved_mask1);
    write_then_wait(&mut worker.data, saved_mask2);

    info!("PIC Driver Initialized");
}

pub struct Pic {
    pub cmd: Port<u8>,
    pub data: Port<u8>,
}

impl Pic {
    pub const fn new(port: u16) -> Pic {
        Pic {
            cmd: Port::new(port),
            data: Port::new(port + 1),
        }
    }

    pub fn ack(&mut self) {
        unsafe { self.cmd.write(0x20) }
    }

    pub fn mask_set(&mut self, irq: u8) {
        assert!(irq < 8);

        unsafe {
            let mut mask = self.data.read();
            mask |= 1 << irq;
            self.data.write(mask);
        }
    }

    pub fn mask_clear(&mut self, irq: u8) {
        assert!(irq < 8);

        unsafe {
            let mut mask = self.data.read();
            mask &= !(1 << irq);
            self.data.write(mask);
        }
    }
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub enum ICW1 {
    ICW4_NOT_NEEDED = 0x01,
    SINGLE_CASCADE_MODE = 0x02,
    INTERVAL4 = 0x04,
    LEVEL_TRIGGERED_MODE = 0x08,
    INIT = 0x10,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub enum ICW4 {
    MODE_8086 = 0x01,
    AUTO_EOI = 0x02,
    BUF_SLAVE = 0x08,
    BUF_MASTER = 0x0C,
    SFNM = 0x10,
}
