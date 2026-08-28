//! Level 4: randomised differential fuzzing of the whole hash-map API through
//! both `.so` exports. A small key pool makes duplicate inserts, tombstone
//! reuse, table growth, rebuilds and shrinks all frequent.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Mirrors the macro expansions, exactly as in level2, but driven by the RNG.
struct M<'a> {
    im: &'a Impl,
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
}

impl<'a> M<'a> {
    fn temp(&self) -> isize {
        unsafe { map_temp(self.t, self.elemsize) }
    }
    fn tail(&self, i: isize, seed: u8) {
        unsafe {
            let e = (self.t as *mut u8).offset(i * self.elemsize as isize);
            for b in self.keysize..self.elemsize {
                *e.add(b) = seed.wrapping_mul(31).wrapping_add(b as u8);
            }
        }
    }
    fn put(&mut self, k: *mut c_void, mode: c_int, seed: u8) -> isize {
        unsafe {
            self.t = (self.im.hmput_key)(self.t, self.elemsize, k, self.keysize, mode);
            let t = self.temp();
            self.tail(t, seed);
            t
        }
    }
    fn geti(&mut self, k: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmget_key)(self.t, self.elemsize, k, self.keysize, mode);
            self.temp()
        }
    }
    fn geti_ts(&mut self, k: *mut c_void, mode: c_int) -> isize {
        unsafe {
            let mut tmp = 0isize;
            self.t =
                (self.im.hmget_key_ts)(self.t, self.elemsize, k, self.keysize, &mut tmp, mode);
            tmp
        }
    }
    fn del(&mut self, k: *mut c_void, mode: c_int) -> isize {
        unsafe {
            self.t = (self.im.hmdel_key)(self.t, self.elemsize, k, self.keysize, 0, mode);
            if self.t.is_null() {
                0
            } else {
                self.temp()
            }
        }
    }
    fn default(&mut self, seed: u8) {
        unsafe {
            self.t = (self.im.hmput_default)(self.t, self.elemsize);
            self.tail(-1, seed);
        }
    }
    fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(
                    (self.t as *mut u8).sub(self.elemsize) as *mut c_void,
                    self.elemsize,
                );
            }
            self.t = std::ptr::null_mut();
        }
    }
}

/// `is_string` selects `STBDS_HM_STRING` vs `STBDS_HM_BINARY`; `sh_mode`, when
/// present, creates the map up front via `stbds_shmode_func` (`sh_new_strdup` /
/// `sh_new_arena`) instead of growing it from NULL.
fn fuzz_round(
    c: &Impl,
    r: &Impl,
    rng_seed: u64,
    elemsize: usize,
    is_string: bool,
    sh_mode: Option<c_int>,
    ops: usize,
) {
    let _g = seeded(c, r, 0x3141_5926 ^ rng_seed as usize);
    let mut rng = Rng(rng_seed | 1);

    let mode = if is_string { HM_STRING } else { HM_BINARY };
    let keysize = 8usize;
    let kind = if is_string {
        KeyKind::StringPtr
    } else {
        KeyKind::Inline
    };
    let cfg = format!("string={is_string} sh_mode={sh_mode:?}");

    // Key pool. Kept alive for the whole round (STBDS_SH_DEFAULT stores the
    // caller's pointer verbatim).
    const POOL: usize = 48;
    let mut keys: Vec<Vec<u8>> = Vec::with_capacity(POOL);
    for i in 0..POOL {
        if is_string {
            let mut s = format!("k{i:03}").into_bytes();
            if i % 5 == 0 {
                s.extend(std::iter::repeat(b'z').take(i * 11));
            }
            s.push(0);
            keys.push(s);
        } else {
            keys.push((i as u64).to_le_bytes().to_vec());
        }
    }

    let mut cm = M {
        im: c,
        t: std::ptr::null_mut(),
        elemsize,
        keysize,
    };
    let mut rm = M {
        im: r,
        t: std::ptr::null_mut(),
        elemsize,
        keysize,
    };
    if let Some(shm) = sh_mode {
        unsafe {
            cm.t = (c.shmode_func)(elemsize, shm);
            rm.t = (r.shmode_func)(elemsize, shm);
        }
    }

    for step in 0..ops {
        let ki = rng.below(POOL);
        let kp = keys[ki].as_mut_ptr() as *mut c_void;
        let seed = (step % 251) as u8;
        let op = rng.below(100);
        let (cv, rv, label): (isize, isize, String) = if op < 40 {
            (
                cm.put(kp, mode, seed),
                rm.put(kp, mode, seed),
                format!("put k{ki}"),
            )
        } else if op < 60 {
            (
                cm.geti(kp, mode),
                rm.geti(kp, mode),
                format!("geti k{ki}"),
            )
        } else if op < 70 {
            (
                cm.geti_ts(kp, mode),
                rm.geti_ts(kp, mode),
                format!("geti_ts k{ki}"),
            )
        } else if op < 97 {
            (cm.del(kp, mode), rm.del(kp, mode), format!("del k{ki}"))
        } else {
            cm.default(seed);
            rm.default(seed);
            (0, 0, "default".to_string())
        };
        assert_eq!(
            cv, rv,
            "fuzz seed={rng_seed} elemsize={elemsize} {cfg} step={step} ({label}): C={cv} Rust={rv}"
        );
        let cs = unsafe { snapshot_map(cm.t, elemsize, keysize, kind) };
        let rs = unsafe { snapshot_map(rm.t, elemsize, keysize, kind) };
        assert_same(
            &format!(
                "fuzz seed={rng_seed} elemsize={elemsize} {cfg} step={step} ({label})"
            ),
            &cs,
            &rs,
        );
    }

    cm.free();
    rm.free();
}

#[test]
fn fuzz_binary_maps() {
    let (c, r) = load_pair();
    for (i, &elemsize) in [8usize, 16, 24, 32].iter().enumerate() {
        for round in 0..3u64 {
            fuzz_round(
                &c,
                &r,
                0x1000 + round * 7919 + i as u64 * 104729,
                elemsize,
                false,
                None,
                400,
            );
        }
    }
}

#[test]
fn fuzz_string_maps_default() {
    let (c, r) = load_pair();
    for round in 0..3u64 {
        fuzz_round(&c, &r, 0x2000 + round * 6151, 16, true, None, 400);
    }
}

#[test]
fn fuzz_string_maps_strdup() {
    let (c, r) = load_pair();
    for round in 0..3u64 {
        fuzz_round(&c, &r, 0x3000 + round * 3571, 16, true, Some(SH_STRDUP), 400);
    }
}

#[test]
fn fuzz_string_maps_arena() {
    let (c, r) = load_pair();
    for round in 0..3u64 {
        fuzz_round(&c, &r, 0x4000 + round * 2617, 24, true, Some(SH_ARENA), 400);
    }
}

/// Interleave a standalone string arena with map traffic: `stbds_stralloc` and
/// `stbds_strreset` reached directly rather than via `STBDS_SH_ARENA`.
#[test]
fn fuzz_string_arena() {
    let (c, r) = load_pair();
    let mut rng = Rng(0xfeed_face);
    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    let mut clog = Vec::new();
    let mut rlog = Vec::new();
    unsafe {
        for step in 0..600usize {
            if rng.below(100) < 4 {
                (c.strreset)(&mut ca);
                (r.strreset)(&mut ra);
                clog.extend_from_slice(format!("{step}:reset;").as_bytes());
                rlog.extend_from_slice(format!("{step}:reset;").as_bytes());
                continue;
            }
            let len = 1 + rng.below(1500);
            let mut buf = vec![b'a' + (step % 26) as u8; len];
            buf.push(0);
            let cp = (c.stralloc)(&mut ca, buf.as_mut_ptr() as *mut c_char);
            let rp = (r.stralloc)(&mut ra, buf.as_mut_ptr() as *mut c_char);
            clog.extend_from_slice(
                format!(
                    "{step}:{} rem={} blk={} snull={};",
                    cstr_bytes(cp).len(),
                    ca.remaining,
                    ca.block,
                    ca.storage.is_null()
                )
                .as_bytes(),
            );
            rlog.extend_from_slice(
                format!(
                    "{step}:{} rem={} blk={} snull={};",
                    cstr_bytes(rp).len(),
                    ra.remaining,
                    ra.block,
                    ra.storage.is_null()
                )
                .as_bytes(),
            );
            assert_same(&format!("stralloc step {step} contents"), &cstr_bytes(cp), &cstr_bytes(rp));
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
    assert_same("stralloc fuzz", &clog, &rlog);
}
