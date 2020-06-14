use crate::cpu_6502;
use cpu_6502::MOS6502;

pub struct Bus {
    cpu: MOS6502,
    ram: [u8; 0xFFFF]
}

impl Bus {
    pub fn new() -> Self {
        let out = Self {
            cpu: MOS6502::new(),
            ram: [0; 0xFFFF]
        };
        
        out
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write(&mut self, addr: u16, byte: u8) {
        self.ram[addr as usize] = byte;
    }
}
