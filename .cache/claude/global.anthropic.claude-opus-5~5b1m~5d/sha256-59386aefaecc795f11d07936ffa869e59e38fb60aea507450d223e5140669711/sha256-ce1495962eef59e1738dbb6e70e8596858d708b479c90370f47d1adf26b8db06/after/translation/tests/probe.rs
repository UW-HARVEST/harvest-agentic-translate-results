//! Early smoke probe: the two highest-risk areas (NaN payload propagation and
//! signed-zero ternary asymmetry) before the full Phase B/C suites.

mod common;

use common::*;

#[test]
fn probe_symbols_load() {
    let (c, rust) = both();
    eprintln!("C    .so = {}", c.path.display());
    eprintln!("Rust .so = {}", rust.path.display());
}

#[test]
fn probe_basic() {
    let (c, rust) = both();
    for src in [
        [1.0f32, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.5, 0.25, 0.75],
        [0.2, 0.2, 0.2],
        [0.0, 0.0, 0.0],
    ] {
        let a = call_bits(c.f, &src);
        let b = call_bits(rust.f, &src);
        eprintln!("{src:?} -> C {a:#010x?} | Rust {b:#010x?}");
        assert_eq!(a, b, "diverged on {src:?}");
    }
}

#[test]
fn probe_signed_zero_and_nan() {
    let (c, rust) = both();
    let interesting: [[u32; 3]; 12] = [
        [0x0000_0000, 0x8000_0000, 0x8000_0000], // +0, -0, -0
        [0x8000_0000, 0x0000_0000, 0x8000_0000],
        [0x8000_0000, 0x8000_0000, 0x0000_0000],
        [0x8000_0000, 0x8000_0000, 0x8000_0000],
        [0x7FC0_0000, 0x3F00_0000, 0x3E80_0000], // NaN in r
        [0x3F00_0000, 0x7FC0_0000, 0x3E80_0000], // NaN in g
        [0x3F00_0000, 0x3E80_0000, 0x7FC0_0000], // NaN in b
        [0x7FD5_5555, 0x3F00_0000, 0x3E80_0000], // payload NaN in r
        [0x3F00_0000, 0x7FD5_5555, 0x3E80_0000], // payload NaN in g
        [0x3F00_0000, 0x3E80_0000, 0x7FD5_5555], // payload NaN in b
        [0x7F80_0001, 0x3F00_0000, 0x3E80_0000], // sNaN in r
        [0x7F80_0000, 0xFF80_0000, 0x3F00_0000], // +Inf, -Inf
    ];
    let mut bad = 0;
    for src in interesting {
        let a = call_bits_raw(c.f, &src);
        let b = call_bits_raw(rust.f, &src);
        let mark = if a == b { "ok  " } else { "DIFF" };
        if a != b {
            bad += 1;
        }
        eprintln!("{mark} src {src:#010x?} -> C {a:#010x?} | Rust {b:#010x?}");
    }
    assert_eq!(bad, 0, "{bad} divergences (see log above)");
}
