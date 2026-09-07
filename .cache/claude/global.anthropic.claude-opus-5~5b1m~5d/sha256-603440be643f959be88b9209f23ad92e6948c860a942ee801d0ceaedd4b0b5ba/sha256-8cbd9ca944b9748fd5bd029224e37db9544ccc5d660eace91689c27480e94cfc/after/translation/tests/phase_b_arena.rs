//! Phase B rows C46–C56: `stbds_stralloc` / `stbds_strreset`, `strkey`,
//! `sh_puts` (stdout byte-for-byte) and the composed end-to-end pipelines.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

fn chain_len(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut p = a.storage as *mut StringBlock;
    unsafe {
        while !p.is_null() {
            n += 1;
            assert!(n < 1_000_000, "arena chain looks circular");
            p = (*p).next;
        }
    }
    n
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    blocks: usize,
}

fn arena_snap(a: &StringArena) -> ArenaSnap {
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        has_storage: !a.storage.is_null(),
        blocks: chain_len(a),
    }
}

/// One `stralloc` call on both libraries; compares the returned string, the
/// arena state, and the integrity of every previously returned pointer.
struct Arenas<'a> {
    s: &'a Session<'a>,
    ca: StringArena,
    ra: StringArena,
    /// (returned C ptr, returned Rust ptr, expected bytes)
    live: Vec<(*mut c_char, *mut c_char, Vec<u8>)>,
}

impl<'a> Arenas<'a> {
    fn new(s: &'a Session<'a>) -> Arenas<'a> {
        Arenas {
            s,
            ca: StringArena::zeroed(),
            ra: StringArena::zeroed(),
            live: Vec::new(),
        }
    }

    fn alloc(&mut self, ctx: &str, body: &[u8]) {
        let cs = CStrBuf::new(body);
        unsafe {
            let cp = (self.s.c.stralloc)(&mut self.ca, cs.ptr());
            let rp = (self.s.r.stralloc)(&mut self.ra, cs.ptr());
            assert!(!cp.is_null() && !rp.is_null(), "{ctx}: NULL return");
            assert_eq!(read_cstr(cp), body, "{ctx}: C returned the wrong string");
            assert_eq!(read_cstr(rp), body, "{ctx}: Rust returned the wrong string");
            assert_eq!(
                arena_snap(&self.ca),
                arena_snap(&self.ra),
                "{ctx}: arena state differs"
            );
            self.live.push((cp, rp, body.to_vec()));
            // Every earlier allocation must still read back correctly in BOTH.
            for (i, (c, r, want)) in self.live.iter().enumerate() {
                assert_eq!(&read_cstr(*c), want, "{ctx}: C clobbered allocation #{i}");
                assert_eq!(&read_cstr(*r), want, "{ctx}: Rust clobbered allocation #{i}");
            }
        }
    }

    /// Same, but without the O(n^2) integrity sweep (for the long saturation run).
    fn alloc_fast(&mut self, ctx: &str, body: &[u8]) {
        let cs = CStrBuf::new(body);
        unsafe {
            let cp = (self.s.c.stralloc)(&mut self.ca, cs.ptr());
            let rp = (self.s.r.stralloc)(&mut self.ra, cs.ptr());
            assert_eq!(read_cstr(cp), body, "{ctx}: C string");
            assert_eq!(read_cstr(rp), body, "{ctx}: Rust string");
            assert_eq!(
                arena_snap(&self.ca),
                arena_snap(&self.ra),
                "{ctx}: arena state differs"
            );
        }
    }

    fn reset(&mut self, ctx: &str) {
        unsafe {
            (self.s.c.strreset)(&mut self.ca);
            (self.s.r.strreset)(&mut self.ra);
        }
        assert_eq!(
            arena_snap(&self.ca),
            arena_snap(&self.ra),
            "{ctx}: arena state after strreset"
        );
        assert_eq!(self.ca.remaining, 0, "{ctx}: C remaining must be 0");
        assert_eq!(self.ca.block, 0, "{ctx}: C block must be 0");
        assert!(self.ca.storage.is_null(), "{ctx}: C storage must be NULL");
        assert_eq!(self.ra.remaining, 0, "{ctx}: Rust remaining must be 0");
        assert_eq!(self.ra.block, 0, "{ctx}: Rust block must be 0");
        assert!(self.ra.storage.is_null(), "{ctx}: Rust storage must be NULL");
        self.live.clear();
    }
}

// ---------------------------------------------------------------- C46
#[test]
fn c46_stralloc_short_strings() {
    let s = session(1);
    let mut a = Arenas::new(&s);
    let mut rng = Rng::new(0x46);
    for i in 0..400usize {
        let body = rng.ascii_range(0, 200);
        a.alloc(&format!("C46 #{i} len={}", body.len()), &body);
    }
    a.reset("C46");
}

// ---------------------------------------------------------------- C47
#[test]
fn c47_stralloc_oversize_on_fresh_arena() {
    let s = session(1);
    for len in [512usize, 513, 1000, 5000, 100_000] {
        let mut a = Arenas::new(&s);
        let body = vec![b'X'; len];
        a.alloc(&format!("C47 fresh len={len}"), &body);
        // then a few normal allocations on top
        let mut rng = Rng::new(0x47 + len as u64);
        for i in 0..20usize {
            let b = rng.ascii_range(0, 100);
            a.alloc(&format!("C47 len={len} follow#{i}"), &b);
        }
        a.reset(&format!("C47 len={len}"));
    }
}

// ---------------------------------------------------------------- C48
#[test]
fn c48_stralloc_oversize_after_head_exists() {
    let s = session(1);
    let mut a = Arenas::new(&s);
    // Establish a head block first ...
    a.alloc("C48 seed", b"hello");
    // ... then force the `storage != NULL` oversize sub-case repeatedly.
    for (i, len) in [600usize, 2000, 9000, 70_000, 600].iter().enumerate() {
        a.alloc(&format!("C48 oversize#{i} len={len}"), &vec![b'Y'; *len]);
        a.alloc(&format!("C48 small-after#{i}"), b"tail");
    }
    a.reset("C48");
}

// ---------------------------------------------------------------- C49
#[test]
fn c49_stralloc_block_counter_saturates() {
    let s = session(1);
    let mut a = Arenas::new(&s);
    // 512-byte payloads: every time `remaining` runs out a fresh block is taken
    // and `a->block` advances while `512 << (block>>1) < (1<<20)`.
    let body = vec![b'Z'; 511];
    let mut prev_block = 0u8;
    let mut saturated_at: Option<u8> = None;
    for i in 0..6000usize {
        a.alloc_fast(&format!("C49 #{i}"), &body);
        let b = a.ca.block;
        assert_eq!(b, a.ra.block, "C49 #{i}: block counter diverged");
        if b == prev_block && b >= 20 {
            // may just be the fast path; keep going
        }
        prev_block = b;
        if b >= 22 {
            saturated_at = Some(b);
        }
    }
    assert_eq!(
        saturated_at,
        Some(22),
        "C49: expected a->block to saturate at 22 (512<<11 == 1<<20), got {:?}",
        a.ca.block
    );
    // Many more allocations must NOT push it past 22 in either library.
    for i in 0..3000usize {
        a.alloc_fast(&format!("C49 post#{i}"), &body);
        assert_eq!(a.ca.block, 22, "C49 post#{i}: C block escaped saturation");
        assert_eq!(a.ra.block, 22, "C49 post#{i}: Rust block escaped saturation");
    }
    a.reset("C49");
}

// ---------------------------------------------------------------- C50
#[test]
fn c50_stralloc_exact_boundaries() {
    let s = session(1);
    // len == blocksize, blocksize+1, blocksize-1 on a fresh arena (blocksize=512)
    for len in [510usize, 511, 512, 513] {
        let mut a = Arenas::new(&s);
        a.alloc(&format!("C50 fresh len+1={}", len + 1), &vec![b'A'; len]);
        a.reset(&format!("C50 fresh {len}"));
    }
    // len == remaining and len == remaining+1 on a partially used arena
    for first in [0usize, 1, 100, 400, 510] {
        for delta in [-1i64, 0, 1] {
            let mut a = Arenas::new(&s);
            a.alloc("C50 prime", &vec![b'B'; first]);
            let rem = a.ca.remaining;
            assert_eq!(rem, a.ra.remaining, "C50: remaining diverged after prime");
            let want = rem as i64 + delta; // desired len (= body+1)
            if want < 1 {
                continue;
            }
            let body = vec![b'C'; (want - 1) as usize];
            a.alloc(
                &format!("C50 first={first} delta={delta} rem={rem} len={want}"),
                &body,
            );
            a.reset("C50 boundary");
        }
    }
}

// ---------------------------------------------------------------- C28/C51
#[test]
fn c51_stralloc_reset_and_reuse() {
    let s = session(1);
    let mut a = Arenas::new(&s);
    let mut rng = Rng::new(0x51);
    for round in 0..6usize {
        for i in 0..120usize {
            let body = rng.ascii_range(0, 300);
            a.alloc(&format!("C51 r={round} #{i}"), &body);
        }
        a.reset(&format!("C51 r={round}"));
        // double reset: must stay a no-op
        a.reset(&format!("C51 r={round} again"));
    }
    // empty string allocations
    for i in 0..600usize {
        a.alloc(&format!("C51 empty#{i}"), b"");
    }
    a.reset("C51 empty");
}

// ---------------------------------------------------------------- C52
#[test]
fn c52_strkey() {
    let s = session(1);
    let mut fixed: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1000000000,
        -1000000000,
    ];
    let mut rng = Rng::new(0x52);
    for _ in 0..200 {
        fixed.push(rng.next_u64() as i32);
    }
    for n in fixed {
        unsafe {
            let cp = (s.c.strkey)(n);
            let c = read_cstr(cp);
            let rp = (s.r.strkey)(n);
            let r = read_cstr(rp);
            assert_eq!(
                c,
                r,
                "C52 strkey({n}): C={:?} Rust={:?}",
                String::from_utf8_lossy(&c),
                String::from_utf8_lossy(&r)
            );
            assert_eq!(
                String::from_utf8_lossy(&c),
                format!("test_{n}"),
                "C52 strkey({n}) content"
            );
        }
    }
}

// ---------------------------------------------------------------- C53
#[test]
fn c53_sh_puts_stdout_byte_for_byte() {
    let s = session(0x31415926);
    // NOTE: `num = INT_MAX` is deliberately absent.  `sh_puts` runs
    // `for (i=0; i<num; ++i) stralloc(&sa, strkey(i))`, so a large positive
    // `num` makes the C perform `num` arena allocations (tens of GB) -- it is
    // not a runnable input for either library.  Negative `num` (incl. INT_MIN)
    // skips the loop entirely and IS covered.
    for num in [
        0i32,
        1,
        2,
        3,
        4,
        5,
        7,
        8,
        9,
        10,
        63,
        64,
        65,
        100,
        511,
        512,
        513,
        1000,
        5000,
        20000,
        -1,
        -2,
        -1000,
        i32::MIN,
    ] {
        // Re-seed before each half so both libraries see the same global seed.
        s.seed(0x31415926);
        let cout = capture_stdout(|| unsafe { (s.c.sh_puts)(num) });
        s.seed(0x31415926);
        let rout = capture_stdout(|| unsafe { (s.r.sh_puts)(num) });
        assert_eq!(
            cout,
            rout,
            "C53 sh_puts({num}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            format!("a {num}\n"),
            "C53 sh_puts({num}) content"
        );
    }
    // And the seeds must have advanced identically.
    s.seed(0x31415926);
    for i in 0..10i32 {
        let cout = capture_stdout(|| unsafe { (s.c.sh_puts)(i) });
        let rout = capture_stdout(|| unsafe { (s.r.sh_puts)(i) });
        assert_eq!(cout, rout, "C53 chained sh_puts({i}) without re-seeding");
    }
}

// ---------------------------------------------------------------- C54
#[test]
fn c54_full_string_pipeline() {
    // sh_new_arena / sh_new_strdup / shdefault / shputs / shgets / shdel / shfree
    let s = session(0x31415926);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for elemsize in [16usize, 24] {
            let mut d = Driver::shmode(&s, elemsize, 8, KeyKind::StringAt(0), mode);
            let mut rng = Rng::new(0x54 + mode as u64 * 7 + elemsize as u64);
            // shdefault(t, v)
            d.hmdefault(&format!("C54 m={mode} e={elemsize} default"), &rng.bytes(elemsize));
            d.check(&format!("C54 m={mode} e={elemsize} default"));
            let mut live: Vec<Vec<u8>> = Vec::new();
            for op in 0..300usize {
                let ctx = format!("C54 m={mode} e={elemsize} op#{op}");
                match rng.below(8) {
                    0..=3 => {
                        let k = format!("p{}", rng.below(150)).into_bytes();
                        d.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
                        if !live.contains(&k) {
                            live.push(k);
                        }
                    }
                    4 => {
                        // shputs with a brand-new key (see c26b for existing keys)
                        let k = format!("s{op}").into_bytes();
                        d.shputs(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
                        if !live.contains(&k) {
                            live.push(k);
                        }
                    }
                    5..=6 => {
                        let k = if live.is_empty() {
                            b"none".to_vec()
                        } else {
                            live[rng.below(live.len())].clone()
                        };
                        let i = d.shgeti(&ctx, &k, HM_STRING);
                        if !live.is_empty() {
                            assert!(i >= 0, "{ctx}: live key must be found");
                        }
                    }
                    _ => {
                        if !live.is_empty() {
                            let i = rng.below(live.len());
                            let k = live.remove(i);
                            assert_eq!(d.shdel(&ctx, &k, 0, HM_STRING), 1, "{ctx}");
                        }
                    }
                }
                d.check(&ctx);
            }
            d.check(&format!("C54 m={mode} e={elemsize} final"));
            d.free();
        }
    }
}

// ---------------------------------------------------------------- C55
#[test]
fn c55_full_binary_pipeline() {
    // hmdefault / hmput / hmgeti / hmgeti_ts / hmdel / hmfree
    let s = session(0x31415926);
    for keysize in [1usize, 2, 4, 8, 16] {
        let elemsize = if keysize > 8 { 24 } else { 16 };
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x55 + keysize as u64);
        d.hmdefault(&format!("C55 k={keysize} default"), &rng.bytes(elemsize));
        d.check(&format!("C55 k={keysize} default"));
        let mut live: Vec<Vec<u8>> = Vec::new();
        for op in 0..300usize {
            let ctx = format!("C55 k={keysize} op#{op}");
            match rng.below(8) {
                0..=3 => {
                    let k = rng.bytes(keysize);
                    d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                4..=5 => {
                    let k = if live.is_empty() {
                        rng.bytes(keysize)
                    } else {
                        live[rng.below(live.len())].clone()
                    };
                    d.hmgeti(&ctx, &k, HM_BINARY);
                }
                6 => {
                    let k = rng.bytes(keysize);
                    d.hmgeti_ts(&ctx, &k, HM_BINARY);
                }
                _ => {
                    if !live.is_empty() {
                        let i = rng.below(live.len());
                        let k = live.remove(i);
                        d.hmdel(&ctx, &k, 0, HM_BINARY);
                    }
                }
            }
            d.check(&ctx);
        }
        d.check(&format!("C55 k={keysize} final"));
        d.free();
    }
}

// ---------------------------------------------------------------- C56
#[test]
fn c56_global_seed_stays_in_lockstep() {
    // Create tables alternately in C and Rust WITHOUT re-seeding: both libraries
    // must derive the same table seed and advance their private global
    // `stbds_hash_seed` by exactly the same recurrence.
    let s = session(0x31415926);
    let elemsize = 16usize;
    let mut cseeds = Vec::new();
    let mut rseeds = Vec::new();
    unsafe {
        for _ in 0..64 {
            let cm = (s.c.shmode_func)(elemsize, SH_ARENA);
            let rm = (s.r.shmode_func)(elemsize, SH_ARENA);
            let ct = header(raw_of(cm, elemsize)).hash_table as *mut HashIndex;
            let rt = header(raw_of(rm, elemsize)).hash_table as *mut HashIndex;
            cseeds.push((*ct).seed);
            rseeds.push((*rt).seed);
            (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
            (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
        }
    }
    assert_eq!(cseeds, rseeds, "C56: table seed sequence diverged");
    assert_eq!(cseeds[0], 0x31415926, "C56: first table must use the set seed");
    assert!(
        cseeds.windows(2).all(|w| w[0] != w[1]),
        "C56: the seed must advance between tables"
    );
    // Same for the implicit table created by hmput_key on a NULL map.
    unsafe {
        s.seed(12345);
        let key = 0x11223344u32.to_ne_bytes();
        let kp = key.as_ptr() as *mut c_void;
        for i in 0..16 {
            let cm = (s.c.hmput_key)(std::ptr::null_mut(), elemsize, kp, 4, HM_BINARY);
            let rm = (s.r.hmput_key)(std::ptr::null_mut(), elemsize, kp, 4, HM_BINARY);
            let ct = header(raw_of(cm, elemsize)).hash_table as *mut HashIndex;
            let rt = header(raw_of(rm, elemsize)).hash_table as *mut HashIndex;
            assert_eq!((*ct).seed, (*rt).seed, "C56: implicit table #{i} seed");
            (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
            (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
        }
    }
}
