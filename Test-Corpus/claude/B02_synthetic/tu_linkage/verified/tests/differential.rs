//! C-vs-Rust differential tests driven entirely through the exported C ABI of
//! the two shared objects (see tests/common/mod.rs).
//!
//! * `cfg_*` tests implement the rows of CONFIGS.md (valid paths).
//! * `err_*` tests implement the rows of ERRORS.md (rejections / error codes).
//! * the remaining tests check symbol parity, ABI layout and the harness itself.

#![allow(
    clippy::bool_assert_comparison,
    clippy::same_item_push,
    clippy::manual_repeat_n,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Meta: symbol parity, ABI layout, harness sanity.
// ---------------------------------------------------------------------------

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind == "T" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_parity() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    // sanity: the expected 19 externally linked C functions are all there
    for want in [
        "iv_init",
        "iv_free",
        "iv_reserve",
        "iv_push",
        "iv_pop",
        "iv_peek",
        "prog_init",
        "prog_fetch",
        "vm_init",
        "vm_free",
        "vm_trace",
        "vm_print",
        "run_engine",
        "target",
        "call_a_once",
        "process_a_stream",
        "call_b_once",
        "process_b_stream",
        "main",
    ] {
        assert!(c.contains(&want.to_string()), "C .so lacks {want}");
        assert!(r.contains(&want.to_string()), "Rust .so lacks {want}");
    }
    // and no non-libc undefined symbols on the Rust side
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let name = line.split_whitespace().nth(1).unwrap_or("");
        let base = name.split('@').next().unwrap_or(name);
        let known = base.starts_with('_')
            || matches!(
                base,
                "malloc"
                    | "calloc"
                    | "realloc"
                    | "free"
                    | "posix_memalign"
                    | "strtol"
                    | "strcmp"
                    | "strlen"
                    | "bcmp"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "fgets"
                    | "fputc"
                    | "fprintf"
                    | "printf"
                    | "fwrite"
                    | "stdin"
                    | "stdout"
                    | "stderr"
                    | "abort"
                    | "close"
                    | "open64"
                    | "read"
                    | "write"
                    | "writev"
                    | "lseek64"
                    | "fstat64"
                    | "stat64"
                    | "statx"
                    | "mmap64"
                    | "munmap"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "readlink"
                    | "realpath"
                    | "syscall"
                    | "dl_iterate_phdr"
                    | "pthread_key_create"
                    | "pthread_key_delete"
                    | "pthread_setspecific"
                    | "pthread_getspecific"
            );
        assert!(known, "unexpected undefined (non-libc) symbol: {name}");
    }
}

#[test]
fn abi_layout() {
    // Layout of the harness mirrors must match the C structs; this is implied by
    // every other test passing, but check the sizes explicitly.
    assert_eq!(std::mem::size_of::<IntVec>(), 24);
    assert_eq!(std::mem::align_of::<IntVec>(), 8);
    assert_eq!(std::mem::size_of::<Program>(), 24);
    assert_eq!(std::mem::size_of::<VM>(), 56); // 24 + 24 + 4 + 4 padding
}

#[test]
fn fresh_state_is_independent() {
    // Two independent copies of the same .so must both start from state_a == 0
    // and flipflop == 0, otherwise the whole differential setup would be
    // order-dependent.
    let (c1, r1) = fresh_pair("freshA");
    let (c2, r2) = fresh_pair("freshB");
    // Same first call on two independent copies must give the same answer.
    let a1 = unsafe { (c1.call_a_once)(123) };
    let a2 = unsafe { (c2.call_a_once)(123) };
    assert_eq!(a1, a2, "C statics are not fresh per copy");
    let b1 = unsafe { (r1.call_b_once)(123) };
    let b2 = unsafe { (r2.call_b_once)(123) };
    assert_eq!(b1, b2, "Rust statics are not fresh per copy");
    // C and Rust agree on that same pristine first call.
    let ra1 = unsafe { (r1.call_a_once)(123) };
    assert_eq!(a1, ra1, "pristine call_a_once differs between C and Rust");
    let cb1 = unsafe { (c1.call_b_once)(123) };
    assert_eq!(b1, cb1, "pristine call_b_once differs between C and Rust");
    // The state really does evolve: repeating the same call does not give a
    // constant sequence.  Use a third, untouched pair so both sides see the
    // identical call history.
    let (c3, r3) = fresh_pair("freshC");
    let seq: Vec<i32> = (0..8).map(|_| unsafe { (c3.call_a_once)(123) }).collect();
    let rseq: Vec<i32> = (0..8).map(|_| unsafe { (r3.call_a_once)(123) }).collect();
    assert!(
        seq.iter().any(|v| *v != seq[0]),
        "state_a never changes the result: {seq:?}"
    );
    assert_eq!(seq, rseq, "state evolution differs between C and Rust");
    let _ = (&c2, &r2);
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 1-6: `target` (lib.c)
// ---------------------------------------------------------------------------

#[test]
fn cfg_target_all_branches() {
    let (c, r) = fresh_pair("target");
    let mut probes: Vec<c_int> = Vec::new();
    // every residue class and both signs, exhaustively for a wide window
    for v in -300..=300 {
        probes.push(v);
    }
    for v in [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        7,
        10,
        17,
        2147483640,
        2147483647,
    ] {
        probes.push(v);
    }
    let mut rng = Rng::new(0xA11CE);
    for _ in 0..5000 {
        probes.push(rng.value());
    }
    for v in probes {
        let cv = unsafe { (c.target)(v) };
        let rv = unsafe { (r.target)(v) };
        assert_eq!(cv, rv, "target({v})");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 7-10: `call_a_once` (a.c, stateful)
// ---------------------------------------------------------------------------

#[test]
fn cfg_call_a_once_stateful() {
    let (c, r) = fresh_pair("call_a");
    // first call from pristine state
    assert_eq!(unsafe { (c.call_a_once)(0) }, unsafe {
        (r.call_a_once)(0)
    });
    let mut rng = Rng::new(0xBEEF_1234);
    for i in 0..2000 {
        let x = rng.value();
        let cv = unsafe { (c.call_a_once)(x) };
        let rv = unsafe { (r.call_a_once)(x) };
        assert_eq!(cv, rv, "call_a_once({x}) at iteration {i} (state_a diverged)");
    }
}

#[test]
fn cfg_call_a_once_boundaries() {
    let (c, r) = fresh_pair("call_a_bnd");
    let mut probes: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -2147483647,
        -1073741824,
        -100,
        -7,
        -1,
        0,
        1,
        5,
        7,
        1073741823,
        1073741824,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for k in 0..31 {
        probes.push(1 << k);
        probes.push(-(1 << k));
    }
    for x in probes {
        let cv = unsafe { (c.call_a_once)(x) };
        let rv = unsafe { (r.call_a_once)(x) };
        assert_eq!(cv, rv, "call_a_once({x})");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 12-15: `process_a_stream`
// ---------------------------------------------------------------------------

#[test]
fn cfg_process_a_stream() {
    let (c, r) = fresh_pair("stream_a");
    // single element, chosen to hit continue / break / fall-through
    for v in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 20, 21, 22, -1, -2, -100,
        c_int::MIN,
        c_int::MAX,
    ] {
        let cv = c.stream_a(&[v]);
        let rv = r.stream_a(&[v]);
        assert_eq!(cv, rv, "process_a_stream([{v}])");
    }
    let mut rng = Rng::new(0x5EED_0001);
    for round in 0..400 {
        let n = 2 + rng.below(63) as usize;
        let xs: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
        let cv = c.stream_a(&xs);
        let rv = r.stream_a(&xs);
        assert_eq!(cv, rv, "process_a_stream round {round}, n={n}");
    }
    // long stream: the size_t accumulator grows past INT_MAX before clamping
    let xs: Vec<c_int> = (0..4096).map(|_| rng.value()).collect();
    assert_eq!(c.stream_a(&xs), r.stream_a(&xs), "process_a_stream(4096)");
}

#[test]
fn cfg_a_shared_state() {
    // call_a_once and process_a_stream share `state_a`; interleave them.
    let (c, r) = fresh_pair("a_shared");
    let mut rng = Rng::new(0x1234_5678);
    for i in 0..600 {
        match rng.below(3) {
            0 => {
                let x = rng.value();
                assert_eq!(
                    unsafe { (c.call_a_once)(x) },
                    unsafe { (r.call_a_once)(x) },
                    "step {i}: call_a_once({x})"
                );
            }
            1 => {
                let n = rng.below(6) as usize;
                let xs: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
                assert_eq!(c.stream_a(&xs), r.stream_a(&xs), "step {i}: stream_a({xs:?})");
            }
            _ => {
                let code = rng.program(8);
                let (crc, cs) = c.run(0, &code);
                let (rrc, rs) = r.run(0, &code);
                assert_eq!((crc, &cs), (rrc, &rs), "step {i}: run_engine(0, {code:?})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 16-19: `call_b_once` (b.c, stateful)
// ---------------------------------------------------------------------------

#[test]
fn cfg_call_b_once_stateful() {
    let (c, r) = fresh_pair("call_b");
    assert_eq!(unsafe { (c.call_b_once)(0) }, unsafe {
        (r.call_b_once)(0)
    });
    let mut rng = Rng::new(0xFEED_9876);
    for i in 0..2000 {
        let x = rng.value();
        let cv = unsafe { (c.call_b_once)(x) };
        let rv = unsafe { (r.call_b_once)(x) };
        assert_eq!(cv, rv, "call_b_once({x}) at iteration {i} (flipflop diverged)");
    }
}

#[test]
fn cfg_call_b_once_boundaries() {
    let (c, r) = fresh_pair("call_b_bnd");
    let mut probes: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -2147483640,
        -17,
        -9,
        -1,
        0,
        1,
        8,
        9,
        16,
        c_int::MAX - 9,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for k in 0..31 {
        probes.push(1 << k);
        probes.push(-(1 << k));
    }
    for x in probes {
        let cv = unsafe { (c.call_b_once)(x) };
        let rv = unsafe { (r.call_b_once)(x) };
        assert_eq!(cv, rv, "call_b_once({x})");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 21-24: `process_b_stream`
// ---------------------------------------------------------------------------

#[test]
fn cfg_process_b_stream() {
    let (c, r) = fresh_pair("stream_b");
    for v in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 16, 17, 32, -1, -2, -5, -100,
        c_int::MIN,
        c_int::MIN + 4,
        c_int::MAX,
    ] {
        let cv = c.stream_b(&[v]);
        let rv = r.stream_b(&[v]);
        assert_eq!(cv, rv, "process_b_stream([{v}])");
    }
    let mut rng = Rng::new(0x5EED_0002);
    for round in 0..400 {
        let n = 2 + rng.below(63) as usize;
        let xs: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
        let cv = c.stream_b(&xs);
        let rv = r.stream_b(&xs);
        assert_eq!(cv, rv, "process_b_stream round {round}, n={n}");
    }
    // long stream: acc*3 overflows int over and over
    let xs: Vec<c_int> = (0..1024).map(|_| rng.value()).collect();
    assert_eq!(c.stream_b(&xs), r.stream_b(&xs), "process_b_stream(1024)");
}

#[test]
fn cfg_b_shared_state() {
    let (c, r) = fresh_pair("b_shared");
    let mut rng = Rng::new(0x8765_4321);
    for i in 0..600 {
        match rng.below(3) {
            0 => {
                let x = rng.value();
                assert_eq!(
                    unsafe { (c.call_b_once)(x) },
                    unsafe { (r.call_b_once)(x) },
                    "step {i}: call_b_once({x})"
                );
            }
            1 => {
                let n = rng.below(6) as usize;
                let xs: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
                assert_eq!(c.stream_b(&xs), r.stream_b(&xs), "step {i}: stream_b({xs:?})");
            }
            _ => {
                let code = rng.program(8);
                let (crc, cs) = c.run(1, &code);
                let (rrc, rs) = r.run(1, &code);
                assert_eq!((crc, &cs), (rrc, &rs), "step {i}: run_engine(1, {code:?})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 25-30: IntVec (util.c)
// ---------------------------------------------------------------------------

#[test]
fn cfg_iv_growth_and_reserve() {
    let (c, r) = fresh_pair("iv_growth");
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "after iv_init");

    // 300 pushes: cap must double 0 -> 8 -> 16 -> ... identically.
    let mut rng = Rng::new(0x0011_2233);
    for i in 0..300 {
        let x = rng.value();
        let cok = unsafe { (c.iv_push)(&mut cv, x) };
        let rok = unsafe { (r.iv_push)(&mut rv, x) };
        assert_eq!(cok, rok, "iv_push #{i} return");
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "iv_push #{i} state");
        // peek must agree too
        assert_eq!(
            unsafe { (c.iv_peek)(&cv, -1) },
            unsafe { (r.iv_peek)(&rv, -1) },
            "iv_peek after push #{i}"
        );
    }
    // reserve: no-op, exact, +1, and jumps
    for need in [0usize, 1, 2, 8, 16, 100, 256, 257, 300, 512, 4096] {
        let cok = unsafe { (c.iv_reserve)(&mut cv, need) };
        let rok = unsafe { (r.iv_reserve)(&mut rv, need) };
        assert_eq!(cok, rok, "iv_reserve({need}) return");
        assert_eq!(
            c.snapshot_vec(&cv),
            r.snapshot_vec(&rv),
            "iv_reserve({need}) state"
        );
    }
    // pushing into reserved space must not change cap
    for i in 0..50 {
        assert_eq!(
            unsafe { (c.iv_push)(&mut cv, i) },
            unsafe { (r.iv_push)(&mut rv, i) }
        );
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "push into reserve {i}");
    }
    unsafe { (c.iv_free)(&mut cv) };
    unsafe { (r.iv_free)(&mut rv) };
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "after iv_free");

    // fresh vector: reserve *before* any push decides the first capacity
    for need in [1usize, 7, 8, 9, 17, 33] {
        let mut cv2 = c.new_vec();
        let mut rv2 = r.new_vec();
        let cok = unsafe { (c.iv_reserve)(&mut cv2, need) };
        let rok = unsafe { (r.iv_reserve)(&mut rv2, need) };
        assert_eq!(cok, rok, "fresh iv_reserve({need})");
        assert_eq!(
            c.snapshot_vec(&cv2),
            r.snapshot_vec(&rv2),
            "fresh iv_reserve({need}) state"
        );
        for k in 0..(need as c_int + 3) {
            assert_eq!(
                unsafe { (c.iv_push)(&mut cv2, k) },
                unsafe { (r.iv_push)(&mut rv2, k) }
            );
            assert_eq!(c.snapshot_vec(&cv2), r.snapshot_vec(&rv2));
        }
        unsafe { (c.iv_free)(&mut cv2) };
        unsafe { (r.iv_free)(&mut rv2) };
    }
}

#[test]
fn cfg_iv_pop_peek() {
    let (c, r) = fresh_pair("iv_pop");
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    let mut rng = Rng::new(0x4455_6677);
    let items: Vec<c_int> = (0..40).map(|_| rng.value()).collect();
    for &x in &items {
        unsafe {
            (c.iv_push)(&mut cv, x);
            (r.iv_push)(&mut rv, x);
        }
    }
    // pop everything, alternating out=&x and out=NULL
    for i in 0..45 {
        let mut co: c_int = 0x5A5A_5A5A;
        let mut ro: c_int = 0x5A5A_5A5A;
        let (cok, rok) = if i % 3 == 2 {
            unsafe {
                (
                    (c.iv_pop)(&mut cv, std::ptr::null_mut()),
                    (r.iv_pop)(&mut rv, std::ptr::null_mut()),
                )
            }
        } else {
            unsafe { ((c.iv_pop)(&mut cv, &mut co), (r.iv_pop)(&mut rv, &mut ro)) }
        };
        assert_eq!(cok, rok, "iv_pop #{i} return");
        assert_eq!(co, ro, "iv_pop #{i} out value");
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "iv_pop #{i} state");
        for def in [0, -1, -777, c_int::MIN, c_int::MAX] {
            assert_eq!(
                unsafe { (c.iv_peek)(&cv, def) },
                unsafe { (r.iv_peek)(&rv, def) },
                "iv_peek(def={def}) after pop #{i}"
            );
        }
    }
    unsafe { (c.iv_free)(&mut cv) };
    unsafe { (r.iv_free)(&mut rv) };
}

#[test]
fn cfg_iv_free_reuse() {
    let (c, r) = fresh_pair("iv_reuse");
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    for round in 0..5 {
        for i in 0..(10 * (round + 1)) {
            unsafe {
                (c.iv_push)(&mut cv, i);
                (r.iv_push)(&mut rv, i);
            }
        }
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "round {round} filled");
        unsafe { (c.iv_free)(&mut cv) };
        unsafe { (r.iv_free)(&mut rv) };
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "round {round} freed");
        unsafe { (c.iv_init)(&mut cv) };
        unsafe { (r.iv_init)(&mut rv) };
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "round {round} reinit");
    }
}

#[test]
fn cfg_iv_random_op_sequence() {
    let (c, r) = fresh_pair("iv_random");
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    let mut rng = Rng::new(0x9988_7766);
    for step in 0..5000 {
        match rng.below(10) {
            0..=4 => {
                let x = rng.value();
                assert_eq!(
                    unsafe { (c.iv_push)(&mut cv, x) },
                    unsafe { (r.iv_push)(&mut rv, x) },
                    "step {step}: iv_push({x})"
                );
            }
            5..=6 => {
                let mut co: c_int = -12345;
                let mut ro: c_int = -12345;
                assert_eq!(
                    unsafe { (c.iv_pop)(&mut cv, &mut co) },
                    unsafe { (r.iv_pop)(&mut rv, &mut ro) },
                    "step {step}: iv_pop"
                );
                assert_eq!(co, ro, "step {step}: iv_pop out");
            }
            7 => {
                let def = rng.value();
                assert_eq!(
                    unsafe { (c.iv_peek)(&cv, def) },
                    unsafe { (r.iv_peek)(&rv, def) },
                    "step {step}: iv_peek({def})"
                );
            }
            8 => {
                let need = rng.below(600) as usize;
                assert_eq!(
                    unsafe { (c.iv_reserve)(&mut cv, need) },
                    unsafe { (r.iv_reserve)(&mut rv, need) },
                    "step {step}: iv_reserve({need})"
                );
            }
            _ => {
                if rng.below(4) == 0 {
                    unsafe { (c.iv_free)(&mut cv) };
                    unsafe { (r.iv_free)(&mut rv) };
                } else {
                    let mut co: c_int = 0;
                    let mut ro: c_int = 0;
                    // pop with NULL out on the C side too
                    let cok = unsafe { (c.iv_pop)(&mut cv, std::ptr::null_mut()) };
                    let rok = unsafe { (r.iv_pop)(&mut rv, std::ptr::null_mut()) };
                    assert_eq!(cok, rok, "step {step}: iv_pop(NULL)");
                    let _ = (&mut co, &mut ro);
                }
            }
        }
        assert_eq!(
            c.snapshot_vec(&cv),
            r.snapshot_vec(&rv),
            "state diverged at step {step}"
        );
    }
    unsafe { (c.iv_free)(&mut cv) };
    unsafe { (r.iv_free)(&mut rv) };
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 31: Program / prog_fetch
// ---------------------------------------------------------------------------

#[test]
fn cfg_prog_fetch_sequences() {
    let (c, r) = fresh_pair("prog");
    let mut rng = Rng::new(0x0F0F_0F0F);
    for &n in &[0usize, 1, 2, 3, 37] {
        let code: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
        let ptr = if n == 0 {
            std::ptr::null()
        } else {
            code.as_ptr()
        };
        let mut cp = Program::zeroed();
        let mut rp = Program::zeroed();
        unsafe {
            (c.prog_init)(&mut cp, ptr, n);
            (r.prog_init)(&mut rp, ptr, n);
        }
        assert_eq!(cp.n, rp.n, "prog_init n");
        assert_eq!(cp.ip, rp.ip, "prog_init ip");
        assert_eq!(cp.code, rp.code, "prog_init code");
        for step in 0..(n + 3) {
            let mut co: c_int = -999;
            let mut ro: c_int = -999;
            let cok = unsafe { (c.prog_fetch)(&mut cp, &mut co) };
            let rok = unsafe { (r.prog_fetch)(&mut rp, &mut ro) };
            assert_eq!(cok, rok, "prog_fetch(n={n}) #{step} return");
            assert_eq!(co, ro, "prog_fetch(n={n}) #{step} out");
            assert_eq!(cp.ip, rp.ip, "prog_fetch(n={n}) #{step} ip");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 33-36: VM helpers and vm_print
// ---------------------------------------------------------------------------

#[test]
fn cfg_vm_trace_and_free() {
    let (c, r) = fresh_pair("vm_trace");
    let mut cvm = c.new_vm();
    let mut rvm = r.new_vm();
    assert_eq!(c.snapshot_vm(&cvm), r.snapshot_vm(&rvm), "after vm_init");
    let mut rng = Rng::new(0xDEAD_BEEF);
    for i in 0..500 {
        let t = if i % 5 == 0 { rng.value() } else { (i % 15) as c_int };
        unsafe {
            (c.vm_trace)(&mut cvm, t);
            (r.vm_trace)(&mut rvm, t);
        }
        assert_eq!(
            c.snapshot_vm(&cvm),
            r.snapshot_vm(&rvm),
            "vm_trace #{i} (t={t})"
        );
    }
    // steps is a plain field the caller can also set
    for s in [0, 1, -1, c_int::MIN, c_int::MAX] {
        cvm.steps = s;
        rvm.steps = s;
        assert_eq!(c.snapshot_vm(&cvm), r.snapshot_vm(&rvm), "steps={s}");
    }
    unsafe {
        (c.vm_free)(&mut cvm);
        (r.vm_free)(&mut rvm);
    }
    assert_eq!(c.snapshot_vm(&cvm), r.snapshot_vm(&rvm), "after vm_free");
}

#[test]
fn cfg_vm_print_labels() {
    let (c, r) = fresh_pair("vm_print_lbl");
    let long_label = "L".repeat(300);
    let labels = [
        "",
        "A:",
        "B:",
        "EXT:",
        "%d %s %n",
        "tab\tand\nnewline",
        long_label.as_str(),
    ];
    let stacks: [&[c_int]; 4] = [
        &[],
        &[0],
        &[c_int::MIN, c_int::MAX],
        &[1, 2, 3, 4, 5, -6, -7],
    ];
    let steps = [0, 1, 42, -1, c_int::MIN, c_int::MAX];
    for label in labels {
        for stack in stacks {
            for &s in &steps {
                let co = c.print_vm(label, stack, &[0, 1, 2, 13, 14], s);
                let ro = r.print_vm(label, stack, &[0, 1, 2, 13, 14], s);
                assert_eq!(
                    co,
                    ro,
                    "vm_print(label={label:?}, stack={stack:?}, steps={s})\n C: {}\n R: {}",
                    String::from_utf8_lossy(&co),
                    String::from_utf8_lossy(&ro)
                );
            }
        }
    }
}

#[test]
fn cfg_vm_print_trace_alphabet() {
    let (c, r) = fresh_pair("vm_print_tr");
    // the values the engine can actually emit
    let engine_traces: Vec<c_int> = (0..=14).collect();
    let co = c.print_vm("A:", &[7], &engine_traces, 15);
    let ro = r.print_vm("A:", &[7], &engine_traces, 15);
    assert_eq!(co, ro, "engine trace alphabet");
    assert_eq!(
        String::from_utf8_lossy(&co).trim_end().to_string(),
        String::from_utf8_lossy(&ro).trim_end().to_string()
    );
    // arbitrary ints (t & 25 indexes the 26 letter table; negative included)
    let mut rng = Rng::new(0xCAFE_F00D);
    for round in 0..200 {
        let n = rng.below(40) as usize;
        let tr: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
        let co = c.print_vm("T:", &[1, 2], &tr, round as c_int);
        let ro = r.print_vm("T:", &[1, 2], &tr, round as c_int);
        assert_eq!(co, ro, "vm_print trace {tr:?}");
    }
    // extremes and a long trace
    let tr: Vec<c_int> = vec![c_int::MIN, c_int::MAX, -1, -25, -26, 25, 26, 51, 0];
    assert_eq!(
        c.print_vm("X:", &[], &tr, 3),
        r.print_vm("X:", &[], &tr, 3),
        "vm_print extreme trace values"
    );
    let long: Vec<c_int> = (0..4000).map(|i| i % 15).collect();
    assert_eq!(
        c.print_vm("LONG:", &[9], &long, 4000),
        r.print_vm("LONG:", &[9], &long, 4000),
        "vm_print long trace"
    );
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 37-55: run_engine (engine.c)
// ---------------------------------------------------------------------------

/// The three "documented" implementations plus the values that fall into the
/// `else` branch of `classify`/`process_stream`.
const IMPLS: [c_int; 3] = [0, 1, 2];

fn engine_same(c: &Api, r: &Api, impl_id: c_int, code: &[c_int], ctx: &str) {
    let (crc, cs) = c.run(impl_id, code);
    let (rrc, rs) = r.run(impl_id, code);
    assert_eq!(
        crc, rrc,
        "{ctx}: run_engine rc differs (impl={impl_id}, code={code:?})"
    );
    assert_eq!(
        cs, rs,
        "{ctx}: VM state differs (impl={impl_id}, code={code:?})"
    );
}

/// Hand written programs, one (or more) per opcode.
fn opcode_programs() -> Vec<(&'static str, Vec<c_int>)> {
    vec![
        ("empty", vec![]),
        ("op0 push", vec![0, 42]),
        ("op0 push min", vec![0, c_int::MIN]),
        ("op0 push max", vec![0, c_int::MAX]),
        ("op0 twice", vec![0, 1, 0, 2]),
        ("op1 add", vec![0, 3, 0, 4, 1]),
        ("op1 add overflow", vec![0, c_int::MAX, 0, c_int::MAX, 1]),
        ("op1 add min", vec![0, c_int::MIN, 0, c_int::MIN, 1]),
        ("op2 mul", vec![0, 6, 0, 7, 2]),
        ("op2 mul overflow", vec![0, 65536, 0, 65536, 2]),
        ("op2 mul neg", vec![0, -3, 0, 1000000007, 2]),
        ("op3 dup empty", vec![3]),
        ("op3 dup", vec![0, 9, 3, 3]),
        ("op4 drop", vec![0, 9, 4]),
        ("op4 drop twice", vec![0, 9, 0, 8, 4, 4]),
        ("op5 empty stack", vec![5]),
        ("op5", vec![0, 7, 5]),
        ("op5 neg", vec![0, -7, 5]),
        ("op5 repeated", vec![0, 3, 5, 5, 5, 5]),
        ("op6 cond false", vec![0, 0, 6, 2, 0, 7, 0, 8]),
        ("op6 cond true k0", vec![0, 1, 6, 0, 0, 8]),
        ("op6 cond true k2", vec![0, 1, 6, 2, 0, 7, 0, 8]),
        ("op6 cond true k boundary", vec![0, 1, 6, 4, 0, 7, 0, 8]),
        ("op7 times0", vec![7, 0, 3]),
        ("op7 times1 dup", vec![0, 5, 7, 1, 3]),
        ("op7 times3 dup", vec![0, 5, 7, 3, 3]),
        ("op7 times3 classify", vec![0, 6, 7, 3, 5]),
        ("op7 inner halt", vec![7, 2, 10]),
        ("op7 inner unknown", vec![7, 2, 11]),
        ("op7 inner add fails", vec![7, 2, 1]),
        ("op7 nested", vec![7, 2, 7]),
        ("op7 negative times", vec![0, 4, 7, -5, 3]),
        ("op7 then more", vec![0, 5, 7, 2, 3, 0, 8, 1]),
        ("op8 empty", vec![8]),
        ("op8", vec![0, 12345, 8]),
        ("op8 chain", vec![0, 12345, 8, 8, 8]),
        ("op9 m0", vec![9, 0]),
        ("op9 m1", vec![0, 5, 9, 1]),
        ("op9 m2 of 2", vec![0, 5, 0, 6, 9, 2]),
        ("op9 m2 of 4", vec![0, 1, 0, 2, 0, 3, 0, 4, 9, 2]),
        ("op9 m4 of 4", vec![0, 1, 0, 2, 0, 3, 0, 4, 9, 4]),
        ("op9 m3 of 4", vec![0, 1, 0, 2, 0, 3, 0, 4, 9, 3]),
        ("op9 extremes", vec![0, c_int::MAX, 0, c_int::MIN, 9, 2]),
        ("op9 then op9", vec![0, 5, 3, 3, 3, 9, 2, 9, 2]),
        ("op10 halt", vec![10]),
        ("op10 halt mid", vec![0, 5, 10, 0, 6, 1]),
        ("mixed", vec![0, 100, 3, 3, 3, 3, 9, 3, 9, 2, 5, 8, 1, 2, 10]),
        ("long mixed", vec![0, 3, 0, 4, 1, 3, 5, 8, 3, 9, 2, 2, 5, 3, 4, 8, 1]),
    ]
}

#[test]
fn cfg_engine_opcodes_per_impl() {
    for &imp in IMPLS.iter() {
        // one fresh pair per impl so the static state history is identical
        let (c, r) = fresh_pair(&format!("ops{imp}"));
        for (name, code) in opcode_programs() {
            engine_same(&c, &r, imp, &code, name);
        }
    }
}

#[test]
fn cfg_engine_classify_buckets() {
    // Drive opcode 5 / 8 with a wide range of stack values so that every
    // `bucket` case (0,1,2,3/4 fall-through, default) is hit for every impl.
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("buckets{imp}"));
        for x in -60..=60 {
            engine_same(&c, &r, imp, &[0, x, 5], "op5 sweep");
            engine_same(&c, &r, imp, &[0, x, 8], "op8 sweep");
            engine_same(&c, &r, imp, &[0, x, 5, 5, 8, 8], "op5/8 chain sweep");
        }
        for x in [c_int::MIN, c_int::MIN + 1, -1, 0, 1, c_int::MAX - 1, c_int::MAX] {
            engine_same(&c, &r, imp, &[0, x, 5, 8], "op5/8 boundary");
        }
    }
}

#[test]
fn cfg_engine_op6_jumps() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("jump{imp}"));
        for cond in [0, 1, -1, c_int::MIN, c_int::MAX] {
            for k in [0, 1, 2, 3, 4, 5, 6] {
                let code = vec![0, cond, 6, k, 0, 7, 3, 5, 8, 4];
                engine_same(&c, &r, imp, &code, "op6 grid");
            }
        }
        // jump landing exactly on the last word, and one past it
        for k in 0..6 {
            engine_same(&c, &r, imp, &[0, 1, 6, k, 3, 3, 3], "op6 tail");
        }
        // two jumps in a row
        engine_same(&c, &r, imp, &[0, 1, 6, 2, 0, 9, 0, 1, 6, 0, 3], "op6 twice");
    }
}

#[test]
fn cfg_engine_op7_repeat() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("repeat{imp}"));
        let inner_ops: [c_int; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        for times in [-3, -1, 0, 1, 2, 3, 5] {
            for inner in inner_ops {
                // with a couple of values on the stack first
                let code = vec![0, 6, 0, 7, 7, times, inner];
                engine_same(&c, &r, imp, &code, "op7 grid (with stack)");
                // and with an empty stack
                let code = vec![7, times, inner];
                engine_same(&c, &r, imp, &code, "op7 grid (empty stack)");
                // and with trailing instructions after the repeat window
                let code = vec![0, 3, 7, times, inner, 3, 8, 1];
                engine_same(&c, &r, imp, &code, "op7 grid (with tail)");
            }
        }
    }
}

#[test]
fn cfg_engine_op9_stream_double_pop() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("stream{imp}"));
        let mut rng = Rng::new(0x5150_5150 + imp as u64);
        // build stacks of every size 0..=8 and every m in 0..=size
        for size in 0usize..=8 {
            for m in 0..=(size as c_int) {
                let mut code: Vec<c_int> = Vec::new();
                for _ in 0..size {
                    code.push(0);
                    code.push(rng.value());
                }
                code.push(9);
                code.push(m);
                engine_same(&c, &r, imp, &code, "op9 size/m grid");
            }
        }
        // repeated stream ops so the second pop round sometimes underflows
        for _ in 0..200 {
            let size = rng.below(6) as usize;
            let mut code: Vec<c_int> = Vec::new();
            for _ in 0..size {
                code.push(0);
                code.push(rng.value());
            }
            code.push(9);
            code.push(rng.below(size as u64 + 1) as c_int);
            code.push(9);
            code.push(rng.below(2) as c_int);
            engine_same(&c, &r, imp, &code, "op9 chained");
        }
    }
}

#[test]
fn cfg_engine_random_programs() {
    let (c, r) = fresh_pair("rand_prog");
    let mut rng = Rng::new(0x1357_9BDF);
    for i in 0..1200 {
        let code = rng.program(24);
        for &imp in IMPLS.iter() {
            engine_same(&c, &r, imp, &code, &format!("random #{i}"));
        }
    }
}

#[test]
fn cfg_engine_long_programs() {
    let (c, r) = fresh_pair("long_prog");
    let mut rng = Rng::new(0x2468_ACE0);
    for i in 0..120 {
        let code = rng.program(120);
        for &imp in IMPLS.iter() {
            engine_same(&c, &r, imp, &code, &format!("long random #{i}"));
        }
    }
}

#[test]
fn cfg_engine_random_garbage() {
    // completely unbiased random ints: mostly the `default: return 99` path
    let (c, r) = fresh_pair("garbage");
    let mut rng = Rng::new(0xF0F0_1234);
    for i in 0..600 {
        let n = 1 + rng.below(10) as usize;
        let code: Vec<c_int> = (0..n).map(|_| rng.value()).collect();
        for &imp in IMPLS.iter() {
            engine_same(&c, &r, imp, &code, &format!("garbage #{i}"));
        }
    }
}

#[test]
fn cfg_engine_vm_reuse() {
    // The API allows calling run_engine repeatedly on the same VM; stack, trace
    // and steps then accumulate (this is also what opcode 7 does internally).
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("reuse{imp}"));
        let mut cvm = c.new_vm();
        let mut rvm = r.new_vm();
        let mut rng = Rng::new(0xABCD_0000 + imp as u64);
        for i in 0..300 {
            let code = rng.program(10);
            let ptr = if code.is_empty() {
                std::ptr::null()
            } else {
                code.as_ptr()
            };
            let crc = unsafe { (c.run_engine)(imp, ptr, code.len(), &mut cvm) };
            let rrc = unsafe { (r.run_engine)(imp, ptr, code.len(), &mut rvm) };
            assert_eq!(crc, rrc, "reuse #{i}: rc (impl={imp}, code={code:?})");
            assert_eq!(
                c.snapshot_vm(&cvm),
                r.snapshot_vm(&rvm),
                "reuse #{i}: VM state (impl={imp}, code={code:?})"
            );
            // and the printed form must match byte for byte
            assert_eq!(
                c.print_vm_raw("R:", &cvm),
                r.print_vm_raw("R:", &rvm),
                "reuse #{i}: vm_print"
            );
        }
        unsafe {
            (c.vm_free)(&mut cvm);
            (r.vm_free)(&mut rvm);
        }
    }
}

#[test]
fn cfg_engine_impl_id_variants() {
    // Every impl_id outside {0,1} must behave exactly like impl_id == 2.
    let (c, r) = fresh_pair("impl_variants");
    let mut rng = Rng::new(0x0BAD_BEEF);
    let variants: [c_int; 8] = [2, 3, 4, 7, 99, -1, c_int::MIN, c_int::MAX];
    for i in 0..300 {
        let code = rng.program(16);
        let reference = c.run(2, &code);
        for &imp in variants.iter() {
            let cres = c.run(imp, &code);
            let rres = r.run(imp, &code);
            assert_eq!(cres.0, rres.0, "impl {imp} rc #{i} code={code:?}");
            assert_eq!(cres.1, rres.1, "impl {imp} state #{i} code={code:?}");
            assert_eq!(
                cres, reference,
                "C: impl {imp} should behave like impl 2 (code={code:?})"
            );
        }
    }
}

#[test]
fn cfg_engine_state_across_runs() {
    // Repeating the *same* program on the same library instance can give a
    // different answer every time, because `state_a` / `flipflop` survive.  Both
    // implementations must evolve identically.  (Some programs are fixed points
    // of the state update -- e.g. every `call_b_once` toggles `flipflop` an even
    // number of times -- hence the "at least one program varies" form.)
    let programs: Vec<Vec<c_int>> = vec![
        vec![0, 37, 5, 8, 3, 5, 9, 2, 8, 1],
        vec![3, 5, 5],
        vec![0, 9, 8, 8, 8, 8],
        vec![0, 5, 9, 1, 5, 8],
        vec![0, -3, 5, 5, 9, 1],
        vec![0, 6, 9, 1, 8, 5, 3, 9, 2],
    ];
    for &imp in &[0 as c_int, 1, 2] {
        let mut varying = 0;
        for (pi, code) in programs.iter().enumerate() {
            let (c, r) = fresh_pair(&format!("state_runs{imp}_{pi}"));
            let mut seen = std::collections::HashSet::new();
            for i in 0..200 {
                let (crc, cs) = c.run(imp, code);
                let (rrc, rs) = r.run(imp, code);
                assert_eq!(crc, rrc, "impl {imp} prog {pi} run #{i}: rc");
                assert_eq!(cs, rs, "impl {imp} prog {pi} run #{i}: state");
                seen.insert((cs.stack.clone(), cs.trace.clone()));
            }
            if seen.len() > 1 {
                varying += 1;
            }
            // impl 2 uses the stateless lib.c `target`, and impl 1's `flipflop`
            // provably returns to 0 at the end of every public call (each
            // `call_b_once` performs 4 toggles, and each `process_b_stream`
            // element performs an even number), so both are run-to-run
            // deterministic.  impl 0's `state_a` is not.
            if imp != 0 {
                assert_eq!(
                    seen.len(),
                    1,
                    "impl {imp} must be run-to-run deterministic (prog {pi})"
                );
            }
        }
        if imp == 0 {
            assert!(
                varying > 0,
                "impl 0: no program was affected by the persistent `state_a`"
            );
        }
    }
}

#[test]
fn cfg_engine_pipeline_like_main() {
    // Exactly what main() does: three VMs, impl 0/1/2, then vm_print each.
    let (c, r) = fresh_pair("pipeline");
    let mut rng = Rng::new(0x600D_F00D);
    for i in 0..400 {
        let code = rng.program(20);
        let ptr = if code.is_empty() {
            std::ptr::null()
        } else {
            code.as_ptr()
        };
        let mut cvms = [c.new_vm(), c.new_vm(), c.new_vm()];
        let mut rvms = [r.new_vm(), r.new_vm(), r.new_vm()];
        let mut crcs = [0; 3];
        let mut rrcs = [0; 3];
        for k in 0..3 {
            crcs[k] = unsafe { (c.run_engine)(k as c_int, ptr, code.len(), &mut cvms[k]) };
        }
        for k in 0..3 {
            rrcs[k] = unsafe { (r.run_engine)(k as c_int, ptr, code.len(), &mut rvms[k]) };
        }
        assert_eq!(crcs, rrcs, "pipeline #{i}: rcs (code={code:?})");
        for (k, label) in ["A:", "B:", "EXT:"].iter().enumerate() {
            assert_eq!(
                c.snapshot_vm(&cvms[k]),
                r.snapshot_vm(&rvms[k]),
                "pipeline #{i}: vm {label} state (code={code:?})"
            );
            assert_eq!(
                c.print_vm_raw(label, &cvms[k]),
                r.print_vm_raw(label, &rvms[k]),
                "pipeline #{i}: vm_print {label} (code={code:?})"
            );
        }
        for k in 0..3 {
            unsafe {
                (c.vm_free)(&mut cvms[k]);
                (r.vm_free)(&mut rvms[k]);
            }
        }
    }
}

// ===========================================================================
// Phase C -- ERRORS.md rows.  Each test constructs the exact invalid condition
// and asserts that C and Rust return the *same* error code / sentinel (and that
// it is the code the C source documents).
// ===========================================================================

/// ERRORS.md rows 1-19 (all `run_engine` error codes) in one table.
#[test]
fn err_engine_codes() {
    #[rustfmt::skip]
    let cases: Vec<(&str, Vec<c_int>, c_int)> = vec![
        // row 19: empty program
        ("empty program",                vec![],                       0),
        // row 1: op0 without immediate
        ("op0 missing immediate",        vec![0],                      1),
        ("op0 missing immediate late",   vec![0, 1, 3, 0],             1),
        // rows 2-3: op1 with 0 / 1 operands
        ("op1 empty stack",              vec![1],                      2),
        ("op1 one operand",              vec![0, 5, 1],                2),
        // rows 4-5: op2 with 0 / 1 operands
        ("op2 empty stack",              vec![2],                      3),
        ("op2 one operand",              vec![0, 5, 2],                3),
        // row 6: op4 on empty stack
        ("op4 empty stack",              vec![4],                      4),
        ("op4 after drop",               vec![0, 5, 4, 4],             4),
        // row 7: op6 without k
        ("op6 missing k",                vec![6],                      5),
        ("op6 missing k late",           vec![0, 1, 6],                5),
        // row 8: op6 without condition on the stack
        ("op6 empty stack",              vec![6, 1],                   6),
        // rows 9-10: op6 jumping too far / negative distance
        ("op6 k too far",                vec![0, 1, 6, 99],            7),
        ("op6 k one past end",           vec![0, 1, 6, 5, 3, 3, 3],    7),
        ("op6 k negative",               vec![0, 1, 6, -1, 0, 5],      7),
        ("op6 k INT_MIN",                vec![0, 1, 6, c_int::MIN, 3], 7),
        ("op6 k INT_MAX",                vec![0, 1, 6, c_int::MAX, 3], 7),
        // row 11: op7 without times
        ("op7 missing times",            vec![7],                      8),
        ("op7 missing times late",       vec![0, 1, 7],                8),
        // row 12: op7 with nothing to repeat
        ("op7 nothing to repeat",        vec![7, 3],                   9),
        ("op7 nothing to repeat 0",      vec![7, 0],                   9),
        ("op7 nothing to repeat neg",    vec![7, -4],                  9),
        // row 13: op9 without m
        ("op9 missing m",                vec![9],                      10),
        ("op9 missing m late",           vec![0, 1, 9],                10),
        // row 14: op9 negative m
        ("op9 m negative",               vec![9, -1],                  11),
        ("op9 m INT_MIN",                vec![9, c_int::MIN],          11),
        ("op9 m negative with stack",    vec![0, 1, 0, 2, 9, -2],      11),
        // row 15: op9 m greater than the stack depth
        ("op9 m > len (empty)",          vec![9, 1],                   11),
        ("op9 m > len",                  vec![0, 1, 9, 2],             11),
        ("op9 m INT_MAX",                vec![0, 1, 9, c_int::MAX],    11),
        // rows 16-17: unknown opcodes
        ("op 11",                        vec![11],                     99),
        ("op 12",                        vec![12],                     99),
        ("op 100",                       vec![100],                    99),
        ("op -1",                        vec![-1],                     99),
        ("op INT_MIN",                   vec![c_int::MIN],             99),
        ("op INT_MAX",                   vec![c_int::MAX],             99),
        ("unknown op after work",        vec![0, 5, 3, 1, 42],         99),
        // row 18: op10 halts successfully
        ("op10 halt",                    vec![10],                     0),
        ("op10 halt mid program",        vec![0, 5, 10, 11, 12],       0),
    ];

    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("errcodes{imp}"));
        for (name, code, expect) in &cases {
            let (crc, cs) = c.run(imp, code);
            let (rrc, rs) = r.run(imp, code);
            assert_eq!(
                crc, *expect,
                "C rc for {name} (impl={imp}, code={code:?}) is not the documented {expect}"
            );
            assert_eq!(
                rrc, *expect,
                "Rust rc for {name} (impl={imp}, code={code:?}) differs from C's {expect}"
            );
            assert_eq!(cs, rs, "VM state after {name} (impl={imp}) differs");
        }
    }
}

/// ERRORS.md row 10 -- negative jump distances, exhaustively around the boundary.
#[test]
fn err_jump_negative() {
    let (c, r) = fresh_pair("jump_neg");
    for &imp in IMPLS.iter() {
        for k in [-1, -2, -3, -1000, c_int::MIN, c_int::MIN + 1] {
            for tail in 0..4 {
                let mut code = vec![0, 1, 6, k];
                for _ in 0..tail {
                    code.push(3);
                }
                let (crc, cs) = c.run(imp, &code);
                let (rrc, rs) = r.run(imp, &code);
                assert_eq!(crc, 7, "C must reject k={k} with 7");
                assert_eq!(rrc, 7, "Rust must reject k={k} with 7");
                assert_eq!(cs, rs);
            }
        }
        // and the exact boundary: k == n - ip is accepted, k == n - ip + 1 is not
        let (crc, _) = c.run(imp, &[0, 1, 6, 3, 3, 3, 3]);
        let (rrc, _) = r.run(imp, &[0, 1, 6, 3, 3, 3, 3]);
        assert_eq!((crc, rrc), (0, 0), "k == n-ip must be accepted");
        let (crc, _) = c.run(imp, &[0, 1, 6, 4, 3, 3, 3]);
        let (rrc, _) = r.run(imp, &[0, 1, 6, 4, 3, 3, 3]);
        assert_eq!((crc, rrc), (7, 7), "k == n-ip+1 must be rejected");
    }
}

/// ERRORS.md rows 14-15 -- the `m` range check of opcode 9.
#[test]
fn err_stream_m_range() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("m_range{imp}"));
        for depth in 0usize..=4 {
            for m in -3..=6 {
                let mut code: Vec<c_int> = Vec::new();
                for k in 0..depth {
                    code.push(0);
                    code.push(k as c_int + 1);
                }
                code.push(9);
                code.push(m);
                let expect = if m < 0 || (m as usize) > depth { 11 } else { 0 };
                let (crc, cs) = c.run(imp, &code);
                let (rrc, rs) = r.run(imp, &code);
                assert_eq!(crc, expect, "C: depth={depth} m={m}");
                assert_eq!(rrc, expect, "Rust: depth={depth} m={m}");
                assert_eq!(cs, rs, "state: depth={depth} m={m}");
            }
        }
    }
}

/// ERRORS.md rows 16-17 -- every opcode outside 0..=10 must yield 99.
#[test]
fn err_opcode_out_of_range() {
    let (c, r) = fresh_pair("op_range");
    let mut rng = Rng::new(0x00FF_00FF);
    for &imp in IMPLS.iter() {
        for op in -40..=40 {
            let code = vec![op, 3, 4];
            let (crc, cs) = c.run(imp, &code);
            let (rrc, rs) = r.run(imp, &code);
            assert_eq!(crc, rrc, "rc for opcode {op}");
            assert_eq!(cs, rs, "state for opcode {op}");
            if !(0..=10).contains(&op) {
                assert_eq!(crc, 99, "C: opcode {op} must be rejected with 99");
            }
        }
        for _ in 0..500 {
            let op = rng.next_i32();
            let code = vec![op];
            let (crc, cs) = c.run(imp, &code);
            let (rrc, rs) = r.run(imp, &code);
            assert_eq!(crc, rrc, "rc for random opcode {op}");
            assert_eq!(cs, rs, "state for random opcode {op}");
        }
    }
}

/// ERRORS.md rows 19/33/35 -- zero length inputs everywhere.
#[test]
fn err_zero_len() {
    let (c, r) = fresh_pair("zero_len");
    for &imp in [0, 1, 2, -1, 7, c_int::MIN, c_int::MAX].iter() {
        let (crc, cs) = c.run(imp, &[]);
        let (rrc, rs) = r.run(imp, &[]);
        assert_eq!((crc, rrc), (0, 0), "empty program must return 0");
        assert_eq!(cs, rs);
        assert!(cs.stack.is_empty() && cs.trace.is_empty() && cs.steps == 0);
    }
    assert_eq!(c.stream_a(&[]), r.stream_a(&[]));
    assert_eq!(c.stream_a(&[]), c_int::MIN, "process_a_stream(n=0) is INT_MIN");
    assert_eq!(c.stream_b(&[]), r.stream_b(&[]));
    assert_eq!(c.stream_b(&[]), 1, "process_b_stream(n=0) is 1");
}

/// ERRORS.md row G1 -- NULL code pointer with n == 0.
#[test]
fn err_null_code_zero_len() {
    let (c, r) = fresh_pair("null_code");
    for &imp in IMPLS.iter() {
        let (crc, cs) = c.run_raw(imp, std::ptr::null(), 0);
        let (rrc, rs) = r.run_raw(imp, std::ptr::null(), 0);
        assert_eq!((crc, rrc), (0, 0));
        assert_eq!(cs, rs);
    }
}

/// ERRORS.md row G2 -- NULL stream pointer with n == 0.
#[test]
fn err_null_ptr_zero_len() {
    let (c, r) = fresh_pair("null_stream");
    let ca = unsafe { (c.process_a_stream)(std::ptr::null(), 0) };
    let ra = unsafe { (r.process_a_stream)(std::ptr::null(), 0) };
    assert_eq!(ca, ra);
    assert_eq!(ca, c_int::MIN);
    let cb = unsafe { (c.process_b_stream)(std::ptr::null(), 0) };
    let rb = unsafe { (r.process_b_stream)(std::ptr::null(), 0) };
    assert_eq!(cb, rb);
    assert_eq!(cb, 1);
}

/// ERRORS.md row 20 -- a failing inner instruction inside opcode 7 is swallowed
/// (trace 12, outer loop continues, final rc 0).
#[test]
fn err_repeat_inner_failure() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("inner_fail{imp}"));
        // inner opcodes that always fail on a 1-word program
        for inner in [0, 1, 2, 4, 6, 7, 9, 11, -1, c_int::MAX] {
            for times in [1, 2, 3] {
                let code = vec![7, times, inner];
                let (crc, cs) = c.run(imp, &code);
                let (rrc, rs) = r.run(imp, &code);
                assert_eq!(
                    crc, 0,
                    "C: inner failure must not propagate (inner={inner}, times={times})"
                );
                assert_eq!(rrc, 0, "Rust: inner failure must not propagate");
                assert_eq!(cs, rs, "state (inner={inner}, times={times})");
                assert!(
                    cs.trace.contains(&12),
                    "trace should record 12 for inner={inner}: {:?}",
                    cs.trace
                );
            }
        }
        // inner opcode 3 (dup) succeeds -> no 12 in the trace
        let (crc, cs) = c.run(imp, &[7, 3, 3]);
        let (rrc, rs) = r.run(imp, &[7, 3, 3]);
        assert_eq!((crc, rrc), (0, 0));
        assert_eq!(cs, rs);
        assert!(!cs.trace.contains(&12), "trace: {:?}", cs.trace);
    }
}

/// ERRORS.md rows 21-23 -- `iv_reserve` rejections.
#[test]
fn err_iv_reserve() {
    let (c, r) = fresh_pair("reserve_err");

    // row 21: need <= cap is a no-op success
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    for x in 0..10 {
        unsafe {
            (c.iv_push)(&mut cv, x);
            (r.iv_push)(&mut rv, x);
        }
    }
    let before_c = c.snapshot_vec(&cv);
    for need in [0usize, 1, 5, 10, 16] {
        assert!(unsafe { (c.iv_reserve)(&mut cv, need) });
        assert!(unsafe { (r.iv_reserve)(&mut rv, need) });
        assert_eq!(c.snapshot_vec(&cv), before_c, "C changed for need={need}");
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));
    }

    // row 22: the SIZE_MAX/2 guard -- returns false without calling realloc
    for need in [usize::MAX, usize::MAX - 1, (1usize << 63) + 1, 3usize << 62] {
        let cok = unsafe { (c.iv_reserve)(&mut cv, need) };
        let rok = unsafe { (r.iv_reserve)(&mut rv, need) };
        assert_eq!(cok, false, "C must reject need={need:#x}");
        assert_eq!(rok, false, "Rust must reject need={need:#x}");
        assert_eq!(c.snapshot_vec(&cv), before_c, "vector must be untouched");
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));
    }

    // row 23: realloc failure (8 EiB request).  NB: `need == 1<<62` is *not*
    // used here -- see err_iv_reserve_size_wrap for why.
    for need in [1usize << 61, (1usize << 61) - 1, (1usize << 60) + 1] {
        let cok = unsafe { (c.iv_reserve)(&mut cv, need) };
        let rok = unsafe { (r.iv_reserve)(&mut rv, need) };
        assert_eq!(cok, rok, "iv_reserve({need:#x})");
        assert_eq!(cok, false, "allocation of {need:#x} ints must fail");
        assert_eq!(c.snapshot_vec(&cv), before_c, "vector must be untouched");
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));
    }
    unsafe {
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
    }

    // same on a pristine vector (cap == 0 -> nc starts at 8)
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    for need in [usize::MAX, 1usize << 61] {
        assert_eq!(
            unsafe { (c.iv_reserve)(&mut cv, need) },
            unsafe { (r.iv_reserve)(&mut rv, need) },
            "pristine iv_reserve({need:#x})"
        );
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));
        assert!(cv.data.is_null(), "C must not have allocated");
        assert!(rv.data.is_null(), "Rust must not have allocated");
    }
}

/// ERRORS.md row 24 -- `iv_push` when the implied `iv_reserve` fails.
#[test]
fn err_iv_push_reserve_fail() {
    let (c, r) = fresh_pair("push_fail");
    // A vector claiming len == cap == 2^60 with a NULL buffer: iv_push must ask
    // iv_reserve for 2^61 ints (8 EiB), which fails, so nothing is dereferenced.
    let big = 1usize << 60;
    let mut cv = IntVec {
        data: std::ptr::null_mut(),
        len: big,
        cap: big,
    };
    let mut rv = IntVec {
        data: std::ptr::null_mut(),
        len: big,
        cap: big,
    };
    let cok = unsafe { (c.iv_push)(&mut cv, 7) };
    let rok = unsafe { (r.iv_push)(&mut rv, 7) };
    assert_eq!(cok, false, "C iv_push must fail");
    assert_eq!(rok, false, "Rust iv_push must fail");
    assert_eq!(cv.len, big);
    assert_eq!(rv.len, big);
    assert_eq!(cv.cap, big);
    assert_eq!(rv.cap, big);
    assert!(cv.data.is_null() && rv.data.is_null());
}

/// ERRORS.md rows 25-26 -- `iv_pop` on empty, and with a NULL out pointer.
#[test]
fn err_iv_pop() {
    let (c, r) = fresh_pair("pop_err");
    // zero-initialised (never even iv_init'ed) vector
    let mut cv = IntVec::zeroed();
    let mut rv = IntVec::zeroed();
    let mut co: c_int = 0x1234_5678;
    let mut ro: c_int = 0x1234_5678;
    assert_eq!(unsafe { (c.iv_pop)(&mut cv, &mut co) }, false);
    assert_eq!(unsafe { (r.iv_pop)(&mut rv, &mut ro) }, false);
    assert_eq!(co, 0x1234_5678, "C must not write *out on failure");
    assert_eq!(ro, 0x1234_5678, "Rust must not write *out on failure");
    assert_eq!(unsafe { (c.iv_pop)(&mut cv, std::ptr::null_mut()) }, false);
    assert_eq!(unsafe { (r.iv_pop)(&mut rv, std::ptr::null_mut()) }, false);
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));

    // NULL out on a non-empty vector: succeeds, len drops, no store
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    for x in 0..4 {
        unsafe {
            (c.iv_push)(&mut cv, x);
            (r.iv_push)(&mut rv, x);
        }
    }
    for i in 0..6 {
        let cok = unsafe { (c.iv_pop)(&mut cv, std::ptr::null_mut()) };
        let rok = unsafe { (r.iv_pop)(&mut rv, std::ptr::null_mut()) };
        assert_eq!(cok, rok, "iv_pop(NULL) #{i}");
        assert_eq!(cok, i < 4, "iv_pop(NULL) #{i} expected success={}", i < 4);
        assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "after pop #{i}");
    }
    unsafe {
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
    }
}

/// ERRORS.md row 27 -- `iv_peek` default substitution.
#[test]
fn err_iv_peek_default() {
    let (c, r) = fresh_pair("peek_def");
    let cv = IntVec::zeroed();
    let rv = IntVec::zeroed();
    let mut rng = Rng::new(0xDEFA_1234);
    let mut defs: Vec<c_int> = vec![0, -1, -777, 1, c_int::MIN, c_int::MAX];
    for _ in 0..200 {
        defs.push(rng.value());
    }
    for def in defs {
        let cvv = unsafe { (c.iv_peek)(&cv, def) };
        let rvv = unsafe { (r.iv_peek)(&rv, def) };
        assert_eq!(cvv, def, "C must return the default verbatim");
        assert_eq!(rvv, def, "Rust must return the default verbatim");
    }
}

/// ERRORS.md rows 28-29 -- `prog_fetch` exhaustion, including a caller-mangled ip.
#[test]
fn err_prog_fetch() {
    let (c, r) = fresh_pair("fetch_err");
    let code: Vec<c_int> = vec![10, 20, 30];
    for (n, ip) in [
        (0usize, 0usize),
        (0, 1),
        (3, 3),
        (3, 4),
        (3, 100),
        (3, usize::MAX),
        (1, 1),
    ] {
        let mut cp = Program {
            code: code.as_ptr(),
            n,
            ip,
        };
        let mut rp = Program {
            code: code.as_ptr(),
            n,
            ip,
        };
        let mut co: c_int = -424242;
        let mut ro: c_int = -424242;
        let cok = unsafe { (c.prog_fetch)(&mut cp, &mut co) };
        let rok = unsafe { (r.prog_fetch)(&mut rp, &mut ro) };
        assert_eq!(cok, false, "C prog_fetch(n={n}, ip={ip}) must fail");
        assert_eq!(rok, false, "Rust prog_fetch(n={n}, ip={ip}) must fail");
        assert_eq!(co, -424242, "C must not write *out");
        assert_eq!(ro, -424242, "Rust must not write *out");
        assert_eq!(cp.ip, ip, "C must not advance ip");
        assert_eq!(rp.ip, ip, "Rust must not advance ip");
    }
}

/// ERRORS.md row 30 -- lib.c `target` on negative input.
#[test]
fn err_target_negative() {
    let (c, r) = fresh_pair("target_neg");
    let mut rng = Rng::new(0x7777_1111);
    let mut probes: Vec<c_int> = vec![-1, -2, -9, -10, -11, c_int::MIN, c_int::MIN + 1];
    for _ in 0..500 {
        probes.push(-(1 + (rng.next_u64() >> 33) as c_int));
    }
    for v in probes {
        let cv = unsafe { (c.target)(v) };
        let rv = unsafe { (r.target)(v) };
        assert_eq!(cv, 7, "C target({v}) must be 7");
        assert_eq!(rv, 7, "Rust target({v}) must be 7");
    }
}

/// ERRORS.md row 31 -- a.c `target` negative branch depends on `state_a & 1`
/// and must NOT update `state_a`.
#[test]
fn err_a_negative_state() {
    let (c, r) = fresh_pair("a_neg");
    let mut rng = Rng::new(0x3333_2222);
    for i in 0..800 {
        // mix negative-only calls (which take the early return) with positive
        // ones (which mutate state_a) so both parities are exercised
        let x = if i % 3 == 0 {
            -(1 + (rng.next_u64() >> 40) as c_int)
        } else {
            rng.value()
        };
        assert_eq!(
            unsafe { (c.call_a_once)(x) },
            unsafe { (r.call_a_once)(x) },
            "call_a_once({x}) at #{i}"
        );
        let xs: Vec<c_int> = vec![-1, -5, x, -(x.saturating_abs().max(1))];
        assert_eq!(c.stream_a(&xs), r.stream_a(&xs), "stream_a({xs:?}) at #{i}");
    }
}

/// ERRORS.md row 32 -- b.c `target` negative branch toggles `flipflop` first.
#[test]
fn err_b_negative_state() {
    let (c, r) = fresh_pair("b_neg");
    let mut rng = Rng::new(0x4444_5555);
    for i in 0..800 {
        let x = if i % 3 == 0 {
            -(1 + (rng.next_u64() >> 40) as c_int)
        } else {
            rng.value()
        };
        assert_eq!(
            unsafe { (c.call_b_once)(x) },
            unsafe { (r.call_b_once)(x) },
            "call_b_once({x}) at #{i}"
        );
        let xs: Vec<c_int> = vec![-1, -5, x, -(x.saturating_abs().max(1))];
        assert_eq!(c.stream_b(&xs), r.stream_b(&xs), "stream_b({xs:?}) at #{i}");
    }
}

/// ERRORS.md row 36 / G6 -- out-of-range `impl_id` values across the FFI border.
#[test]
fn err_impl_id_out_of_range() {
    let (c, r) = fresh_pair("impl_err");
    let weird: [c_int; 10] = [2, 3, 5, 42, -1, -2, -100, c_int::MIN, c_int::MAX, 0x7FFF];
    // error producing programs, so the *error* paths are checked too
    let programs: Vec<Vec<c_int>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![2],
        vec![4],
        vec![6],
        vec![6, 1],
        vec![0, 1, 6, -1],
        vec![7],
        vec![7, 3],
        vec![9],
        vec![9, -1],
        vec![9, 5],
        vec![11],
        vec![10],
        vec![0, 5, 5, 8, 9, 1],
    ];
    for code in &programs {
        let reference = c.run(2, code);
        for &imp in weird.iter() {
            let cres = c.run(imp, code);
            let rres = r.run(imp, code);
            assert_eq!(cres.0, rres.0, "rc for impl={imp}, code={code:?}");
            assert_eq!(cres.1, rres.1, "state for impl={imp}, code={code:?}");
            assert_eq!(
                cres, reference,
                "C: impl={imp} must behave like impl 2 for {code:?}"
            );
        }
    }
}

/// ERRORS.md row G4 -- double free / free of a zeroed vector / re-init.
#[test]
fn err_double_free_and_reinit() {
    let (c, r) = fresh_pair("double_free");
    // free a never-initialised (zeroed) vector: free(NULL) is a no-op
    let mut cv = IntVec::zeroed();
    let mut rv = IntVec::zeroed();
    unsafe {
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
    }
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv));
    // fill, free, free again, then reuse
    for x in 0..20 {
        unsafe {
            (c.iv_push)(&mut cv, x);
            (r.iv_push)(&mut rv, x);
        }
    }
    unsafe {
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
    }
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "after double free");
    assert!(cv.data.is_null() && rv.data.is_null());
    for x in 0..5 {
        assert_eq!(
            unsafe { (c.iv_push)(&mut cv, x) },
            unsafe { (r.iv_push)(&mut rv, x) }
        );
    }
    assert_eq!(c.snapshot_vec(&cv), r.snapshot_vec(&rv), "reuse after free");
    unsafe {
        (c.iv_free)(&mut cv);
        (r.iv_free)(&mut rv);
    }
    // same for a VM
    let mut cvm = VM::zeroed();
    let mut rvm = VM::zeroed();
    unsafe {
        (c.vm_free)(&mut cvm);
        (r.vm_free)(&mut rvm);
        (c.vm_init)(&mut cvm);
        (r.vm_init)(&mut rvm);
    }
    assert_eq!(c.snapshot_vm(&cvm), r.snapshot_vm(&rvm));
    for t in 0..30 {
        unsafe {
            (c.vm_trace)(&mut cvm, t);
            (r.vm_trace)(&mut rvm, t);
        }
    }
    unsafe {
        (c.vm_free)(&mut cvm);
        (r.vm_free)(&mut rvm);
        (c.vm_free)(&mut cvm);
        (r.vm_free)(&mut rvm);
    }
    assert_eq!(c.snapshot_vm(&cvm), r.snapshot_vm(&rvm), "VM double free");
    assert_eq!(
        c.print_vm_raw("Z:", &cvm),
        r.print_vm_raw("Z:", &rvm),
        "vm_print after free"
    );
}

/// ERRORS.md row 22b -- `nc * sizeof(int)` itself overflows `size_t` and wraps to
/// 0, so the C calls `realloc(ptr, 0)`.  With a pristine (NULL) buffer glibc
/// returns a minimal allocation and `iv_reserve` reports success with a bogus
/// `cap`; with a live buffer glibc *frees* it and returns NULL, so the C is left
/// with a dangling `data` pointer (a real latent bug in the C -- the buffer
/// contents are indeterminate afterwards, so only the reported result can be
/// compared).
#[test]
fn err_iv_reserve_size_wrap() {
    let (c, r) = fresh_pair("reserve_wrap");
    // sizeof(int) == 4, so any nc >= 2^62 makes nc*4 wrap to 0.
    for need in [1usize << 62, 3usize << 61, (1usize << 62) + 1] {
        let mut cv = c.new_vec();
        let mut rv = r.new_vec();
        let cok = unsafe { (c.iv_reserve)(&mut cv, need) };
        let rok = unsafe { (r.iv_reserve)(&mut rv, need) };
        assert_eq!(cok, rok, "iv_reserve({need:#x}) on a pristine vector");
        assert_eq!(cv.len, rv.len, "len after iv_reserve({need:#x})");
        assert_eq!(cv.cap, rv.cap, "cap after iv_reserve({need:#x})");
        assert_eq!(
            cv.data.is_null(),
            rv.data.is_null(),
            "data==NULL after iv_reserve({need:#x})"
        );
        // The (possibly 0-byte) allocation is still valid memory: free it.
        unsafe {
            (c.iv_free)(&mut cv);
            (r.iv_free)(&mut rv);
        }
    }
    // Same request on a *populated* vector: both sides must report the same
    // thing.  The buffer is freed by realloc(ptr, 0), so the IntVec must not be
    // touched again afterwards (deliberately leaked).
    let mut cv = c.new_vec();
    let mut rv = r.new_vec();
    for x in 0..10 {
        unsafe {
            (c.iv_push)(&mut cv, x);
            (r.iv_push)(&mut rv, x);
        }
    }
    let cok = unsafe { (c.iv_reserve)(&mut cv, 1usize << 62) };
    let rok = unsafe { (r.iv_reserve)(&mut rv, 1usize << 62) };
    assert_eq!(cok, rok, "iv_reserve(2^62) on a populated vector");
    assert_eq!(cv.len, rv.len);
    assert_eq!(cv.cap, rv.cap);
    assert_eq!(cv.data.is_null(), rv.data.is_null());
    // NB: `cv`/`rv` now hold dangling `data` pointers (realloc freed them) and
    // must not be used or freed again; they are simply abandoned here.
    let _ = (cv.len, rv.len);
}

// ===========================================================================
// Remaining surface: the exported `main` symbol, `steps` overflow, NULL label.
// ===========================================================================

/// The 19th exported symbol: `main`, called through `dlopen`/`dlsym` on each
/// `.so` by a tiny C loader (see common::main_caller), comparing stdout, stderr
/// and the exit status.
#[test]
fn err_main_symbol_via_dlopen() {
    let (c, r) = fresh_pair("main_sym");
    let cases: Vec<Vec<&str>> = vec![
        vec![],                       // argc == 0, argv == NULL
        vec!["./driver"],             // no program -> rc 2
        vec!["./driver", "--help"],   // usage on stderr, rc 0
        vec!["./driver", "0", "42"],  // a real program
        vec!["./driver", "abc"],      // skip message + no program
        vec!["./driver", ""],         // empty arg pushes 0
        vec!["./driver", "9", "0"],   // opcode 9 with m == 0
        vec!["./driver", "11"],       // unknown opcode -> rc 99 in the output
        vec!["./driver", "0", "100", "3", "5", "8", "9", "2", "1", "10"],
    ];
    for case in cases {
        let co = c.call_main(&case);
        let ro = r.call_main(&case);
        assert_eq!(co.rc, ro.rc, "main{case:?} exit code");
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout),
            "main{case:?} stdout"
        );
        assert_eq!(
            String::from_utf8_lossy(&co.stderr),
            String::from_utf8_lossy(&ro.stderr),
            "main{case:?} stderr"
        );
        assert_eq!(co, ro, "main{case:?} bytes");
    }
}

/// `vm->steps++` overflows for a caller-supplied `steps` near INT_MAX (UB in C,
/// wraps in practice -- the Rust must wrap the same way).
#[test]
fn cfg_engine_steps_overflow() {
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("steps{imp}"));
        for start in [
            c_int::MAX,
            c_int::MAX - 1,
            c_int::MAX - 3,
            c_int::MIN,
            -1,
            -5,
        ] {
            let code = vec![0, 5, 3, 5, 8, 3, 9, 2, 1, 10];
            let mut cvm = c.new_vm();
            let mut rvm = r.new_vm();
            cvm.steps = start;
            rvm.steps = start;
            let crc = unsafe { (c.run_engine)(imp, code.as_ptr(), code.len(), &mut cvm) };
            let rrc = unsafe { (r.run_engine)(imp, code.as_ptr(), code.len(), &mut rvm) };
            assert_eq!(crc, rrc, "steps={start} rc");
            assert_eq!(
                c.snapshot_vm(&cvm),
                r.snapshot_vm(&rvm),
                "steps={start} state (impl={imp})"
            );
            assert_eq!(
                c.print_vm_raw("S:", &cvm),
                r.print_vm_raw("S:", &rvm),
                "steps={start} vm_print"
            );
            unsafe {
                (c.vm_free)(&mut cvm);
                (r.vm_free)(&mut rvm);
            }
        }
    }
}

/// ERRORS.md row G7 -- `vm_print` with a NULL label.  The C hands the pointer
/// straight to `%s`; glibc prints "(null)".  The Rust must forward it unchanged.
#[test]
fn err_vm_print_null_label() {
    let (c, r) = fresh_pair("null_label");
    let mut cvm = c.new_vm();
    let mut rvm = r.new_vm();
    for t in [0, 1, 2, 13] {
        unsafe {
            (c.vm_trace)(&mut cvm, t);
            (r.vm_trace)(&mut rvm, t);
        }
    }
    let co = c.print_vm_null_label(&cvm);
    let ro = r.print_vm_null_label(&rvm);
    assert_eq!(
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
        "vm_print(NULL label)"
    );
    unsafe {
        (c.vm_free)(&mut cvm);
        (r.vm_free)(&mut rvm);
    }
}

/// CONFIGS.md rows 26b/30b -- `iv_reserve` / `iv_push` starting from a capacity
/// the caller chose (not one produced by the doubling loop).  Through the public
/// API `cap` is always 0 or 8*2^k, so the doubling start value is invisible;
/// a C consumer may however hand over any `cap`, and then `nc` starts there.
#[test]
fn cfg_iv_arbitrary_cap() {
    let (c, r) = fresh_pair("arb_cap");
    let caps = [1usize, 2, 3, 5, 6, 7, 9, 10, 13, 17, 20, 33, 100, 1000];
    let mut rng = Rng::new(0xA0B0_C0D0);
    for &cap in caps.iter() {
        for len in [0usize, 1, cap / 2, cap] {
            let items: Vec<c_int> = (0..len).map(|_| rng.value()).collect();
            // ---- iv_reserve from this cap
            for need in [
                0,
                1,
                cap.saturating_sub(1),
                cap,
                cap + 1,
                cap + 2,
                2 * cap,
                2 * cap + 1,
                3 * cap + 1,
                8 * cap + 3,
            ] {
                let mut cv = make_vec(&items, cap);
                let mut rv = make_vec(&items, cap);
                let cok = unsafe { (c.iv_reserve)(&mut cv, need) };
                let rok = unsafe { (r.iv_reserve)(&mut rv, need) };
                assert_eq!(cok, rok, "iv_reserve(need={need}) from cap={cap}");
                assert_eq!(
                    c.snapshot_vec(&cv),
                    r.snapshot_vec(&rv),
                    "iv_reserve(need={need}) from cap={cap}, len={len}"
                );
                unsafe {
                    (c.iv_free)(&mut cv);
                    (r.iv_free)(&mut rv);
                }
            }
            // ---- iv_push (which reserves cap*2) from this cap
            let mut cv = make_vec(&items, cap);
            let mut rv = make_vec(&items, cap);
            for k in 0..(cap + 5) {
                let x = rng.value();
                assert_eq!(
                    unsafe { (c.iv_push)(&mut cv, x) },
                    unsafe { (r.iv_push)(&mut rv, x) },
                    "iv_push #{k} from cap={cap}, len={len}"
                );
                assert_eq!(
                    c.snapshot_vec(&cv),
                    r.snapshot_vec(&rv),
                    "iv_push #{k} state from cap={cap}, len={len}"
                );
            }
            unsafe {
                (c.iv_free)(&mut cv);
                (r.iv_free)(&mut rv);
            }
        }
    }
}

/// CONFIGS.md row 51b -- `run_engine` on a VM whose stack the caller prepared
/// with an arbitrary capacity, so the engine's pushes grow it from there.
#[test]
fn cfg_engine_caller_supplied_vm() {
    let mut rng = Rng::new(0x0DD0_1234);
    for &imp in IMPLS.iter() {
        let (c, r) = fresh_pair(&format!("arbvm{imp}"));
        for &cap in [1usize, 2, 3, 5, 9, 10, 17].iter() {
            for len in [0usize, 1, cap] {
                let items: Vec<c_int> = (0..len).map(|_| rng.value()).collect();
                for _ in 0..20 {
                    let code = rng.program(14);
                    let mut cvm = make_vm(&items, cap, 0);
                    let mut rvm = make_vm(&items, cap, 0);
                    let ptr = if code.is_empty() {
                        std::ptr::null()
                    } else {
                        code.as_ptr()
                    };
                    let crc = unsafe { (c.run_engine)(imp, ptr, code.len(), &mut cvm) };
                    let rrc = unsafe { (r.run_engine)(imp, ptr, code.len(), &mut rvm) };
                    assert_eq!(
                        crc, rrc,
                        "rc (impl={imp}, cap={cap}, len={len}, code={code:?})"
                    );
                    assert_eq!(
                        c.snapshot_vm(&cvm),
                        r.snapshot_vm(&rvm),
                        "VM state (impl={imp}, cap={cap}, len={len}, code={code:?})"
                    );
                    assert_eq!(
                        c.print_vm_raw("V:", &cvm),
                        r.print_vm_raw("V:", &rvm),
                        "vm_print (impl={imp}, cap={cap}, len={len})"
                    );
                    unsafe {
                        (c.vm_free)(&mut cvm);
                        (r.vm_free)(&mut rvm);
                    }
                }
            }
        }
    }
}
