use spike_sys::Spike;
use std::collections::VecDeque;

pub struct Deque {
    n: VecDeque<u8>,
}

impl Deque {
    pub fn new(data: [u8; 512]) -> Self {
        Self {
            n: VecDeque::from(data),
        }
    }

    pub fn u8(&mut self) -> u8 {
        let r = self.n.pop_front().unwrap();
        self.n.push_back(r);
        r
    }

    pub fn u32(&mut self) -> u32 {
        let mut r = [0u8; 4];
        r.fill_with(|| self.u8());
        u32::from_le_bytes(r)
    }

    pub fn u64(&mut self) -> u64 {
        let mut r = [0u8; 8];
        r.fill_with(|| self.u8());
        u64::from_le_bytes(r)
    }
}

pub fn assert_registers_eq(spike: &Spike, int_regs: &[u64], asm_regs: &[u64]) {
    for i in 0..32u64 {
        let spike_reg = spike.get_reg(i).unwrap();
        assert_eq!(spike_reg, int_regs[i as usize]);
        assert_eq!(spike_reg, asm_regs[i as usize]);
    }
}
