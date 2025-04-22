
pub(crate) struct IP {
    pub ip: usize,
    code: Vec<u8>,
    debug_mode: bool
}

/// TODO: IP creates a clone of the code chunk, which would be nice to avoid
/// Also uses an actual index instead of, for example, an iterator or actual
/// pointer, which should be more performant. Still, this actually runs code,
/// so I can't complain.
impl IP {
    pub fn new(code: &Vec<u8>, debug_mode: bool) -> IP {
        IP {
            ip: 0,
            code: code.clone(),
            debug_mode
        }
    }

    pub fn next(&mut self) -> u8 {
        self.debug(&format!("IP (NEXT) -> {:0x}", self.ip));
        match self.code.get(self.ip) {
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
    
    pub fn idx(&self, offset: isize) -> usize {
        if offset < 0 {
            if offset.abs() as usize > self.ip { return 0 }
        }
        (self.ip as isize + offset) as usize 
    }
    
    fn debug(&self, msg: &str) {
        if self.debug_mode { println!("{}", msg); }
    }
}