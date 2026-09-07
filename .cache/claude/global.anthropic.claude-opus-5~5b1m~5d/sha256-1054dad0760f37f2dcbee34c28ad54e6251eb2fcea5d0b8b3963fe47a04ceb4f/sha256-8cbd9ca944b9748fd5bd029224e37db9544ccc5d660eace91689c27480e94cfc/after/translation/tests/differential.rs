//! C-vs-Rust differential tests, driven entirely through the two `.so`
//! exports via `libloading` (never by calling the Rust functions directly).
//!
//! * `cfg_*`  -> one test per row of CONFIGS.md (Phase B, valid paths)
//! * `err_*`  -> one test per row of ERRORS.md  (Phase C, error/boundary paths)
//! * `sym_*`  -> Phase D symbol parity, checked through real `dlsym`
//!
//! The library's only entry point is `void driver(int)`; its whole observable
//! behaviour is the hex line it `printf`s, so "outputs match byte-for-byte"
//! means the captured stdout bytes are equal.

mod common;

use common::*;

// ===========================================================================
// Phase B -- CONFIGS.md rows
// ===========================================================================

/// C1: `floors == 0` -- the falsy / all-zero shape.
#[test]
fn cfg_c1_zero() {
    assert_same_both_ways("C1", &[0]);
}

/// C2: `floors == 1` -- smallest positive; exercises `%02x` zero padding.
#[test]
fn cfg_c2_one() {
    assert_same_both_ways("C2", &[1]);
}

/// C3: every single-hex-digit value -- the `%02x` zero-pad path.
#[test]
fn cfg_c3_single_hex_digit() {
    let inputs: Vec<i32> = (0..16).collect();
    assert_same_both_ways("C3", &inputs);
}

/// C4: every one-byte value 0..=255 (low byte varies, high three bytes zero).
#[test]
fn cfg_c4_all_single_byte_values() {
    let inputs: Vec<i32> = (0..256).collect();
    assert_same("C4", &inputs);
}

/// C5: every single-bit position `1 << k`, k = 0..31 (incl. the sign bit).
#[test]
fn cfg_c5_every_single_bit() {
    let inputs: Vec<i32> = (0..32).map(|k| 1i32.wrapping_shl(k)).collect();
    assert_same_both_ways("C5", &inputs);
}

/// C6: bit-run boundaries on both signs: `(1<<k)-1` and `-(1<<k)`.
#[test]
fn cfg_c6_bit_run_boundaries() {
    let mut inputs = Vec::new();
    for k in 0..32u32 {
        let p = 1i32.wrapping_shl(k);
        inputs.push(p.wrapping_sub(1));
        inputs.push(p.wrapping_neg());
        inputs.push(p.wrapping_add(1));
    }
    assert_same("C6", &inputs);
}

/// C7: byte-order probes -- pins the little-endian byte layout of the dump.
#[test]
fn cfg_c7_byte_order_probes() {
    let inputs: Vec<i32> = [
        0x0102_0304u32,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0xff00_0000,
        0x1234_5678,
        0x7856_3412,
        0xdead_beef,
        0xefbe_adde,
        0xcafe_babe,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();
    assert_same_both_ways("C7", &inputs);
}

/// C8: values containing embedded zero bytes.
#[test]
fn cfg_c8_embedded_zero_bytes() {
    let inputs: Vec<i32> = [
        0x00ff_00ffu32,
        0xff00_ff00,
        0x0000_0100, // 256
        0x0001_0000, // 65536
        0x0100_0000, // 1 << 24
        0x00ff_ff00,
        0xff00_00ff,
        0x0000_0000,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();
    assert_same_both_ways("C8", &inputs);
}

/// C9: small negatives -1 ..= -256 -- two's-complement high-byte fill.
#[test]
fn cfg_c9_small_negatives() {
    let inputs: Vec<i32> = (1..=256).map(|n| -n).collect();
    assert_same("C9", &inputs);
}

/// C10: the extremes of `int`.
#[test]
fn cfg_c10_extremes() {
    let inputs = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    assert_same_both_ways("C10", &inputs);
}

/// C11: 4096 uniform random `i32`, fixed seed.
#[test]
fn cfg_c11_random_full_range() {
    let mut rng = Rng::new(SEED);
    let inputs: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
    assert_same("C11", &inputs);
}

/// C12: 1024 random values clustered near INT_MIN / INT_MAX / 0.
#[test]
fn cfg_c12_random_near_boundaries() {
    let mut rng = Rng::new(SEED ^ 0x1111_1111);
    let inputs: Vec<i32> = (0..1024)
        .map(|_| {
            let delta = rng.below(4096) as i32 - 2048;
            match rng.below(4) {
                0 => i32::MIN.wrapping_add(delta),
                1 => i32::MAX.wrapping_add(delta),
                2 => delta,
                _ => (rng.next_i32() >> 1).wrapping_add(delta),
            }
        })
        .collect();
    assert_same("C12", &inputs);
}

/// C13: 1024 random values with random bytes forced to 0x00 / 0xff.
#[test]
fn cfg_c13_random_sparse_bytes() {
    let mut rng = Rng::new(SEED ^ 0x2222_2222);
    let inputs: Vec<i32> = (0..1024)
        .map(|_| {
            let mut bytes = rng.next_u32().to_le_bytes();
            for b in bytes.iter_mut() {
                match rng.below(3) {
                    0 => *b = 0x00,
                    1 => *b = 0xff,
                    _ => {}
                }
            }
            u32::from_le_bytes(bytes) as i32
        })
        .collect();
    assert_same("C13", &inputs);
}

/// C14: many calls in sequence with different inputs -- statelessness, and the
/// same sequence run C-first then Rust-first to rule out ordering effects.
#[test]
fn cfg_c14_many_calls_stateless() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x3333_3333);
    let inputs: Vec<i32> = (0..512).map(|_| rng.next_i32()).collect();

    // C then Rust
    let c_first = call_capture_batch(&p.c, &inputs);
    let r_second = call_capture_batch(&p.rust, &inputs);
    // Rust then C
    let r_first = call_capture_batch(&p.rust, &inputs);
    let c_second = call_capture_batch(&p.c, &inputs);

    for (i, x) in inputs.iter().enumerate() {
        assert_eq!(c_first[i], r_second[i], "[C14] C-first vs Rust for driver({x})");
        assert_eq!(r_first[i], c_second[i], "[C14] Rust-first vs C for driver({x})");
        // No carry-over across calls: each library is self-consistent.
        assert_eq!(c_first[i], c_second[i], "[C14] C not stateless at driver({x})");
        assert_eq!(r_first[i], r_second[i], "[C14] Rust not stateless at driver({x})");
    }
}

/// C15: the same input repeated -- idempotence, no accumulation.
#[test]
fn cfg_c15_repeated_same_input() {
    for x in [0, 1, -1, 7, i32::MIN, i32::MAX, 0x0102_0304] {
        let inputs = vec![x; 64];
        assert_same("C15", &inputs);

        // Every repetition of the same input must be byte-identical.
        let p = libs();
        let c = call_capture_batch(&p.c, &inputs);
        let r = call_capture_batch(&p.rust, &inputs);
        for i in 1..inputs.len() {
            assert_eq!(c[i], c[0], "[C15] C output changed on repeat {i} of driver({x})");
            assert_eq!(r[i], r[0], "[C15] Rust output changed on repeat {i} of driver({x})");
        }
    }
}

/// C16: struct-layout invariants of every dump, checked identically on both
/// libraries (this is the only observation of the private `print_hex` /
/// `house_t`, which have no external linkage).
#[test]
fn cfg_c16_struct_layout_invariants() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x4444_4444);
    let mut inputs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, 3, 0x0102_0304];
    inputs.extend((0..256).map(|_| rng.next_i32()));

    let c_out = call_capture_batch(&p.c, &inputs);
    let r_out = call_capture_batch(&p.rust, &inputs);

    for (i, &x) in inputs.iter().enumerate() {
        for (who, line) in [("C", &c_out[i]), ("Rust", &r_out[i])] {
            // sizeof(house_t) == 16 -> 32 hex chars + '\n'
            assert_eq!(
                line.len(),
                33,
                "[C16] {who} line length for driver({x}) is {} not 33: {:?}",
                line.len(),
                String::from_utf8_lossy(line)
            );
            assert_eq!(line[32], b'\n', "[C16] {who} missing trailing newline for driver({x})");
            let hex = std::str::from_utf8(&line[..32]).expect("[C16] non-UTF8 output");
            assert!(
                hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "[C16] {who} output for driver({x}) is not lowercase hex: {hex}"
            );

            // floors  @ bytes 0..4  == x, little-endian
            let expect_floors: String =
                x.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(&hex[0..8], expect_floors, "[C16] {who} floors bytes for driver({x})");
            // bedrooms @ bytes 4..8 == 3
            assert_eq!(&hex[8..16], "03000000", "[C16] {who} bedrooms bytes for driver({x})");
            // bathrooms @ bytes 8..16 == 2.0 -> IEEE-754 0x4000000000000000 LE
            assert_eq!(
                &hex[16..32], "0000000000000040",
                "[C16] {who} bathrooms bytes for driver({x})"
            );
        }
        assert_eq!(c_out[i], r_out[i], "[C16] divergence for driver({x})");
    }
}

/// C17: one whole-process stream -- every input dumped by C in a single
/// capture vs. every input dumped by Rust in a single capture, compared as one
/// blob. Catches per-call buffering / flush differences that a line-by-line
/// comparison could mask.
#[test]
fn cfg_c17_bulk_stream_equivalence() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x5555_5555);
    let mut inputs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX];
    inputs.extend((0..2048).map(|_| rng.next_i32()));

    let cf = p.c.driver();
    let rf = p.rust.driver();

    let c_blob = capture_fd1("bulk_c", || {
        for &x in &inputs {
            unsafe { cf(x) }
        }
    });
    let r_blob = capture_fd1("bulk_r", || {
        for &x in &inputs {
            unsafe { rf(x) }
        }
    });

    assert_eq!(
        c_blob.len(),
        inputs.len() * 33,
        "[C17] C blob length unexpected"
    );
    assert_eq!(
        c_blob, r_blob,
        "[C17] bulk stream divergence (C {} bytes vs Rust {} bytes)",
        c_blob.len(),
        r_blob.len()
    );
}

/// C18: the project builds NO binary -- `c_src/CMakeLists.txt` has only
/// `add_library(driver SHARED ...)` and `translation/Cargo.toml` only a
/// `cdylib` lib target. Asserted mechanically so the claim cannot silently rot.
#[test]
fn cfg_c18_no_binary_target() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let cmake = std::fs::read_to_string(root.join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "[C18] c_src now builds an executable -- add a binary stdout comparison"
    );

    let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[[bin]]"),
        "[C18] the crate now builds a binary -- add a binary stdout comparison"
    );
    assert!(
        !root.join("src/main.rs").exists(),
        "[C18] src/main.rs appeared -- add a binary stdout comparison"
    );
}

// ===========================================================================
// Phase C -- ERRORS.md rows
//
// The C library has no error channel at all (`void driver(int)`: no return
// code, no out-param, no errno, no assert, no range check). "Same error" is
// therefore "same acceptance": both libraries must accept the input and emit
// byte-identical output, and neither may abort/trap.
// ===========================================================================

/// E1: `floors == 0`.
#[test]
fn err_e1_zero() {
    assert_same_both_ways("E1", &[0]);

    let p = libs();
    let out = call_capture(&p.c, 0);
    assert_eq!(
        String::from_utf8_lossy(&out),
        "00000000030000000000000000000040\n",
        "[E1] unexpected C baseline for driver(0)"
    );
    assert_eq!(call_capture(&p.rust, 0), out, "[E1] Rust differs from C for driver(0)");
}

/// E2: `floors == INT_MAX`.
#[test]
fn err_e2_int_max() {
    assert_same_both_ways("E2", &[i32::MAX]);
    let p = libs();
    let c = call_capture(&p.c, i32::MAX);
    assert_eq!(
        String::from_utf8_lossy(&c),
        "ffffff7f030000000000000000000040\n",
        "[E2] unexpected C baseline"
    );
    assert_eq!(call_capture(&p.rust, i32::MAX), c);
}

/// E3: `floors == INT_MIN` -- most negative, no positive counterpart.
#[test]
fn err_e3_int_min() {
    assert_same_both_ways("E3", &[i32::MIN]);
    let p = libs();
    let c = call_capture(&p.c, i32::MIN);
    assert_eq!(
        String::from_utf8_lossy(&c),
        "00000080030000000000000000000040\n",
        "[E3] unexpected C baseline"
    );
    assert_eq!(call_capture(&p.rust, i32::MIN), c);
}

/// E4: `floors == -1` -- all bits set, the classic C error sentinel passed *in*.
#[test]
fn err_e4_minus_one() {
    assert_same_both_ways("E4", &[-1]);
    let p = libs();
    let c = call_capture(&p.c, -1);
    assert_eq!(
        String::from_utf8_lossy(&c),
        "ffffffff030000000000000000000040\n",
        "[E4] unexpected C baseline"
    );
    assert_eq!(call_capture(&p.rust, -1), c);
}

/// E5: negative values in general -- a "count" that cannot exist is NOT
/// validated; the value is stored verbatim by both libraries.
#[test]
fn err_e5_negatives() {
    let mut rng = Rng::new(SEED ^ 0x6666_6666);
    let mut inputs: Vec<i32> = vec![-1, -2, -3, -7, -16, -255, -256, -65536, -1_000_000];
    inputs.extend((0..512).map(|_| -(rng.next_i32().wrapping_abs().max(1))));
    inputs.push(i32::MIN); // wrapping_abs of INT_MIN is INT_MIN
    assert_same("E5", &inputs);
}

/// E6: the caller passes 0x80000000 as an *unsigned* 32-bit value across the
/// FFI boundary; C reinterprets it as INT_MIN. Identical to E3.
#[test]
fn err_e6_unsigned_wraparound() {
    let p = libs();

    // Call through an `unsigned int` signature -- same ABI slot, no valid `int`.
    type UDriver = unsafe extern "C" fn(u32);
    let cf = p.c.driver();
    let rf = p.rust.driver();
    let cu: UDriver = unsafe { std::mem::transmute(*cf) };
    let ru: UDriver = unsafe { std::mem::transmute(*rf) };

    for u in [0x8000_0000u32, 0x8000_0001, 0xffff_ffff, 0xffff_fffe, 0x7fff_ffff] {
        let c = capture_fd1("e6c", || unsafe { cu(u) });
        let r = capture_fd1("e6r", || unsafe { ru(u) });
        assert_eq!(
            c, r,
            "[E6] divergence for unsigned {u:#010x}\n  C    : {:?}\n  Rust : {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        // And it must equal the signed reinterpretation.
        let signed = call_capture(&p.c, u as i32);
        assert_eq!(c, signed, "[E6] unsigned {u:#010x} != signed {}", u as i32);
    }
}

/// E7: out-of-range "enum-like" values. C enums accept any `int`, and
/// `driver`'s parameter has no variant set at all, so a value corresponding to
/// no meaningful variant is a real input both libraries must handle the same.
#[test]
fn err_e7_out_of_range_enum_values() {
    let mut inputs: Vec<i32> = vec![
        -999_999,
        -1_000_000_000,
        12_345_678,
        0x7fff_fffe,
        0x7fff_ffff,
        -0x7fff_ffff,
        1 << 30,
        (1i32 << 30).wrapping_neg(),
        // one step past every "plausible" small valid range
        -1, 0, 1, 2, 3, 4, 5, 100, 101, 255, 256, 257, 65_535, 65_536, 65_537,
    ];
    // Plus a swept range of arbitrary out-of-range ints.
    let mut rng = Rng::new(SEED ^ 0x7777_7777);
    inputs.extend((0..512).map(|_| rng.next_i32()));
    assert_same("E7", &inputs);
}

/// E8: inputs whose bytes contain embedded NULs -- would truncate any
/// string-based implementation; both must still emit all 16 bytes.
#[test]
fn err_e8_embedded_nul_bytes() {
    let inputs: Vec<i32> = [
        0x00ff_00ffu32,
        0xff00_ff00,
        0x0000_0100,
        0x0001_0000,
        0x0100_0000,
        0x0000_0000,
        0x00_00_00_01,
        0x01_00_00_00,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();

    assert_same_both_ways("E8", &inputs);

    // Every one of these still yields a full 33-byte line from both libs.
    let p = libs();
    for &x in &inputs {
        for lib in [&p.c, &p.rust] {
            assert_eq!(
                call_capture(lib, x).len(),
                33,
                "[E8] {} truncated output for driver({x})",
                lib.name
            );
        }
    }
}

/// E9: repeated / back-to-back invocation -- no init needed, no global state
/// to corrupt, both libraries stay in lockstep over a long run.
#[test]
fn err_e9_repeated_calls_no_state() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x8888_8888);
    let inputs: Vec<i32> = (0..1024).map(|_| rng.next_i32()).collect();

    // Interleave the two libraries call-for-call in one capture window each.
    for &x in inputs.iter().take(64) {
        let c = call_capture(&p.c, x);
        let r = call_capture(&p.rust, x);
        assert_eq!(c, r, "[E9] divergence for driver({x}) while interleaving");
    }

    // Then a long uninterrupted run of each.
    assert_same("E9", &inputs);
}

/// E10: `print_hex`'s `len` is never caller-controlled (`sizeof(house_t)` is
/// hardcoded), so 0 / negative / oversized lengths are unreachable from the
/// public API. Verified as an invariant: the line is ALWAYS exactly 33 bytes,
/// for every input, in both libraries. This also stands in for E11 -- the
/// public API takes no pointers, so NULL is not expressible.
#[test]
fn err_e10_output_length_invariant() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x9999_9999);
    let mut inputs: Vec<i32> = vec![0, -1, 1, i32::MIN, i32::MAX];
    inputs.extend((0..1024).map(|_| rng.next_i32()));

    for lib in [&p.c, &p.rust] {
        let out = call_capture_batch(lib, &inputs);
        for (i, x) in inputs.iter().enumerate() {
            assert_eq!(
                out[i].len(),
                33,
                "[E10] {} produced a {}-byte line for driver({x})",
                lib.name,
                out[i].len()
            );
        }
    }

    // The public API's arity/signature is pointer-free (E11): `driver` takes a
    // single `int`. Asserted against the header so it cannot silently change.
    let hdr = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .unwrap();
    assert!(
        hdr.contains("void driver(int x);"),
        "[E11] public API signature changed -- re-derive the pointer/NULL rows"
    );
    assert_eq!(
        hdr.matches('*').count(),
        0,
        "[E11] a pointer appeared in the public header -- add NULL-pointer rows"
    );
}

// ===========================================================================
// Phase D -- symbol parity through dlsym
// ===========================================================================

/// Every symbol the C `.so` exports must be resolvable in the Rust `.so` under
/// the exact same name.
#[test]
fn sym_parity_c_exports_are_all_in_rust() {
    let p = libs();
    for sym in c_dynamic_symbols() {
        assert!(
            p.rust.has_symbol(sym.as_bytes()),
            "Rust .so is missing symbol `{sym}` exported by the C .so"
        );
    }
}

/// `print_hex` is `static` in C and must not become exported in Rust.
#[test]
fn sym_private_c_symbols_stay_private_in_rust() {
    let p = libs();
    for sym in ["print_hex", "house_t"] {
        assert!(
            !p.c.has_symbol(sym.as_bytes()),
            "sanity: C .so unexpectedly exports `{sym}`"
        );
        assert!(
            !p.rust.has_symbol(sym.as_bytes()),
            "Rust .so exports `{sym}` which is file-local in C"
        );
    }
}

/// Read the C `.so`'s exported dynamic symbols with `nm -D --defined-only`.
fn c_dynamic_symbols() -> Vec<String> {
    let path = c_so_path();
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(&path)
        .output()
        .expect("failed to run `nm`");
    assert!(out.status.success(), "nm failed: {:?}", out);

    let text = String::from_utf8_lossy(&out.stdout);
    let syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only real definitions: text/data/bss/rodata, not weak-undefined.
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();

    assert!(
        syms.iter().any(|s| s == "driver"),
        "sanity: `driver` not found in C .so exports: {syms:?}"
    );
    syms
}
