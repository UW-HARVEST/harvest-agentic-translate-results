//! Phase B — valid-path differential tests, GATED on `CONFIGS.md`.
//!
//! One test per row of `CONFIGS.md`. Every case loads both `.so`s through
//! `libloading` and compares stdout byte-for-byte. Rows that take input use
//! many randomized values (fixed seed) rather than one hand-picked one.
//!
//! Tests are grouped so that residue-sensitive rows (`bad`, `driver(0)`) each
//! get a *fresh process*, because their output is a function of the call-site
//! stack state.

mod common;

use common::{fixture, Rng, SEED};

/// Random printable ASCII of length `len` (no NUL, no `\n`), hex-encoded.
fn rand_printable(rng: &mut Rng, len: usize) -> String {
    let bytes: Vec<u8> = (0..len).map(|_| 32 + rng.below(95) as u8).collect();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Rows 1-10: printLine — the lowest-level entry point, driven directly
// ---------------------------------------------------------------------------

#[test]
fn row01_printline_empty_lazy() {
    fixture().assert_same("1", "lazy", "p:");
}

#[test]
fn row02_printline_single_byte_lazy() {
    fixture().assert_same("2", "lazy", "p:61");
}

#[test]
fn row03_printline_short_ascii_lazy() {
    fixture().assert_same("3", "lazy", "p:737472696e67");
}

#[test]
fn row04_printline_randomized_strings_lazy() {
    let f = fixture();
    let mut rng = Rng::new(SEED);
    for i in 0..64 {
        let len = rng.below(256) as usize;
        let ops = format!("p:{}", rand_printable(&mut rng, len));
        f.assert_same(&format!("4[i={i},len={len}]"), "lazy", &ops);
    }
}

#[test]
fn row05_printline_all_byte_values_lazy() {
    let f = fixture();
    // All 255 non-NUL byte values, batched into one child (one call each).
    let ops: Vec<String> = (1u16..=255).map(|b| format!("p:{b:02x}")).collect();
    f.assert_same("5", "lazy", &ops.join(","));
}

#[test]
fn row06_printline_embedded_newlines_lazy() {
    // "a\nb\nc" — puts appends one more newline.
    fixture().assert_same("6", "lazy", "p:610a620a63");
}

#[test]
fn row07_printline_interior_nul_lazy() {
    // "ab\0cd" — output must truncate at the NUL.
    fixture().assert_same("7", "lazy", "p:61620063 64".replace(' ', "").as_str());
}

#[test]
fn row08_printline_oversized_1mib_lazy() {
    fixture().assert_same("8", "lazy", "pr:78:1048576");
}

#[test]
fn row09_printline_all_shapes_now() {
    let f = fixture();
    let mut rng = Rng::new(SEED);
    let mut cases: Vec<String> = vec![
        "p:".into(),
        "p:61".into(),
        "p:737472696e67".into(),
        "p:610a620a63".into(),
        "p:6162006364".into(),
        "pr:78:1048576".into(),
    ];
    for _ in 0..16 {
        let len = rng.below(256) as usize;
        cases.push(format!("p:{}", rand_printable(&mut rng, len)));
    }
    for (i, ops) in cases.iter().enumerate() {
        f.assert_same(&format!("9[i={i}]"), "now", ops);
    }
}

#[test]
fn row10_printline_repeated_calls_lazy() {
    fixture().assert_same(
        "10",
        "lazy",
        "p:61,p:6262,p:,p:636363,pr:7a:4096,p:64",
    );
}

// ---------------------------------------------------------------------------
// Rows 11-13: good
// ---------------------------------------------------------------------------

#[test]
fn row11_good_fresh_lazy() {
    fixture().assert_same("11", "lazy", "g");
}

#[test]
fn row12_good_fresh_now() {
    fixture().assert_same("12", "now", "g");
}

#[test]
fn row13_good_repeated_lazy() {
    fixture().assert_same("13", "lazy", "g,g,g");
}

// ---------------------------------------------------------------------------
// Rows 14-20: bad — the CWE-457 path
// ---------------------------------------------------------------------------
//
// Two flavours, because `bad()`'s output has two different provenances:
//
//   * **primed** (`tg`, `tp:`, `tn`): the slot was last written by the previous
//     *library* call, so the forwarded pointer is library-determined and must be
//     byte-identical. Asserted exactly.
//   * **unprimed** (`b`): the slot was last written by `dlopen`, so the pointer
//     is a function of the object's load footprint. Two behaviourally identical
//     C builds already disagree here (see `phase_b_residue_control.rs`), so
//     everything *except* those lines is required to match.

#[test]
fn row14_bad_fresh_lazy() {
    let f = fixture();
    f.assert_same_except_loader_residue("14", "lazy", "b", 1);
    // Primed: fully comparable, and pinned to an absolute expected value.
    f.assert_same("14/primed", "lazy", "tg");
    assert_eq!(f.c_output("lazy", "tg"), b"string\nstring\n".to_vec());
}

#[test]
fn row15_bad_fresh_now() {
    let f = fixture();
    f.assert_same_except_loader_residue("15", "now", "b", 1);
    f.assert_same("15/primed", "now", "tg");
}

#[test]
fn row16_bad_after_good_lazy() {
    // `good()` stores its "string" pointer in the slot `bad()` then reads.
    let f = fixture();
    f.assert_same("16", "lazy", "tg");
    assert_eq!(
        f.c_output("lazy", "tg"),
        b"string\nstring\n".to_vec(),
        "bad() must forward the pointer good() left in the shared slot"
    );
    assert_eq!(f.rust_output("lazy", "tg"), b"string\nstring\n".to_vec());
    f.assert_same_except_loader_residue("16/unprimed", "lazy", "g,b", 1);
}

#[test]
fn row17_bad_after_good_now() {
    let f = fixture();
    f.assert_same("17", "now", "tg");
    assert_eq!(f.c_output("now", "tg"), b"string\nstring\n".to_vec());
    f.assert_same_except_loader_residue("17/unprimed", "now", "g,b", 1);
}

#[test]
fn row18_bad_after_printline_lazy() {
    // `printLine` spills its parameter into the same slot.
    let f = fixture();
    for (hex, want) in [
        ("666f6f", "foo\nfoo\n"),
        ("", "\n\n"),
        ("61", "a\na\n"),
        ("6162636465666768", "abcdefgh\nabcdefgh\n"),
    ] {
        let ops = format!("tp:{hex}");
        f.assert_same(&format!("18[{hex}]"), "lazy", &ops);
        assert_eq!(f.c_output("lazy", &ops), want.as_bytes().to_vec());
        assert_eq!(f.rust_output("lazy", &ops), want.as_bytes().to_vec());
    }
    // printLine(NULL) primes the slot with NULL: the guard then rejects twice.
    f.assert_same("18/null", "lazy", "tn");
    assert_eq!(f.c_output("lazy", "tn"), Vec::<u8>::new());
    f.assert_same_except_loader_residue("18/unprimed", "lazy", "p:666f6f,b", 1);
}

#[test]
fn row19_bad_repeated_lazy() {
    let f = fixture();
    f.assert_same_except_loader_residue("19", "lazy", "b,b", 2);
    f.assert_same("19/primed", "lazy", "tg,tg,tg");
}

#[test]
fn row20_bad_repeated_now() {
    let f = fixture();
    f.assert_same_except_loader_residue("20", "now", "b,b", 2);
    f.assert_same("20/primed", "now", "tg,tg,tg");
}

// ---------------------------------------------------------------------------
// Rows 21-32: driver — the top-level wrapper, both branches
// ---------------------------------------------------------------------------

#[test]
fn row21_driver_true_lazy() {
    fixture().assert_same("21", "lazy", "d:1");
}

#[test]
fn row22_driver_true_now() {
    fixture().assert_same("22", "now", "d:1");
}

#[test]
fn row23_driver_false_first_call_lazy() {
    // First PLT call runs _dl_runtime_resolve, which clobbers the slot.
    fixture().assert_same("23", "lazy", "d:0");
}

#[test]
fn row24_driver_false_first_call_now() {
    // RTLD_NOW: resolver does not run, so different residue is read.
    fixture().assert_same("24", "now", "d:0");
}

#[test]
fn row25_driver_false_twice_lazy() {
    // First vs subsequent invocation differ; both must match the C.
    fixture().assert_same("25", "lazy", "d:0,d:0");
}

#[test]
fn row26_driver_false_twice_now() {
    fixture().assert_same("26", "now", "d:0,d:0");
}

#[test]
fn row27_driver_boundary_nonzero_lazy() {
    let f = fixture();
    for v in ["-1", "2", "-2147483648", "2147483647"] {
        f.assert_same(&format!("27[{v}]"), "lazy", &format!("d:{v}"));
    }
}

#[test]
fn row28_driver_boundary_nonzero_now() {
    let f = fixture();
    for v in ["-1", "2", "-2147483648", "2147483647"] {
        f.assert_same(&format!("28[{v}]"), "now", &format!("d:{v}"));
    }
}

#[test]
fn row29_driver_randomized_nonzero_lazy() {
    let f = fixture();
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let mut ops: Vec<String> = Vec::new();
    for _ in 0..64 {
        let mut v = rng.next_u64() as u32 as i32;
        if v == 0 {
            v = 1;
        }
        ops.push(format!("d:{v}"));
    }
    // Batched: 64 non-zero values must each take the good() branch.
    f.assert_same("29", "lazy", &ops.join(","));
}

#[test]
fn row30_driver_true_then_false_lazy() {
    fixture().assert_same("30", "lazy", "d:1,d:0");
}

#[test]
fn row31_driver_true_then_false_now() {
    fixture().assert_same("31", "now", "d:1,d:0");
}

#[test]
fn row32_driver_false_then_true_lazy() {
    fixture().assert_same("32", "lazy", "d:0,d:1");
}

// ---------------------------------------------------------------------------
// Rows 33-36: composed pipelines over a dirtied stack
// ---------------------------------------------------------------------------

fn random_program(rng: &mut Rng) -> String {
    let n = 1 + rng.below(8);
    let mut ops = Vec::new();
    for _ in 0..n {
        ops.push(match rng.below(7) {
            0 => "tg".to_string(),
            1 => "g".to_string(),
            2 => "d:0".to_string(),
            3 => format!("d:{}", (rng.next_u64() as u32 as i32) | 1),
            4 => {
                let len = rng.below(24) as usize;
                let bytes: Vec<u8> = (0..len).map(|_| 33 + rng.below(94) as u8).collect();
                format!(
                    "p:{}",
                    bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                )
            }
            5 => "tn".to_string(),
            _ => "pn".to_string(),
        });
    }
    ops.join(",")
}

#[test]
fn row33_mixed_random_sequences_lazy() {
    let f = fixture();
    let mut rng = Rng::new(SEED ^ 0xABCD);
    for i in 0..40 {
        let ops = random_program(&mut rng);
        f.assert_same(&format!("33[i={i}]"), "lazy", &ops);
    }
}

#[test]
fn row34_mixed_random_sequences_now() {
    let f = fixture();
    let mut rng = Rng::new(SEED ^ 0x1234);
    for i in 0..40 {
        let ops = random_program(&mut rng);
        f.assert_same(&format!("34[i={i}]"), "now", &ops);
    }
}

#[test]
fn row35_good_bad_interleaved_lazy() {
    let f = fixture();
    // Primed: the slot is alternately stored by good() and read by bad().
    f.assert_same("35", "lazy", "tg,tg,tg");
    assert_eq!(
        f.c_output("lazy", "tg,tg,tg"),
        b"string\nstring\nstring\nstring\nstring\nstring\n".to_vec()
    );
    f.assert_same_except_loader_residue("35/unprimed", "lazy", "g,b,g,b,g,b", 3);
}

/// Unprimed `bad()` mixed into random sequences: everything except the
/// loader-residue lines must still match byte-for-byte.
#[test]
fn row33b_mixed_random_sequences_with_unprimed_bad_lazy() {
    let f = fixture();
    let mut rng = Rng::new(SEED ^ 0x55AA);
    for i in 0..40 {
        let n = 1 + rng.below(6);
        let mut ops = Vec::new();
        let mut n_bad = 0usize;
        for _ in 0..n {
            match rng.below(5) {
                0 => {
                    ops.push("b".to_string());
                    n_bad += 1;
                }
                1 => ops.push("g".to_string()),
                2 => ops.push("d:0".to_string()),
                3 => ops.push(format!("d:{}", (rng.next_u64() as u32 as i32) | 1)),
                _ => ops.push("p:61".to_string()),
            }
        }
        // driver(0) also invokes bad(), but through driver, where the residue is
        // library-determined; only the direct calls are excused.
        f.assert_same_except_loader_residue(
            &format!("33b[i={i}]"),
            "lazy",
            &ops.join(","),
            n_bad,
        );
    }
}

#[test]
fn row36_driver_deep_sequence_now() {
    fixture().assert_same("36", "now", "d:1,d:1,d:0,d:0,d:1,d:0");
}
