//! Phase D — symbol parity + large-scale exhaustive differential sweeps.
//!
//! `nm -D` parity is asserted from inside the test suite so it cannot drift, and
//! the sweeps push far past the randomized coverage of Phase B by walking each
//! axis over a large contiguous range and over the whole `u32` domain by stride.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn nm_defined(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// Symbols the Rust `.so` legitimately adds: Rust/libc runtime scaffolding that
/// is not part of the translated API surface.
fn is_runtime_noise(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_R")
        || name.starts_with("__cxa")
        || name.starts_with("_ITM_")
        || name.starts_with("__gnu")
        || name.starts_with("_fini")
        || name.starts_with("_init")
        || name.starts_with("__bss")
        || name.starts_with("_edata")
        || name.starts_with("_end")
}

#[test]
fn d01_symbol_parity() {
    let c_so = c_so_path();
    let r_so = rust_so_path();

    let c_syms = nm_defined(&c_so);
    let r_syms = nm_defined(&r_so);

    // The C .so must export at least the documented API.
    assert!(
        c_syms.contains("update_frame_header"),
        "C .so lost update_frame_header?"
    );

    // EVERY symbol the C exports must also be exported by the Rust .so.
    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !r_syms.contains(*s) && !is_runtime_noise(s))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // And the Rust must not be missing it under a mangled name either.
    assert!(r_syms.contains("update_frame_header"));
}

/// Every symbol the Rust `.so` imports must actually resolve at load time.
///
/// `nm -D --undefined-only` lists all imports, but for a Rust cdylib that
/// legitimately includes the whole libgcc unwinder and a chunk of glibc, so an
/// allowlist would be guesswork. `ldd -r` performs real relocation processing
/// and reports only the symbols that genuinely fail to resolve — which is
/// exactly the signature of an untranslated C helper that nothing defines.
#[test]
fn d02_no_undefined_non_libc_symbols() {
    let r_so = rust_so_path();

    let out = Command::new("ldd").arg("-r").arg(&r_so).output().expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let unresolved: Vec<&str> = text
        .lines()
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            l.contains("undefined symbol") || l.contains("not found")
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved symbols (untranslated C?):\n{}",
        unresolved.join("\n")
    );

    // Sanity: the same check must be clean for the C .so too, otherwise the
    // check itself is broken.
    let c_so = c_so_path();
    let out = Command::new("ldd").arg("-r").arg(&c_so).output().expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let c_unresolved: Vec<&str> = text
        .lines()
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            l.contains("undefined symbol") || l.contains("not found")
        })
        .collect();
    assert!(c_unresolved.is_empty(), "C .so unresolved: {c_unresolved:?}");

    // Finally: no symbol the Rust .so imports may be one the C .so *defines*
    // (that would mean the Rust is calling back into untranslated C).
    let c_defined = nm_defined(&c_so);
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&r_so)
        .output()
        .expect("run nm");
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(name) = line.split_whitespace().next() {
            let bare = name.split('@').next().unwrap_or(name);
            assert!(
                !c_defined.contains(bare),
                "Rust .so imports {bare}, which is defined by the C .so — \
                 that C source was not translated"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Large-scale sweeps
// ---------------------------------------------------------------------------

/// Exhaustive over `0..=1_000_000` for each axis in turn.
#[test]
fn d03_exhaustive_low_million_per_axis() {
    let p = Pair::load();
    for axis in 0..4 {
        // A different quiet-ish baseline per axis so the swept field is the
        // only thing moving.
        let base = Tflac {
            samplerate: 65537,
            channels: 3,
            bitdepth: 16,
            channel_mode: 0,
            frame_header: 0xA5A5_5A5A,
            cur_blocksize: 4096,
        };
        for v in 0u32..=1_000_000 {
            let mut t = base;
            match axis {
                0 => t.cur_blocksize = v,
                1 => t.samplerate = v,
                2 => t.channels = v,
                _ => t.bitdepth = v,
            }
            let (c, r) = p.run(t);
            if c != r {
                panic!("axis {axis} value {v}: C {c:?} != Rust {r:?}");
            }
        }
    }
}

/// Whole `u32` domain per axis, by a large odd stride (hits every residue class
/// mod 1000 and mod 10, so it lands in all six samplerate sub-branches).
#[test]
fn d04_full_u32_stride_per_axis() {
    let p = Pair::load();
    const STRIDE: u32 = 9973; // prime, coprime with 1000 and 10
    for axis in 0..4 {
        let base = Tflac {
            samplerate: 44100,
            channels: 2,
            bitdepth: 24,
            channel_mode: 1,
            frame_header: 0,
            cur_blocksize: 1152,
        };
        let mut v: u32 = 0;
        loop {
            let mut t = base;
            match axis {
                0 => t.cur_blocksize = v,
                1 => t.samplerate = v,
                2 => t.channels = v,
                _ => t.bitdepth = v,
            }
            // channel_mode must be independent for the channels axis to matter.
            if axis == 2 {
                t.channel_mode = 0;
            }
            let (c, r) = p.run(t);
            if c != r {
                panic!("axis {axis} value {v}: C {c:?} != Rust {r:?}");
            }
            match v.checked_add(STRIDE) {
                Some(n) => v = n,
                None => break,
            }
        }
        // Always finish on the exact ceiling.
        let mut t = base;
        match axis {
            0 => t.cur_blocksize = u32::MAX,
            1 => t.samplerate = u32::MAX,
            2 => {
                t.channels = u32::MAX;
                t.channel_mode = 0;
            }
            _ => t.bitdepth = u32::MAX,
        }
        p.check(t);
    }
}

/// Exhaustive over the entire `channel_mode` byte crossed with every
/// interesting value of the other four fields.
#[test]
fn d05_channel_mode_exhaustive_cross() {
    let p = Pair::load();
    let bs = [0u32, 192, 256, 257, 4096, 32768, 32769, u32::MAX];
    let sr = [0u32, 8000, 44100, 65535, 65536, 255_000, 256_000, u32::MAX];
    let ch = [0u32, 1, 8, 9, 16, 17, u32::MAX];
    let bd = [0u32, 8, 16, 32, 33, u32::MAX];
    for m in 0u16..=255 {
        for &a in &bs {
            for &b in &sr {
                for &c in &ch {
                    for &d in &bd {
                        let t = Tflac {
                            samplerate: b,
                            channels: c,
                            bitdepth: d,
                            channel_mode: m as u8,
                            frame_header: 0xFFFF_FFFF,
                            cur_blocksize: a,
                        };
                        let (co, ro) = p.run(t);
                        if co != ro {
                            panic!("m={m} bs={a} sr={b} ch={c} bd={d}: {co:?} != {ro:?}");
                        }
                    }
                }
            }
        }
    }
}

/// TRULY exhaustive: every one of the 2^32 possible `samplerate` values.
///
/// `samplerate` has by far the most intricate logic (11 enumerated cases plus a
/// 4-deep nested `if`/`else if` chain with three division/modulo range checks),
/// so it is the axis where an off-by-one can hide at a value no random sample
/// would pick. Run with `--ignored` (it takes ~1 minute):
///
/// ```text
/// cargo test --release --test phase_d_parity -- --ignored --nocapture
/// ```
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn d06_samplerate_exhaustive_full_u32() {
    let p = Pair::load();
    let base = Tflac {
        samplerate: 0,
        channels: 5,
        bitdepth: 20,
        channel_mode: 0,
        frame_header: 0x1234_5678,
        cur_blocksize: 2304,
    };
    let mut v: u32 = 0;
    loop {
        let mut a = base;
        a.samplerate = v;
        let mut b = a;
        unsafe {
            (p.c)(&mut a);
            (p.rust)(&mut b);
        }
        if a != b {
            panic!("samplerate {v}: C {a:?} != Rust {b:?}");
        }
        if v == u32::MAX {
            break;
        }
        v += 1;
    }
    eprintln!("d06: swept all 2^32 samplerate values with 0 divergences");
}

/// TRULY exhaustive over `cur_blocksize` (2^32 values).
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn d07_blocksize_exhaustive_full_u32() {
    let p = Pair::load();
    let base = Tflac {
        samplerate: 48000,
        channels: 0, // exercise the underflow spill at the same time
        bitdepth: 12,
        channel_mode: 4, // out-of-range enum -> aliases to independent
        frame_header: 0xFFFF_FFFF,
        cur_blocksize: 0,
    };
    let mut v: u32 = 0;
    loop {
        let mut a = base;
        a.cur_blocksize = v;
        let mut b = a;
        unsafe {
            (p.c)(&mut a);
            (p.rust)(&mut b);
        }
        if a != b {
            panic!("cur_blocksize {v}: C {a:?} != Rust {b:?}");
        }
        if v == u32::MAX {
            break;
        }
        v += 1;
    }
    eprintln!("d07: swept all 2^32 cur_blocksize values with 0 divergences");
}

/// TRULY exhaustive over `channels` in independent mode (2^32 values) — the
/// axis whose `(channels - 1) << 4` underflows and shifts bits off the top.
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn d08_channels_exhaustive_full_u32() {
    let p = Pair::load();
    let base = Tflac {
        samplerate: 22050,
        channels: 0,
        bitdepth: 32,
        channel_mode: 0,
        frame_header: 0,
        cur_blocksize: 8192,
    };
    let mut v: u32 = 0;
    loop {
        let mut a = base;
        a.channels = v;
        let mut b = a;
        unsafe {
            (p.c)(&mut a);
            (p.rust)(&mut b);
        }
        if a != b {
            panic!("channels {v}: C {a:?} != Rust {b:?}");
        }
        if v == u32::MAX {
            break;
        }
        v += 1;
    }
    eprintln!("d08: swept all 2^32 channels values with 0 divergences");
}

/// TRULY exhaustive over `bitdepth` (2^32 values).
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn d09_bitdepth_exhaustive_full_u32() {
    let p = Pair::load();
    let base = Tflac {
        samplerate: 176400,
        channels: 7,
        bitdepth: 0,
        channel_mode: 255, // out-of-range enum -> aliases to mid/side
        frame_header: 0xDEAD_BEEF,
        cur_blocksize: 576,
    };
    let mut v: u32 = 0;
    loop {
        let mut a = base;
        a.bitdepth = v;
        let mut b = a;
        unsafe {
            (p.c)(&mut a);
            (p.rust)(&mut b);
        }
        if a != b {
            panic!("bitdepth {v}: C {a:?} != Rust {b:?}");
        }
        if v == u32::MAX {
            break;
        }
        v += 1;
    }
    eprintln!("d09: swept all 2^32 bitdepth values with 0 divergences");
}
