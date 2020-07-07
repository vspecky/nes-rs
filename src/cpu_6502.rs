// NOTE: In comments, 'X -> Y' reads as "X points to Y"
// Important: Maybe need to subtract 1 from the PC in branch instructions
// since PC is incremented when the relative address byte is read.

use crate::cpu_bus;
use cpu_bus::Bus;

// Struct of the NES CPU (MOS 6502)
pub struct MOS6502 {
    a: u8,         // Accumulator
    x: u8,         // X Register
    y: u8,         // Y Register
    s: u8,         // Status Register
    pc: u16,       // Program Counter
    sp: i16,       // Stack Pointer
    clk: u32,      // Additional Clock Cycles
    acc_addr: bool // Set when Accumulator addressing occurs
}

enum Flags {
    Carry     = 0x01,      // Carry Flag
    Zero      = 0x01 << 1, // Zero Flag
    Interrupt = 0x01 << 2, // Interrupt Disable Flag
    Decimal   = 0x01 << 3, // Decimal Mode Flag (Unused in NES)
    Break     = 0x01 << 4, // Break Flag
    Overflow  = 0x01 << 6, // Overflow Flag
    Negative  = 0x01 << 7  // Negative Flag
}

// Struct used for returning the result from
// addressing mode functions
struct AddrRes {
    addr: u16,
    cycle: bool,
}

impl AddrRes {
    pub fn new(addr: u16, cycle: bool) -> Self {
        Self { addr, cycle }
    }
}

type AddrMode = fn(&mut MOS6502, &mut Bus) -> AddrRes;

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
            clk: 0,
            acc_addr: false,
        }
    }

    // Set flag in the status register
    fn set_flag(&mut self, flag: Flags, val: bool) {
        if val {
            self.s |= flag as u8;
        } else {
            self.s &= !(flag as u8);
        }
    }

    // Get the state of a flag from the status register
    fn get_flag(&self, flag: Flags) -> bool {
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

    fn read_opcode(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read(self.pc);
        self.pc += 1;
        opcode
    }

    fn tick(&mut self) {

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
    fn addr_implied(&mut self, bus: &mut Bus) {
        unimplemented!();
    }

    // Accumulator Addressing
    // Used by operations that act directly on the accumulator.
    fn addr_acc(&mut self, bus: &mut Bus) -> AddrRes {
        self.acc_addr = true;
        AddrRes::new(self.a as u16, false)
    }

    // Immediate Addressing
    // The byte right after the opcode is the argument.
    fn addr_immediate(&mut self, bus: &mut Bus) -> AddrRes {
        let byte = self.pc;
        self.pc += 1;
        AddrRes::new(byte, false)
    }

    // Relative Addressing
    // Used by Branch Instructions. The byte after the opcode
    // is the value by which the Program Counter needs to be offset.
    // This is a signed byte so further calculation is required to
    // convert the number from unsigned to signed (Using 2's complement)
    fn addr_relative(&mut self, bus: &mut Bus) -> AddrRes {
        let mut byte = bus.read(self.pc) as u16;
        self.pc += 1;

        if byte & 0x80 > 1 {
            byte |= 0xFF00;
        }

        AddrRes::new(byte, (self.pc + byte) & 0xFF00 > self.pc & 0xFF00)
    }

    // Zero Page Addressing
    // Zero Page Addressing == 0x0000 to 0x00FF
    // The byte after the opcode points to the memory address
    // in the aforementioned range which has the actual arg
    // i.e byte_after_opcode -> argument.
    fn addr_zero_pg(&mut self, bus: &mut Bus) -> AddrRes {
        let addr = bus.read(self.pc) as u16;
        self.pc += 1;
        AddrRes::new(addr, false)
    }

    // Absolute Addressing
    // The two bytes after the opcode form the 16-bit argument
    // NES == little endian so first byte is low byte
    fn addr_absolute(&mut self, bus: &mut Bus) -> AddrRes {
        let byte_lo = bus.read(self.pc) as u16;
        self.pc += 1;
        let byte_hi = bus.read(self.pc) as u16;
        self.pc += 1;

        let addr = (byte_hi << 8) | byte_lo;

        AddrRes::new(addr, false)
    }

    // Indirect Addressing
    // the next two bytes after the opcode form an address.
    // The address     -> low byte of the arg.
    // The address + 1 -> high byte of the arg.
    // The high and low byte form a 16-bit argument.
    // This is used exclusively by the JMP opcode.
    fn addr_indirect(&mut self, bus: &mut Bus) -> AddrRes {
        let addr_hi = bus.read(self.pc) as u16;
        self.pc += 1;
        let addr_lo = bus.read(self.pc) as u16;
        self.pc += 1;

        let addr = (addr_hi << 8) | addr_lo;

        let addr_2 = if addr_lo == 0x00FF {
            (((bus.read(addr & 0xFF00) as u16) << 8) | bus.read(addr) as u16)
        } else {
            (((bus.read(addr + 1) as u16) << 8) | bus.read(addr) as u16)
        };

        AddrRes::new(addr_2, false)
    }

    // X-Indexed Zero Page Addressing
    // Basically ZPA but with X register contents added
    fn addr_zero_pg_x(&mut self, bus: &mut Bus) -> AddrRes {
        let addr = bus.read(self.pc).wrapping_add(self.x) as u16;
        self.pc += 1;
        AddrRes::new(addr, false)
    }

    // Y-Indexed Zero Page Addressing
    // Basically ZPA but with Y register contents added
    fn addr_zero_pg_y(&mut self, bus: &mut Bus) -> AddrRes {
        let addr = bus.read(self.pc).wrapping_add(self.y) as u16;
        self.pc += 1;
        AddrRes::new(addr, false)
    }

    // X-Indexed Absolute Address
    // Basically Absolute Addressing offset with the X reg value
    fn addr_absolute_x(&mut self, bus: &mut Bus) -> AddrRes {
        let byte_lo = bus.read(self.pc) as u16;
        self.pc += 1;
        let byte_hi = bus.read(self.pc) as u16;
        self.pc += 1;

        let mut addr = (byte_hi << 8) | byte_lo;
        addr = addr.wrapping_add(self.x as u16);

        let cycle = addr & 0xFF00 != byte_hi << 8;

        AddrRes::new(addr, cycle)
    }

    // Y-Indexed Absolute Address
    // Basically Absolute Addressing offset with the Y reg value
    fn addr_absolute_y(&mut self, bus: &mut Bus) -> AddrRes {
        let byte_lo = bus.read(self.pc) as u16;
        self.pc += 1;
        let byte_hi = bus.read(self.pc) as u16;
        self.pc += 1;

        let mut addr = (byte_hi << 8) | byte_lo;
        addr = addr.wrapping_add(self.y as u16);

        let cycle = addr & 0xFF00 != byte_hi << 8;

        AddrRes::new(addr, cycle)
    }

    // Indexed Indirect Addressing
    // b <- Byte after the opcode + X
    // address_low_byte  = mem[b]
    // address_high_byte = mem[b + 1]
    // arg = mem[address]
    fn addr_idx_indirect(&mut self, bus: &mut Bus) -> AddrRes {
        let byte = bus.read(self.pc).wrapping_add(self.x) as u16;
        self.pc += 1;

        let byte_lo = bus.read(byte) as u16;
        let byte_hi = bus.read(byte + 1) as u16;

        let addr = (byte_hi << 8) | byte_lo;

        AddrRes::new(addr, false)
    }

    // Indirect Indexed Addressing
    // b <- Byte after the opcode
    // addr_low = mem[b]
    // addr_hi  = mem[b + 1]
    // arg = mem[addr + Y]
    fn addr_indirect_idx(&mut self, bus: &mut Bus) -> AddrRes {
        let byte = bus.read(self.pc) as u16;
        self.pc += 1;

        let byte_lo = bus.read(byte) as u16;
        let byte_hi = bus.read(byte + 1) as u16;

        let addr = ((byte_hi << 8) | byte_lo) + self.y as u16;
        
        let cycle = addr & 0xFF00 != byte_lo << 8;

        AddrRes::new(addr, false)
    }

    /*
         _______  _______  _______  _______  ______   _______  _______ 
        (  ___  )(  ____ )(  ____ \(  ___  )(  __  \ (  ____ \(  ____ \
        | (   ) || (    )|| (    \/| (   ) || (  \  )| (    \/| (    \/
        | |   | || (____)|| |      | |   | || |   ) || (__    | (_____ 
        | |   | ||  _____)| |      | |   | || |   | ||  __)   (_____  )
        | |   | || (      | |      | |   | || |   ) || (            ) |
        | (___) || )      | (____/\| (___) || (__/  )| (____/\/\____) |
        (_______)|/       (_______/(_______)(______/ (_______/\_______)
    */


    /*
        ADC - Add with Carry
        A,Z,C,N = A+M+C

        This instruction adds the contents of a memory location to the accumulator together with the carry bit. 
        If overflow occurs the carry bit is set, this enables multiple byte addition to be performed.
    */
    fn opcode_adc(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);
        let byte = bus.read(addr_res.addr);

        let res = self.a as u16 + byte as u16 + self.get_flag(Flags::Carry) as u16;

        self.set_flag(Flags::Carry, res > 255);
        self.set_flag(Flags::Zero, res & 0x00FF == 0);

        let overflow = (!(self.a as u16 ^ byte as u16) & (self.a as u16 ^ res)) & 0x80 > 1;

        self.set_flag(Flags::Overflow, overflow);
        self.set_flag(Flags::Negative, res & 0x80 > 1);

        self.a = (res & 0x00FF) as u8;

        // Additional clock cycles do not depend on opcode execution
        if addr_res.cycle { 1 } else { 0 }
    }

    /*
        AND - Logical AND
        A,Z,N = A&M

        A logical AND is performed, bit by bit, on the accumulator contents using the contents of a byte of memory.        
    */
    fn opcode_and(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &mut Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);
        let byte = bus.read(addr_res.addr);

        // Perform bitwise AND and reassign value
        self.a = self.a & byte;

        // Set Zero Flag
        self.set_flag(Flags::Zero, self.a == 0);
        // Set Negative Flag
        self.set_flag(Flags::Negative, self.a & 0x80 > 0);

        // Additional clock cycles do not depend on opcode execution
        if addr_res.cycle { 1 } else { 0 }
    }

    /*
        ASL - Arithmetic Shift Left
        A,Z,C,N = M*2 or M,Z,C,N = M*2

        This operation shifts all the bits of the accumulator or memory contents one bit left. 
        Bit 0 is set to 0 and bit 7 is placed in the carry flag.
        The effect of this operation is to multiply the memory contents by 2 (ignoring 2's complement considerations),
        setting the carry if the result will not fit in 8 bits.
    */
    fn opcode_asl(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &mut Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);
        // If accumulator addressing was performed then the CPU is operating on the
        // Accumulator. Set var 'byte' accordingly
        let mut byte = if self.acc_addr {
            addr_res.addr as u8
        } else {
            bus.read(addr_res.addr)
        };

        // MSB is moved into Carry bit
        self.set_flag(Flags::Carry, byte & 0x80 > 0);

        byte = byte << 1;
        
        // Set Zero and Negative flags accordingly
        self.set_flag(Flags::Zero, byte == 0);
        self.set_flag(Flags::Negative, byte & 0x80 > 0);

        // If Accumulator addressing was performed, reassign the new byte to the
        // Accumulator else the memeory address
        if self.acc_addr {
            self.a = byte;
        } else {
            bus.write(addr_res.addr, byte);
        }

        // Unset the Accumulator addressing switch
        self.acc_addr = false;

        // Opcode execution never requires additional clock cycles
        0
    }

    /*
        BCC - Branch if Carry Clear
        If the carry flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
    */
    fn opcode_bcc(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &mut Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);
        
        // If the Carry flag is set, return with no additional clock cycles
        if self.get_flag(Flags::Carry) {
            return 0;
        }

        let old_pc = self.pc;

        self.pc += addr_res.addr;

        let mut additional_cycles = 1;

        // The number of additional cycles is 1 if branch succeeds
        // and an additional +1 if branch occurs to a new page
        if (self.pc & 0xFF00) != (old_pc & 0xFF00) {
            additional_cycles += 1;
        }

        additional_cycles
    }

    /*
        BCS - Branch if Carry Set
        If the carry flag is set then add the relative displacement to the program counter to cause a branch to a new location.
    */
    fn opcode_bcs(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &mut Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);
        
        // If the Carry flag is unset, return with no additional clock cycles
        if !self.get_flag(Flags::Carry) {
            return 0;
        }

        let old_pc = self.pc;

        self.pc += addr_res.addr;

        let mut additional_cycles = 1;

        // The number of additional cycles is 1 if branch succeeds
        // and an additional +1 if branch occurs to a new page
        if (self.pc & 0xFF00) != (old_pc & 0xFF00) {
            additional_cycles += 1;
        }

        additional_cycles
    }

    /*
        BEQ - Branch if Equal
        If the zero flag is set then add the relative displacement to the 
        program counter to cause a branch to a new location.
    */
    fn opcode_beq(&mut self, bus: &mut Bus, addr_mode: fn(&mut Self, &mut Bus) -> AddrRes) -> u8 {
        let addr_res = addr_mode(self, bus);

        if !self.get_flag(Flags::Zero) {
            return 0;
        }

        let old_pc = self.pc;

        self.pc += addr_res.addr;

        let mut additional_cycles = if (self.pc & 0xFF00) != (old_pc & 0xFF00) {
            2
        } else {
            1
        };

        additional_cycles
    }

    /* BIT - Bit Test
     * A & M, N = M7, V = M6
     * Used to test if one or more bits are set in a memory location. A is
     * ANDed with the byte in memory to set or clear the Zero flag. The
     * result is not saved. Bits 7 and 6 of the memory byte are copied
     * into the negative and overflow flags respectively.
     */
    fn opcode_bit(&mut self, bus: &mut Bus, addr_mode: AddrMode) -> u8 {
        let addr_res = addr_mode(self, bus);
        let byte = bus.read(addr_res.addr);

        // Set Zero flag to MEM & A
        self.set_flag(Flags::Zero, self.a & byte == 0);
        // Set Negative flag to the last bit of memory
        self.set_flag(Flags::Negative, byte & 0x80 == 0);
        // Set overflow flag to bit 6 of memory
        self.set_flag(Flags::Overflow, byte & 0x40 == 0);

        0
    }

    /* BMI - Branch if Minus
     * If the negative flag is set then add the relative displacement
     * to the program counter to cause a branch to a new location
     */
    fn opcode_bmi(&mut self, bus: &mut Bus, addr_mode: AddrMode) -> u8 {
        let byte = addr_mode(self, bus);

        if self.get_flag(Flags::Negative) {
            self.pc += byte.addr;

            if byte.cycle {2} else {1}
        } else {
            0
        }
    }

    /* BNE - Branch if Not Equal
     * If the zero flag is clear then add the relative displacement to
     * the program counter to cause a branch to a new location
     */
    fn opcode_bne(&mut self, bus: &mut Bus, addr_mode: AddrMode) -> u8 {
        let addr_res = addr_mode(self, bus);

        if !self.get_flag(Flags::Zero) {
            self.pc += addr_res.addr;

            if addr_res.cycle {2} else {1}
        } else {
            0
        }
    }
}
