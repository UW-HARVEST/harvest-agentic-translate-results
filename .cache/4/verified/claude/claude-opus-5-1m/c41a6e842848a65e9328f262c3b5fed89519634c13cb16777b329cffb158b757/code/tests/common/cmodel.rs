//! An **independent** model of `c_src/src/lib.c`'s `cp_inflate`, transcribed
//! line by line from the C source.
//!
//! It serves two purposes:
//!
//! 1. **Cross-check.** For well-defined inputs its outcome (return value, error
//!    message, failing `assert` line, output bytes) must equal what both shared
//!    libraries produce. A row that passes only because the C and the Rust `.so`
//!    are *equally* wrong about the C source would be caught here.
//!
//! 2. **Mechanical UB classification.** `lib.c` contains a handful of operations
//!    that are undefined in C, and whose observable effect therefore depends on
//!    stack layout / stack garbage rather than on the input:
//!
//!    * `cp_dynamic`: `lens[n]` written for `n >= 288 + 32` (a `16`/`17`/`18`
//!      run that overshoots `nlit + ndst` — up to 137 bytes past the array),
//!    * `cp_dynamic`: `lens[-1]` read when the very first code-length symbol
//!      is `16` (uninitialised stack),
//!    * `cp_build`: `counts[lens[n]]++` with `lens[n] >= 16` (past `int counts[16]`),
//!    * `cp_block`: `cp_len_extra_bits[symbol - 257]` / `cp_dist_*[dsym]` read
//!      past the end of the tables when a garbage Huffman key decodes to a
//!      symbol outside `257..=287` / `0..=31`,
//!    * `cp_stored`: `memcpy(s->out, p, LEN)` with no output bounds check.
//!
//!    Such inputs have no defined behaviour for the Rust port to reproduce, so
//!    the differential tests report them separately instead of failing.
//!
//! The state arrays are modelled as one contiguous byte blob with **exactly the
//! `cp_state_t` offsets**, so that the C code's `tree[lo - 1]` read (which lands
//! in the *previous* member) is reproduced faithfully.

#![allow(dead_code)]

/// `offsetof(cp_state_t, ...)` for the x86-64 SysV ABI.
pub const OFF_LOOKUP: usize = 72; // uint16_t lookup[512]
pub const OFF_LIT: usize = 1096; // uint32_t lit[288]
pub const OFF_DST: usize = 2248; // uint32_t dst[32]
pub const OFF_LEN: usize = 2376; // uint32_t len[19]
pub const STATE_SIZE: usize = 2464;

/// Step budget; a legitimate decode needs at most ~`out_bytes` steps.
pub const STEP_BUDGET: u64 = 400_000;

pub const FIXED_TABLE: [u8; 288 + 32] = {
    let mut t = [8u8; 320];
    let mut i = 144;
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    while i < 320 {
        t[i] = 5;
        i += 1;
    }
    t
};

pub const PERM: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
pub const LEN_EXTRA: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
pub const LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
pub const DIST_EXTRA: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];
pub const DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

pub const ERR_LEN_NLEN: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
pub const ERR_STORED_BEYOND: &str = "Stored block extends beyond end of input stream.";
pub const ERR_OUT_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
pub const ERR_BACKWARDS: &str = "Attempted to write before out buffer (invalid backwards distance).";
pub const ERR_OUT_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
pub const ERR_UNKNOWN_BLOCK: &str = "Detected unknown block type within input stream.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum End {
    /// `cp_inflate` returned this value; `Some(msg)` is `cp_error_reason`.
    Ret(i32, Option<&'static str>),
    /// a live `assert()` failed at `lib.c:<line>`
    Abort(u32),
    /// the model exceeded its step budget (the C code spins forever)
    Loop,
    /// a read outside the caller's input mapping (would fault)
    Fault,
}

#[derive(Debug, Clone)]
pub struct ModelResult {
    pub end: End,
    /// every C operation with undefined behaviour that the run performed
    pub ub: Vec<String>,
    /// bytes written into the output buffer (only the in-bounds prefix)
    pub out: Vec<u8>,
}

impl ModelResult {
    pub fn defined(&self) -> bool {
        self.ub.is_empty()
    }
}

struct M<'a> {
    map: &'a [u8],
    in_off: i64,
    in_bytes: i32,

    bits: u64,
    count: i32,
    word_index: i32,
    word_count: i32,
    bits_left: i32,
    fwa: i32,
    final_word: u32,
    first_bytes: i32,

    mem: Vec<u8>,
    nlit: u32,
    ndst: u32,
    nlen: u32,

    out: Vec<u8>,
    out_pos: i64,
    out_bytes: i32,

    ub: Vec<String>,
    steps: u64,
    faulted: bool,
}

/// Signals a non-local exit, mirroring the C control flow.
enum Stop {
    Abort(u32),
    Loop,
    Fault,
}

type R<T> = Result<T, Stop>;

impl<'a> M<'a> {
    fn ubx(&mut self, s: String) {
        if self.ub.len() < 8 && !self.ub.contains(&s) {
            self.ub.push(s);
        }
    }

    /// `((uint8_t *)in)[idx]`
    fn inb(&mut self, idx: i64) -> R<u8> {
        let abs = self.in_off + idx;
        if abs < 0 || abs >= self.map.len() as i64 {
            self.faulted = true;
            return Err(Stop::Fault);
        }
        Ok(self.map[abs as usize])
    }

    /// `s->words[i]`
    fn word(&mut self, i: i32) -> R<u32> {
        let base = self.first_bytes as i64 + 4 * i as i64;
        let mut v = 0u32;
        for k in 0..4 {
            v |= (self.inb(base + k)? as u32) << (8 * k);
        }
        Ok(v)
    }

    fn rd32(&self, off: i64) -> u32 {
        let o = off as usize;
        u32::from_le_bytes([
            self.mem[o],
            self.mem[o + 1],
            self.mem[o + 2],
            self.mem[o + 3],
        ])
    }
    fn wr32(&mut self, off: i64, v: u32) {
        let o = off as usize;
        self.mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn wr16(&mut self, off: i64, v: u16) {
        let o = off as usize;
        self.mem[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }

    // -- bit reader ---------------------------------------------------------

    fn would_overflow(&self, num_bits: i32) -> bool {
        self.bits_left
            .wrapping_add(self.count)
            .wrapping_sub(num_bits)
            < 0
    }

    /// `cp_ptr()` — byte offset relative to `in`
    fn ptr(&mut self) -> R<i64> {
        if self.bits_left & 7 != 0 {
            return Err(Stop::Abort(95));
        }
        Ok(self.first_bytes as i64 + 4 * self.word_index as i64 - (self.count as i64 / 8))
    }

    fn peak(&mut self, num_bits: i32) -> R<u64> {
        if self.count < num_bits {
            if self.word_index < self.word_count {
                let w = self.word(self.word_index)?;
                self.word_index = self.word_index.wrapping_add(1);
                self.bits |= (w as u64) << (self.count as u32 & 63);
                self.count = self.count.wrapping_add(32);
                if self.word_index > self.word_count {
                    return Err(Stop::Abort(104));
                }
            } else if self.fwa != 0 {
                let w = self.final_word;
                self.bits |= (w as u64) << (self.count as u32 & 63);
                self.count = self.count.wrapping_add(self.bits_left);
                self.fwa = 0;
            }
        }
        Ok(self.bits)
    }

    fn consume(&mut self, num_bits: i32) -> R<u32> {
        if self.count < num_bits {
            return Err(Stop::Abort(115));
        }
        let v = (self.bits & (1u64.wrapping_shl(num_bits as u32).wrapping_sub(1))) as u32;
        self.bits >>= num_bits as u32 & 63;
        self.count = self.count.wrapping_sub(num_bits);
        self.bits_left = self.bits_left.wrapping_sub(num_bits);
        Ok(v)
    }

    fn read(&mut self, num_bits: i32) -> R<u32> {
        if num_bits > 32 {
            return Err(Stop::Abort(123));
        }
        if num_bits < 0 {
            return Err(Stop::Abort(124));
        }
        if self.bits_left <= 0 {
            return Err(Stop::Abort(125));
        }
        if self.count > 64 {
            return Err(Stop::Abort(126));
        }
        if self.would_overflow(num_bits) {
            return Err(Stop::Abort(127));
        }
        self.peak(num_bits)?;
        self.consume(num_bits)
    }

    // -- huffman ------------------------------------------------------------

    fn build(&mut self, fill_lookup: bool, tree_off: usize, lens: &[u8], sym_count: i32) -> R<i32> {
        let mut codes = [0i32; 16];
        let mut first = [0i32; 16];
        let mut counts = [0i32; 256];
        for n in 0..sym_count as usize {
            let l = lens[n] as usize;
            if l >= 16 {
                self.ubx(format!("cp_build: counts[{l}]++ past int counts[16]"));
            }
            counts[l] += 1;
        }
        counts[0] = 0;
        for n in 1..=15usize {
            codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
            first[n] = first[n - 1] + counts[n - 1];
        }
        if fill_lookup {
            for i in 0..512 {
                self.wr16((OFF_LOOKUP + 2 * i) as i64, 0);
            }
        }
        for i in 0..sym_count as usize {
            let l = lens[i] as i32;
            if l != 0 {
                if l >= 16 {
                    return Err(Stop::Abort(154));
                }
                let code = codes[l as usize] as u32;
                codes[l as usize] += 1;
                let slot = first[l as usize];
                first[l as usize] += 1;
                self.wr32(
                    tree_off as i64 + 4 * slot as i64,
                    (code << (32 - l)) | ((i as u32) << 4) | l as u32,
                );
                if fill_lookup && l <= 9 {
                    let mut j = (rev16(code) >> (16 - l)) as i32;
                    while j < 512 {
                        self.wr16(
                            (OFF_LOOKUP as i64 + 2 * j as i64) as i64,
                            (((l as u32) << 9) | i as u32) as u16,
                        );
                        j += 1 << l;
                    }
                }
            }
        }
        Ok(first[15])
    }

    fn decode(&mut self, tree_off: usize, hi_in: i32) -> R<i32> {
        let bits = self.peak(16)?;
        let search = (rev16(bits as u32) << 16) | 0xFFFF;
        let mut lo = 0i32;
        let mut hi = hi_in;
        while lo < hi {
            let guess = (lo + hi) >> 1;
            if search < self.rd32(tree_off as i64 + 4 * guess as i64) {
                hi = guess;
            } else {
                lo = guess + 1;
            }
        }
        let key = self.rd32(tree_off as i64 + 4 * (lo - 1) as i64);
        let l = 32u32.wrapping_sub(key & 0xF);
        if search.wrapping_shr(l) != key.wrapping_shr(l) {
            return Err(Stop::Abort(217));
        }
        self.consume((key & 0xF) as i32)?;
        Ok(((key >> 4) & 0xFFF) as i32)
    }

    // -- blocks -------------------------------------------------------------

    fn stored(&mut self) -> R<i32> {
        let a = self.count & 7;
        self.read(a)?;
        let len = self.read(16)? as u16;
        let nlen = self.read(16)? as u16;
        if len != !nlen {
            return Ok(0); // ERR_LEN_NLEN
        }
        if !(self.bits_left / 8 <= len as i32) {
            return Ok(-1); // ERR_STORED_BEYOND
        }
        let p = self.ptr()?;
        // NB: `cp_stored` performs `memcpy(s->out, p, LEN)` with **no** output
        // bounds check. In the harness the output is a page-guarded mapping, so
        // a write inside the mapping is well defined (and compared) while a
        // write past it faults on the guard page.
        for i in 0..len as i64 {
            let b = self.inb(p + i)?;
            let dst = self.out_pos + i;
            if dst < 0 || dst >= self.out.len() as i64 {
                self.faulted = true;
                return Err(Stop::Fault);
            }
            self.out[dst as usize] = b;
        }
        self.out_pos += len as i64;
        Ok(1)
    }

    fn fixed(&mut self) -> R<()> {
        let t = FIXED_TABLE;
        self.nlit = self.build(true, OFF_LIT, &t[..288], 288)? as u32;
        self.ndst = self.build(false, OFF_DST, &t[288..], 32)? as u32;
        Ok(())
    }

    fn dynamic(&mut self) -> R<()> {
        let mut lenlens = [0u8; 19];
        let nlit = 257 + self.read(5)? as i32;
        let ndst = 1 + self.read(5)? as i32;
        let nlen = 4 + self.read(4)? as i32;
        for i in 0..nlen as usize {
            lenlens[PERM[i]] = self.read(3)? as u8;
        }
        self.nlen = self.build(false, OFF_LEN, &lenlens, 19)? as u32;

        // C: `uint8_t lens[288 + 32];` (uninitialised). `store[0]` models
        // `lens[-1]`, so `lens[i] == store[i + 1]`.
        const CAP: i32 = 288 + 32;
        let mut store = vec![0u8; 1 + CAP as usize + 200];
        let mut n = 0i32;
        while n < nlit + ndst {
            self.steps += 1;
            if self.steps > STEP_BUDGET {
                return Err(Stop::Loop);
            }
            let sym = self.decode(OFF_LEN, self.nlen as i32)?;
            let (count, val): (i32, Option<u8>) = match sym {
                16 => {
                    if n == 0 {
                        self.ubx("cp_dynamic: lens[-1] read (uninitialised stack)".to_string());
                    }
                    (3 + self.read(2)? as i32, None)
                }
                17 => (3 + self.read(3)? as i32, Some(0)),
                18 => (11 + self.read(7)? as i32, Some(0)),
                _ => (1, Some(sym as u8)),
            };
            for _ in 0..count {
                if n < 0 || n >= CAP {
                    self.ubx(format!(
                        "cp_dynamic: lens[n] written for n >= {CAP}, past uint8_t lens[288+32] \
                         (a 16/17/18 run overshooting nlit+ndst)"
                    ));
                }
                let v = match val {
                    Some(v) => v,
                    // `lens[n] = lens[n - 1]`
                    None => store[(n as usize).min(store.len() - 1)],
                };
                let idx = (n + 1) as usize;
                if idx < store.len() {
                    store[idx] = v;
                }
                n += 1;
            }
        }
        let a = store[1..1 + nlit as usize].to_vec();
        let b = store[1 + nlit as usize..1 + (nlit + ndst) as usize].to_vec();
        self.nlit = self.build(true, OFF_LIT, &a, nlit)? as u32;
        self.ndst = self.build(false, OFF_DST, &b, ndst)? as u32;
        Ok(())
    }

    fn block(&mut self) -> R<i32> {
        loop {
            self.steps += 1;
            if self.steps > STEP_BUDGET {
                return Err(Stop::Loop);
            }
            let mut symbol = self.decode(OFF_LIT, self.nlit as i32)?;
            if symbol < 256 {
                if !(self.out_pos + 1 <= self.out_bytes as i64) {
                    return Ok(0); // ERR_OUT_SYMBOL
                }
                if self.out_pos >= 0 && self.out_pos < self.out.len() as i64 {
                    self.out[self.out_pos as usize] = symbol as u8;
                }
                self.out_pos += 1;
            } else if symbol > 256 {
                symbol -= 257;
                if symbol < 0 || symbol as usize >= LEN_EXTRA.len() {
                    self.ubx(format!(
                        "cp_block: cp_len_extra_bits[{symbol}] past uint8_t[29+2]"
                    ));
                    return Err(Stop::Loop); // no defined continuation
                }
                let length = self
                    .read(LEN_EXTRA[symbol as usize] as i32)?
                    .wrapping_add(LEN_BASE[symbol as usize]) as i32;
                let dsym = self.decode(OFF_DST, self.ndst as i32)?;
                if dsym < 0 || dsym as usize >= DIST_EXTRA.len() {
                    self.ubx(format!(
                        "cp_block: cp_dist_extra_bits[{dsym}] past uint8_t[30+2]"
                    ));
                    return Err(Stop::Loop);
                }
                let bd = self
                    .read(DIST_EXTRA[dsym as usize] as i32)?
                    .wrapping_add(DIST_BASE[dsym as usize]) as i32;
                if !(self.out_pos - bd as i64 >= 0) {
                    return Ok(-1); // ERR_BACKWARDS
                }
                if !(self.out_pos + length as i64 <= self.out_bytes as i64) {
                    return Ok(-2); // ERR_OUT_STRING
                }
                let src0 = self.out_pos - bd as i64;
                let dst0 = self.out_pos;
                self.out_pos += length as i64;
                for i in 0..length as i64 {
                    let s = if bd == 1 { src0 } else { src0 + i };
                    let v = if s >= 0 && s < self.out.len() as i64 {
                        self.out[s as usize]
                    } else {
                        0
                    };
                    let d = dst0 + i;
                    if d >= 0 && d < self.out.len() as i64 {
                        self.out[d as usize] = v;
                    }
                }
            } else {
                return Ok(1);
            }
        }
    }

    fn inflate(&mut self) -> R<(i32, Option<&'static str>)> {
        loop {
            self.steps += 1;
            if self.steps > STEP_BUDGET {
                return Err(Stop::Loop);
            }
            let bfinal = self.read(1)?;
            let btype = self.read(2)?;
            match btype {
                0 => match self.stored()? {
                    1 => {}
                    0 => return Ok((0, Some(ERR_LEN_NLEN))),
                    _ => return Ok((0, Some(ERR_STORED_BEYOND))),
                },
                1 => {
                    self.fixed()?;
                    match self.block()? {
                        1 => {}
                        0 => return Ok((0, Some(ERR_OUT_SYMBOL))),
                        -1 => return Ok((0, Some(ERR_BACKWARDS))),
                        _ => return Ok((0, Some(ERR_OUT_STRING))),
                    }
                }
                2 => {
                    self.dynamic()?;
                    match self.block()? {
                        1 => {}
                        0 => return Ok((0, Some(ERR_OUT_SYMBOL))),
                        -1 => return Ok((0, Some(ERR_BACKWARDS))),
                        _ => return Ok((0, Some(ERR_OUT_STRING))),
                    }
                }
                _ => return Ok((0, Some(ERR_UNKNOWN_BLOCK))),
            }
            if bfinal != 0 {
                return Ok((1, None));
            }
        }
    }
}

fn rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

/// Run the model. `map` is the whole (zero-filled) input mapping, `in_off` the
/// byte offset of the pointer handed to `cp_inflate` — which also determines
/// `first_bytes`, because the mapping itself is page (hence 4-byte) aligned.
pub fn run(
    map: &[u8],
    in_off: usize,
    in_bytes: i32,
    out_bytes: i32,
    out_usable: usize,
) -> ModelResult {
    let first_bytes = (((in_off as i64 + 3) & !3) - in_off as i64) as i32;
    let mut m = M {
        map,
        in_off: in_off as i64,
        in_bytes,
        bits: 0,
        count: 0,
        word_index: 0,
        word_count: in_bytes.wrapping_sub(first_bytes) / 4,
        bits_left: in_bytes.wrapping_mul(8),
        fwa: 0,
        final_word: 0,
        first_bytes,
        mem: vec![0u8; STATE_SIZE],
        nlit: 0,
        ndst: 0,
        nlen: 0,
        out: vec![0xA5u8; out_usable],
        out_pos: 0,
        out_bytes,
        ub: Vec::new(),
        steps: 0,
        faulted: false,
    };
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;

    let mut early: Option<End> = None;
    for i in 0..first_bytes as i64 {
        match m.inb(i) {
            Ok(b) => m.bits |= (b as u64) << (i as u32 * 8),
            Err(_) => {
                early = Some(End::Fault);
                break;
            }
        }
    }
    if early.is_none() {
        m.fwa = if last_bytes != 0 { 1 } else { 0 };
        for i in 0..last_bytes as i64 {
            match m.inb(in_bytes as i64 - last_bytes as i64 + i) {
                Ok(b) => m.final_word |= (b as u32) << (i as u32 * 8),
                Err(_) => {
                    early = Some(End::Fault);
                    break;
                }
            }
        }
    }
    m.count = first_bytes.wrapping_mul(8);

    let end = match early {
        Some(e) => e,
        None => match m.inflate() {
            Ok((r, msg)) => End::Ret(r, msg),
            Err(Stop::Abort(l)) => End::Abort(l),
            Err(Stop::Loop) => End::Loop,
            Err(Stop::Fault) => End::Fault,
        },
    };
    ModelResult {
        end,
        ub: m.ub.clone(),
        out: m.out.clone(),
    }
}
