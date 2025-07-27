use crate::log_debug;

pub(crate) struct IP {
    pub ip: usize,
    bytecode: Vec<u8>,
}

/// TODO: IP creates a clone of the code chunk, which would be nice to avoid
/// Also uses an actual index instead of, for example, an iterator or actual
/// pointer, which should be more performant. Still, this actually runs code,
/// so I can't complain.
impl IP {
    pub fn new(bytecode: &Vec<u8>) -> IP {
        IP {
            ip: 0,
            bytecode: bytecode.clone(),
        }
    }
    
    pub fn is_at_end(&self) -> bool {
        self.ip >= self.bytecode.len()
    }

    pub fn next(&mut self) -> u8 {
        let byte_value = *self.bytecode.get(self.ip).unwrap_or(&0);
        log_debug!("IP advance", ip = format!("{:0x}", self.ip).as_str(), byte = format!("{:0x}", byte_value).as_str());
        match self.bytecode.get(self.ip) {
            Some(v) => { self.ip += 1; *v},
            None => 0
        }
    }
    
    pub fn next_u16(&mut self) -> u16 {
        // Read next 2 bytes as a u16 - written big endian
        let hi = self.next() as u16;
        let lo = self.next() as u16;
        hi << 8 | lo
    }

    pub(crate) fn next_i16(&mut self) -> i16 {
        let hi = self.next() as i16;
        let lo = self.next() as i16;
        hi << 8 | lo
    }
    
    pub fn jump(&mut self, jmp_offset: u16) {
        let jmp_offset = self.ip + jmp_offset as usize;
        log_debug!("IP jump forward", from = format!("{:0x}", self.ip).as_str(), to = format!("{:0x}", jmp_offset).as_str());
        self.ip = jmp_offset;
    }

    pub fn jump_back(&mut self, jmp_offset: u16) {
        let jmp_offset = self.ip - jmp_offset as usize;
        log_debug!("IP jump backward", from = format!("{:0x}", self.ip).as_str(), to = format!("{:0x}", jmp_offset).as_str());
        self.ip = jmp_offset;
    }
    
    pub fn idx(&self, offset: isize) -> usize {
        if offset < 0 {
            if offset.abs() as usize > self.ip { return 0 }
        }
        (self.ip as isize + offset) as usize 
    }
    
}