//! Randomized differential test over both exported functions.

mod common;

use common::{free, GuardedCStr, Libs};
use std::os::raw::{c_char, c_void};

/// Deterministic xorshift64* PRNG so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Random NUL-free byte string, biased towards separator characters so the
    /// interesting branches are hit often.
    fn string(&mut self, max_len: usize) -> Vec<u8> {
        const ALPHABET: &[u8] = b"//\\\\..abZ9 \t:-_\x01\x7f\x80\xfe\xff";
        let len = self.below(max_len + 1);
        (0..len).map(|_| ALPHABET[self.below(ALPHABET.len())]).collect()
    }
}

unsafe fn strlen(mut p: *const c_char) -> usize {
    let mut n = 0;
    while unsafe { *p } != 0 {
        n += 1;
        p = unsafe { p.add(1) };
    }
    n
}

#[test]
fn randomized_differential() {
    let libs = Libs::load();
    let (c_extract, r_extract) = libs.extract_filename();
    let (c_create, r_create) = libs.create_filename();

    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for iter in 0..20_000u32 {
        let path_bytes = rng.string(24);
        let out_dir_bytes = rng.string(12);
        let guard_path = (rng.next_u64() & 0xff) as u8;
        let guard_out = [b'/', b'\\', b'x', 0u8][rng.below(4)];
        let suffix_len = rng.below(16);

        let path = GuardedCStr::new(guard_path, &path_bytes);
        let out_dir = GuardedCStr::new(guard_out, &out_dir_bytes);

        // extractFilename with a random separator
        let sep = (rng.next_u64() & 0xff) as u8 as i8 as c_char;
        let ce = unsafe { c_extract(path.ptr(), sep) };
        let re = unsafe { r_extract(path.ptr(), sep) };
        assert_eq!(
            ce, re,
            "iter {iter}: extractFilename mismatch, path={path_bytes:?} sep={sep}"
        );

        // FIO_createFilename_fromOutDir
        let cc = unsafe { c_create(path.ptr(), out_dir.ptr(), suffix_len) };
        let rc = unsafe { r_create(path.ptr(), out_dir.ptr(), suffix_len) };
        let od_len = unsafe { strlen(out_dir.ptr()) };
        let fname = unsafe { c_extract(path.ptr(), b'/' as i8 as c_char) };
        let total = od_len + 1 + unsafe { strlen(fname) } + suffix_len + 1;
        let cb = unsafe { std::slice::from_raw_parts(cc as *const u8, total) };
        let rb = unsafe { std::slice::from_raw_parts(rc as *const u8, total) };
        assert_eq!(
            cb, rb,
            "iter {iter}: FIO_createFilename_fromOutDir mismatch, path={path_bytes:?} outDir={out_dir_bytes:?} guardOut={guard_out:#04x} suffixLen={suffix_len}"
        );
        unsafe {
            free(cc as *mut c_void);
            free(rc as *mut c_void);
        }
    }
}
