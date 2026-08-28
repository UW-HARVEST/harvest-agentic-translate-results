//! Phase C — error-path differential tests, one `#[test]` per row of
//! `ERRORS.md`, plus the generic C-API boundaries.

mod common;

use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// -------------------------------------------------------------------- row 1
// `rnd == NULL`: the C dereferences address 0 with no check. Both libraries
// must die from the *same* fatal signal.

/// Not a real test: the subprocess body used by
/// [`row01_null_pointer_same_fatal_signal`]. Marked `#[ignore]` so it never
/// runs as part of a normal `cargo test` sweep.
#[test]
#[ignore]
fn helper_null_deref() {
    let which = std::env::var("NULL_DEREF_TARGET").expect("NULL_DEREF_TARGET not set");
    let lib = match which.as_str() {
        "c" => c_lib(),
        p if PROFILES.contains(&p) => rust_lib(p),
        other => panic!("unknown NULL_DEREF_TARGET {other}"),
    };
    // Make sure the library really is loaded and callable before we fault, so
    // a load error can never be mistaken for the expected crash.
    let mut ok = CnRnd::new(1, 2);
    let probe = lib.call(&mut ok);
    assert!(probe >= 0.0 && probe < 1.0);
    eprintln!("about to dereference NULL via {}", lib.label);

    let v = unsafe { lib.call_ptr(std::ptr::null_mut()) };
    // Unreachable in practice; if we get here the behaviour differs from C.
    println!("NO_CRASH {:#018x}", v.to_bits());
    std::process::exit(42);
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    no_crash_line: Option<String>,
}

fn run_null_deref(target: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "helper_null_deref",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("NULL_DEREF_TARGET", target)
        // the child re-uses the already-built .so files
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .output()
        .expect("spawn null-deref child");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
        no_crash_line: stdout
            .lines()
            .find(|l| l.starts_with("NO_CRASH"))
            .map(|s| s.to_string()),
    }
}

#[test]
fn row01_null_pointer_same_fatal_signal() {
    let c = run_null_deref("c");

    assert_eq!(
        c.signal,
        Some(libc::SIGSEGV),
        "expected the C library to die from SIGSEGV on a NULL rnd, got {c:?}"
    );
    assert_eq!(
        c.no_crash_line, None,
        "C unexpectedly survived the NULL dereference"
    );

    // The shipped configurations must fault exactly like the C.
    for profile in ["dev", "release"] {
        let got = run_null_deref(profile);
        assert_eq!(
            c, got,
            "NULL rnd: C and Rust({profile}) must fail identically\n  \
             C   = {c:?}\n  {profile} = {got:?}"
        );
    }

    // `ubcheck` re-enables Rust's *optional* UB checks, which convert this
    // (undefined in both languages) NULL dereference into a fail-safe abort.
    // It must still die fatally and must never silently return a value.
    let ubc = run_null_deref("ubcheck");
    assert_eq!(
        ubc.no_crash_line, None,
        "ubcheck build survived the NULL dereference: {ubc:?}"
    );
    assert!(
        ubc.signal == Some(libc::SIGSEGV) || ubc.signal == Some(libc::SIGABRT),
        "ubcheck build must die from SIGSEGV (like C) or the fail-safe SIGABRT, got {ubc:?}"
    );
}

// -------------------------------------------------------------------- row 2
// Misaligned `cn_rnd_t*`. No alignment check exists in the C; on x86-64 the
// access simply succeeds. Both libraries must agree with each other AND with
// the aligned result.
#[test]
fn row02_misaligned_pointer_matches() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0xA2);
        for offset in 1usize..8 {
            for _ in 0..128 {
                let seed = CnRnd::new(rng.next_u64(), rng.next_u64());

                // aligned reference result (per library)
                let mut ac = seed;
                let mut ar = seed;
                let vac = c.call(&mut ac);
                let var = r.call(&mut ar);

                // misaligned: struct starts `offset` bytes into the buffer
                let run_unaligned = |lib: &Lib| -> (f64, [u8; 16]) {
                    let mut buf = [0u8; 32];
                    buf[offset..offset + 8].copy_from_slice(&seed.state[0].to_ne_bytes());
                    buf[offset + 8..offset + 16].copy_from_slice(&seed.state[1].to_ne_bytes());
                    let v =
                        unsafe { lib.call_ptr(buf.as_mut_ptr().add(offset) as *mut CnRnd) };
                    let mut after = [0u8; 16];
                    after.copy_from_slice(&buf[offset..offset + 16]);
                    (v, after)
                };
                let (vuc, buc) = run_unaligned(c);
                let (vur, bur) = run_unaligned(r);

                assert_eq!(
                    vuc.to_bits(),
                    vur.to_bits(),
                    "row02 offset {offset}: misaligned return value differs \
                     (seed {:#x?})",
                    seed.state
                );
                assert_eq!(
                    buc, bur,
                    "row02 offset {offset}: misaligned post-state differs (seed {:#x?})",
                    seed.state
                );
                // and the misaligned access behaves exactly like the aligned one
                assert_eq!(
                    vuc.to_bits(),
                    vac.to_bits(),
                    "row02 offset {offset}: C misaligned != C aligned"
                );
                assert_eq!(
                    vur.to_bits(),
                    var.to_bits(),
                    "row02 offset {offset}: rust misaligned != rust aligned"
                );
                assert_eq!(buc, ac.bytes(), "row02 C misaligned state != aligned state");
                assert_eq!(bur, ar.bytes(), "row02 rust misaligned state != aligned");
            }
        }
    });
}

// -------------------------------------------------------------------- row 3
// The all-zero seed is *not* rejected and *not* reseeded: the generator is a
// fixed point at zero forever.
#[test]
fn row03_zero_seed_is_not_rejected() {
    for_each_pair(|c, r| {
        let mut sc = CnRnd::new(0, 0);
        let mut sr = CnRnd::new(0, 0);
        for step in 0..1024 {
            let vc = c.call(&mut sc);
            let vr = r.call(&mut sr);
            assert_eq!(vc.to_bits(), 0, "C must return +0.0 at step {step}");
            assert_eq!(
                vr.to_bits(),
                vc.to_bits(),
                "rust must also return +0.0 at step {step}"
            );
            assert_eq!(sc, CnRnd::new(0, 0), "C state must stay zero");
            assert_eq!(sr, sc, "rust state must stay zero, like C");
            assert!(
                vc.is_sign_positive(),
                "must be +0.0, not -0.0 (bits {:#018x})",
                vc.to_bits()
            );
        }
    });
}

// -------------------------------------------------------------------- row 4
// `x + y` overflow on the largest representable state: must wrap, must not
// trap/panic, in both cdylib profiles (dev has overflow-checks = on).
#[test]
fn row04_max_state_add_overflow_no_panic() {
    for_each_pair(|c, r| {
        assert_seq(c, r, CnRnd::new(u64::MAX, u64::MAX), 256, "row04 MAX/MAX");

        // A dense family of deliberately-overflowing states.
        let mut rng = SplitMix64::new(SEED ^ 0xA4);
        for _ in 0..4096 {
            let y = rng.next_nonzero();
            let x_final = u64::MAX - (rng.next_u64() % y);
            assert!(x_final.checked_add(y).is_none());
            let seed = seed_for_value(x_final.wrapping_add(y), y);
            assert_one(c, r, seed, "row04 forced wrapping add");
        }
        // Shift-boundary saturation (x << 23 / x >> 17 / y >> 26 losing bits).
        for i in 0..64 {
            assert_seq(
                c,
                r,
                CnRnd::new(u64::MAX << i, u64::MAX >> i),
                32,
                "row04 shift saturation",
            );
            assert_seq(
                c,
                r,
                CnRnd::new(u64::MAX >> i, u64::MAX << i),
                32,
                "row04 shift saturation (swapped)",
            );
        }
    });
}

// -------------------------------------------------------------------- row 5
// "One step past the valid range": there is no range, so every boundary value
// of the whole u64 x u64 domain is valid and must be accepted identically.
#[test]
fn row05_boundary_value_sweep_never_rejected() {
    for_each_pair(|c, r| {
        let mut vals: Vec<u64> = vec![0, 1, 2, u64::MAX, u64::MAX - 1, u64::MAX - 2];
        for k in 0..64 {
            let p = 1u64 << k;
            vals.push(p);
            vals.push(p.wrapping_sub(1));
            vals.push(p.wrapping_add(1));
            vals.push(!p);
        }
        // shift-relevant boundaries: 2^17, 2^23, 2^26, 2^41, 2^52 and neighbours
        for k in [12u32, 17, 23, 26, 41, 52, 63] {
            let p = 1u64 << k;
            vals.extend_from_slice(&[p - 1, p, p + 1, !(p - 1), !p]);
        }
        vals.sort_unstable();
        vals.dedup();

        for &a in &vals {
            for &b in &vals {
                let mut sc = CnRnd::new(a, b);
                let mut sr = CnRnd::new(a, b);
                let vc = c.call(&mut sc);
                let vr = r.call(&mut sr);
                assert_eq!(
                    vc.to_bits(),
                    vr.to_bits(),
                    "row05 boundary ({a:#018x}, {b:#018x}) return differs"
                );
                assert_eq!(
                    sc, sr,
                    "row05 boundary ({a:#018x}, {b:#018x}) state differs"
                );
                // no rejection: always a finite value in [0, 1)
                assert!(
                    vc.is_finite() && (0.0..1.0).contains(&vc),
                    "row05 C rejected/garbled ({a:#018x}, {b:#018x}) -> {vc:?}"
                );
            }
        }
    });
}

// -------------------------------------------------------------------- row 6
// Out-of-range enum across the FFI boundary: structurally impossible here.
#[test]
fn row06_no_enum_parameters_exist() {
    let hdr = std::fs::read_to_string(repo_root().join("c_src/include/lib.h")).unwrap();
    let src = std::fs::read_to_string(repo_root().join("c_src/src/lib.c")).unwrap();
    for (name, text) in [("lib.h", &hdr), ("lib.c", &src)] {
        assert!(
            !text.contains("enum"),
            "{name} declares an enum -- ERRORS.md row 6 must gain real rows"
        );
    }
    // The single public prototype takes exactly one pointer parameter.
    assert!(
        hdr.contains("double next_double(cn_rnd_t *rnd);"),
        "public prototype changed; re-derive ERRORS.md"
    );
    // Sanity: passing a non-null pointer to a struct holding *any* bit pattern
    // is accepted -- there is no "invalid variant" to reject.
    for_each_pair(|c, r| {
        for bits in [0u64, 1, u64::MAX, 0x5555_5555_5555_5555, 0xAAAA_AAAA_AAAA_AAAA] {
            assert_one(c, r, CnRnd::new(bits, !bits), "row06 arbitrary bit pattern");
        }
    });
}

// -------------------------------------------------------------------- row 7
// Zero / oversized length arguments: structurally impossible here.
#[test]
fn row07_no_length_parameters_exist() {
    let hdr = std::fs::read_to_string(repo_root().join("c_src/include/lib.h")).unwrap();
    // Tokenise so that e.g. `cn_rnd_t` cannot accidentally match a fragment.
    let tokens: Vec<String> = hdr
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();
    for bad in [
        "size_t", "ssize_t", "len", "length", "count", "capacity", "cap", "n", "num", "size",
        "nmemb", "nbytes",
    ] {
        assert!(
            !tokens.iter().any(|t| t == bad),
            "lib.h declares a `{bad}` parameter -- ERRORS.md row 7 must gain real rows \
             (tokens: {tokens:?})"
        );
    }
    assert!(
        hdr.contains("uint64_t state[2];"),
        "cn_rnd_t is no longer a fixed uint64_t[2]; re-derive ERRORS.md"
    );
    // The Rust ABI mirror must be exactly 16 bytes / 8-byte aligned like the C.
    assert_eq!(std::mem::size_of::<CnRnd>(), 16);
    assert_eq!(std::mem::align_of::<CnRnd>(), 8);
}

// -------------------------------------------------------------------- row 8
// Neither library may read or write outside the 16 bytes of `cn_rnd_t`.
#[test]
fn row08_no_out_of_bounds_write() {
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0xA8);
        const N: usize = 8;
        for _ in 0..512 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
            // distinct canary in every neighbouring slot, so any stray write
            // is localised.
            let canaries: [u64; N] = std::array::from_fn(|i| {
                0xC0DE_0000_0000_0000u64 ^ ((i as u64) << 32) ^ seed.state[0]
            });

            let run = |lib: &Lib| -> (f64, [u64; N]) {
                let mut buf = canaries;
                buf[3] = seed.state[0];
                buf[4] = seed.state[1];
                let v = unsafe { lib.call_ptr(buf.as_mut_ptr().add(3) as *mut CnRnd) };
                (v, buf)
            };
            let (vc, bc) = run(c);
            let (vr, br) = run(r);

            assert_eq!(vc.to_bits(), vr.to_bits(), "row08 return value differs");
            assert_eq!(bc, br, "row08 buffers differ");
            for i in [0usize, 1, 2, 5, 6, 7] {
                assert_eq!(
                    bc[i], canaries[i],
                    "row08 C wrote outside cn_rnd_t at slot {i}"
                );
                assert_eq!(
                    br[i], canaries[i],
                    "row08 rust wrote outside cn_rnd_t at slot {i}"
                );
            }
        }
    });
}

// ------------------------------------------------------- generic FFI boundary
// A single-parameter, no-length, no-enum API leaves only these: repeated calls
// on a freshly zeroed struct, aliasing the same struct across libraries, and
// re-entrancy from several threads.
#[test]
fn generic_same_struct_shared_between_libraries() {
    // Alternating calls on ONE struct: any hidden extra state in either
    // implementation shows up as a mismatch against a pure C-only chain.
    for_each_pair(|c, r| {
        let mut rng = SplitMix64::new(SEED ^ 0xB1);
        for _ in 0..128 {
            let seed = CnRnd::new(rng.next_u64(), rng.next_u64());

            let mut mixed = seed;
            let mut mixed_vals = Vec::new();
            for i in 0..256 {
                let v = if i % 2 == 0 {
                    c.call(&mut mixed)
                } else {
                    r.call(&mut mixed)
                };
                mixed_vals.push(v.to_bits());
            }

            let mut pure = seed;
            for (i, expected) in mixed_vals.iter().enumerate() {
                let v = c.call(&mut pure);
                assert_eq!(
                    v.to_bits(),
                    *expected,
                    "alternating C/rust chain diverged from pure C chain at {i}"
                );
            }
            assert_eq!(pure, mixed, "final state of mixed chain != pure C chain");
        }
    });
}

#[test]
fn generic_multithreaded_reentrancy() {
    // No global/TLS state may exist: N threads each with their own struct must
    // reproduce their single-threaded results exactly.
    let expected: Vec<(CnRnd, Vec<u64>)> = {
        let c = c_lib();
        let mut rng = SplitMix64::new(SEED ^ 0xB2);
        (0..8)
            .map(|_| {
                let seed = CnRnd::new(rng.next_u64(), rng.next_u64());
                let mut s = seed;
                let vals = (0..2048).map(|_| c.call(&mut s).to_bits()).collect();
                (seed, vals)
            })
            .collect()
    };

    for profile in PROFILES {
        let path = rust_so_path(profile);
        let handles: Vec<_> = expected
            .iter()
            .cloned()
            .map(|(seed, vals)| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let lib = unsafe { libloading::Library::new(&path).unwrap() };
                    let f: libloading::Symbol<NextDoubleFn> =
                        unsafe { lib.get(b"next_double\0").unwrap() };
                    let mut s = seed;
                    for (i, want) in vals.iter().enumerate() {
                        let got = unsafe { f(&mut s as *mut CnRnd) };
                        assert_eq!(got.to_bits(), *want, "thread diverged at call {i}");
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().expect("worker thread panicked");
        }
    }
}
