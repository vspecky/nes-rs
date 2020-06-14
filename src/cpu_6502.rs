// NOTE: In comments, 'X -> Y' reads as "X points to Y"

use crate::cpu_bus;
use cpu_bus::Bus;

// Struct of the NES CPU (MOS 6502)
pub struct MOS6502 {
    a: u8,    // Accumulator
    x: u8,    // X Register
    y: u8,    // Y Register
    s: u8,    // Status Register
    pc: u16,  // Program Counter
    sp: i16,   // Stack Pointer
    clk: u32  // Additional Clock Cycles
}

enum Flags6502 {
    Carry     = 0x01,      // Carry Flag
    Zero      = 0x01 << 1, // Zero Flag
    Interrupt = 0x01 << 2, // Interrupt Disable Flag
    Decimal   = 0x01 << 3, // Decimal Mode Flag
    Break     = 0x01 << 4, // Break Flag
    Overflow  = 0x01 << 6, // Overflow Flag
    Negative  = 0x01 << 7  // Negative Flag
}

// Struct used for addressing mode functions
struct AddrRes {
    byte: Option<u8>,
    cycle: bool
}

impl AddrRes {
    pub fn new(byte: Option<u8>, cycle: bool) -> Self {
        Self { byte, cycle }
    }
}

// Main CPU class
impl MOS6502 {
    pub fn new() -> Self {
        Self {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            s: 0x00,
            pc: 0x0000,
            sp: -1,
            clk: 0
        }
    }

    // Set flag in the status register
    fn set_flag(&mut self, flag: Flags6502, val: bool) {
        if val {
            self.s |= flag as u8;
        } else {
            self.s &= !(flag as u8);
        }
    }

    // Get the state of a flag from the status register
    fn get_flag(&self, flag: Flags6502) -> bool {
        self.s & (flag as u8) > 0
    }

    // Push a byte onto the stack
    fn stack_push(&mut self, byte: u8, bus: &mut Bus) -> Result<(), &str> {
        if self.sp < 0xFF {
            self.sp += 1;
            bus.write(self.sp as u16 + 0x100, byte);
            Ok(())
        } else {
            Err("Stack Overflow Occurred.")
        }
    }

    // Pop a byte from the stack
    fn stack_pop(&mut self, bus: &mut Bus) -> Result<u8, &str> {
        if self.sp > -1 {
            let byte = bus.read(self.sp as u16 + 0x100);
            self.sp -= 1;
            Ok(byte)
        } else {
            Err("Stack Underflow Occured.")
        }
    }

    /*
         _______  ______   ______   _______  _______  _______  _______ _________ _        _______ 
        (  ___  )(  __  \ (  __  \ (  ____ )(  ____ \(  ____ \(  ____ \\__   __/( (    /|(  ____ \
        | (   ) || (  \  )| (  \  )| (    )|| (    \/| (    \/| (    \/   ) (   |  \  ( || (    \/
        | (___) || |   ) || |   ) || (____)|| (__    | (_____ | (_____    | |   |   \ | || |      
        |  ___  || |   | || |   | ||     __)|  __)   (_____  )(_____  )   | |   | (\ \) || | ____ 
        | (   ) || |   ) || |   ) || (\ (   | (            ) |      ) |   | |   | | \   || | \_  )
        | )   ( || (__/  )| (__/  )| ) \ \__| (____/\/\____) |/\____) |___) (___| )  \  || (___) |
        |/     \|(______/ (______/ |/   \__/(_______/\_______)\_______)\_______/|/    )_)(_______)
    */

    // Implied Addressing
    // CPU knows what to do, no args needed.
    fn addr_implied(&mut self, bus: &mut Bus) -> AddrRes {
        AddrRes::new(None, false)
    }

    // Accumulator Addressing
    // Used by operations that act directly on the accumulator.
    fn addr_acc(&mut self, bus: &mut Bus) -> AddrRes {
        AddrRes::new(Some(self.a), false)
    }

    // Immediate Addressing
    // The byte right after the opcode is the argument.
    fn addr_immediate(&mut self, bus: &mut Bus) -> AddrRes {
        let byte = bus.read(self.pc);
        self.pc += 1;
        AddrRes::new(Some(byte), false)
    }

    // Relative Addressing
    // Used by Branch Instructions. The byte after the opcode
    // is the value by which the Program Counter needs to be offset.
    // This is a signed byte so further calculation is required to
    // convert the number from unsigned to signed (Using 2's complement)
    fn addr_relative(&mut self, bus: &mut Bus) -> AddrRes {
        let byte = bus.read(self.pc);
        self.pc += 1;
        AddrRes::new(Some(byte), false)
    }

    // Zero Page Addressing
    // Zero Page Addressing == 0x0000 to 0x00FF
    // The byte after the opcode points to the memory address
    // in the aforementioned range which has the actual arg
    // i.e byte_after_opcode -> argument.
    fn addr_zero_pg(&mut self, bus: &mut Bus) -> AddrRes {
        let addr = bus.read(self.pc);
        self.pc += 1;
        let byte = bus.read(addr as u16);
        AddrRes::new(Some(byte), false)
    }

    // Indirect Addressing
    // the next two bytes after the opcode form an address.
    // The address     -> low byte of the arg.
    // The address + 1 -> high byte of the arg.
    // The high and low byte form a 16-bit argument.
    // I can use the [AddrRes; 2] return type to return the high
    // and low bytes since only the JMP opcode uses this mode.
    fn addr_indirect(&mut self, bus: &mut Bus) -> [AddrRes; 2] {
        let addr_hi = bus.read(self.pc) as u16;
        self.pc += 1;
        let addr_lo = bus.read(self.pc) as u16;
        self.pc += 1;

        let addr = (addr_hi << 8) | addr_lo;

        let byte_lo = bus.read(addr);
        let byte_hi = bus.read(addr + 0x0001);

        [AddrRes::new(Some(byte_hi), false), AddrRes::new(Some(byte_lo), false)]
    }
}
