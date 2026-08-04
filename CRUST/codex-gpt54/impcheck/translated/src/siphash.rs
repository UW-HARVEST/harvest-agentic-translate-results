use std::cell::RefCell;
use std::collections::HashMap;

pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;

thread_local! {
    static STATES: RefCell<HashMap<usize, SipHashState>> = RefCell::new(HashMap::new());
}

struct SipHashState {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    k0: u64,
    k1: u64,
    inlen: u64,
    buflen: usize,
    buf: [u8; 8],
    out: [u8; 16],
}

pub struct SipHash {
    kk: Vec<u8>,
    out: Vec<u8>,
    buf: Vec<u8>,
}

impl SipHash {
    pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
        let nb = nb_bytes as usize;
        STATES.with(|states| {
            let mut states = states.borrow_mut();
            let state = states.get_mut(&self.id()).unwrap();
            let mut datapos = 0usize;
            loop {
                while state.buflen < 8 && datapos < nb {
                    state.buf[state.buflen] = data[datapos];
                    state.buflen += 1;
                    datapos += 1;
                }
                if state.buflen < 8 {
                    break;
                }
                process_next_block_state(state);
                state.buflen = 0;
            }
            state.inlen += nb_bytes;
        });
    }

    pub fn process_next_block(&mut self) {
        STATES.with(|states| {
            let mut states = states.borrow_mut();
            process_next_block_state(states.get_mut(&self.id()).unwrap());
        });
    }

    pub fn process_final_block(&mut self) {
        STATES.with(|states| {
            let mut states = states.borrow_mut();
            process_final_block_state(states.get_mut(&self.id()).unwrap());
        });
    }

    pub fn siphash_pad(&mut self, nb_bytes: u64) {
        let zero = [0u8; 1];
        for _ in 0..nb_bytes {
            self.siphash_update(&zero, 1);
        }
    }

    pub fn siphash_init(key_128bit: &[u8]) -> Self {
        let buf = vec![0u8; 8];
        let out = vec![0u8; 16];
        let kk = key_128bit.to_vec();
        let mut this = Self { kk, out, buf };
        STATES.with(|states| {
            states.borrow_mut().insert(
                this.id(),
                SipHashState {
                    v0: 0,
                    v1: 0,
                    v2: 0,
                    v3: 0,
                    k0: 0,
                    k1: 0,
                    inlen: 0,
                    buflen: 0,
                    buf: [0; 8],
                    out: [0; 16],
                },
            );
        });
        if !this.kk.is_empty() {
            this.siphash_reset();
        }
        this
    }

    pub fn siphash_reset(&mut self) {
        STATES.with(|states| {
            let mut states = states.borrow_mut();
            let state = states.get_mut(&self.id()).unwrap();
            state.v0 = 0x736f_6d65_7073_6575;
            state.v1 = 0x646f_7261_6e64_6f6d;
            state.v2 = 0x6c79_6765_6e65_7261;
            state.v3 = 0x7465_6462_7974_6573;
            state.k0 = u8_to_u64_le(&self.kk[..8]);
            state.k1 = u8_to_u64_le(&self.kk[8..16]);
            state.v3 ^= state.k1;
            state.v2 ^= state.k0;
            state.v1 ^= state.k1;
            state.v0 ^= state.k0;
            state.inlen = 0;
            state.buflen = 0;
            state.buf = [0; 8];
            state.out = [0; 16];
            state.v1 ^= 0xee;
        });
    }

    pub fn siphash_digest(&self) -> Vec<u8> {
        STATES.with(|states| {
            let mut states = states.borrow_mut();
            let state = states.get_mut(&self.id()).unwrap();
            process_final_block_state(state);
            state.out.to_vec()
        })
    }

    pub fn siphash_free(&mut self) {
        STATES.with(|states| {
            states.borrow_mut().remove(&self.id());
        });
        self.kk.clear();
        self.out.clear();
        self.buf.clear();
    }

    fn id(&self) -> usize {
        self.buf.as_ptr() as usize
    }
}

fn process_next_block_state(state: &mut SipHashState) {
    let m = u8_to_u64_le(&state.buf);
    state.v3 ^= m;
    for _ in 0..C_ROUNDS {
        sip_round(state);
    }
    state.v0 ^= m;
}

fn process_final_block_state(state: &mut SipHashState) {
    let left = (state.inlen & 7) as usize;
    let mut b = state.inlen << 56;
    match left {
        7 => {
            b |= (state.buf[6] as u64) << 48;
            b |= (state.buf[5] as u64) << 40;
            b |= (state.buf[4] as u64) << 32;
            b |= (state.buf[3] as u64) << 24;
            b |= (state.buf[2] as u64) << 16;
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        6 => {
            b |= (state.buf[5] as u64) << 40;
            b |= (state.buf[4] as u64) << 32;
            b |= (state.buf[3] as u64) << 24;
            b |= (state.buf[2] as u64) << 16;
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        5 => {
            b |= (state.buf[4] as u64) << 32;
            b |= (state.buf[3] as u64) << 24;
            b |= (state.buf[2] as u64) << 16;
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        4 => {
            b |= (state.buf[3] as u64) << 24;
            b |= (state.buf[2] as u64) << 16;
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        3 => {
            b |= (state.buf[2] as u64) << 16;
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        2 => {
            b |= (state.buf[1] as u64) << 8;
            b |= state.buf[0] as u64;
        }
        1 => {
            b |= state.buf[0] as u64;
        }
        _ => {}
    }

    state.v3 ^= b;
    for _ in 0..C_ROUNDS {
        sip_round(state);
    }
    state.v0 ^= b;
    state.v2 ^= 0xee;
    for _ in 0..D_ROUNDS {
        sip_round(state);
    }
    let first = state.v0 ^ state.v1 ^ state.v2 ^ state.v3;
    u64_to_le_bytes(first, &mut state.out[..8]);

    state.v1 ^= 0xdd;
    for _ in 0..D_ROUNDS {
        sip_round(state);
    }
    let second = state.v0 ^ state.v1 ^ state.v2 ^ state.v3;
    u64_to_le_bytes(second, &mut state.out[8..16]);
}

fn sip_round(state: &mut SipHashState) {
    state.v0 = state.v0.wrapping_add(state.v1);
    state.v1 = rotl(state.v1, 13);
    state.v1 ^= state.v0;
    state.v0 = rotl(state.v0, 32);
    state.v2 = state.v2.wrapping_add(state.v3);
    state.v3 = rotl(state.v3, 16);
    state.v3 ^= state.v2;
    state.v0 = state.v0.wrapping_add(state.v3);
    state.v3 = rotl(state.v3, 21);
    state.v3 ^= state.v0;
    state.v2 = state.v2.wrapping_add(state.v1);
    state.v1 = rotl(state.v1, 17);
    state.v1 ^= state.v2;
    state.v2 = rotl(state.v2, 32);
}

fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

fn u8_to_u64_le(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(buf)
}

fn u64_to_le_bytes(value: u64, out: &mut [u8]) {
    out.copy_from_slice(&value.to_le_bytes());
}
