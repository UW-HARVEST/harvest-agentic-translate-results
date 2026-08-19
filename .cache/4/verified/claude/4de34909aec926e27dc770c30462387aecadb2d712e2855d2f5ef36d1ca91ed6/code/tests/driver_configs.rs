//! Phase B -- valid-path differential tests for the lowest-level entry point,
//! `void driver(char c)`, loaded from both shared objects.
//!
//! Covers rows 1-21 of CONFIGS.md.

mod common;

use common::*;

/// Exhaustive over `lo..=hi` (isolated child per value), then the whole range
/// batched in one child in shuffled order, then `draws` random draws from the
/// range batched in chunks.
fn check_range(label: &str, lo: u8, hi: u8, draws: usize) {
    for b in lo..=hi {
        assert_driver_same(&format!("{label}/single 0x{b:02x}"), b as i8);
    }

    let mut rng = Rng::new(SEED ^ ((lo as u64) << 16) ^ ((hi as u64) << 8) ^ draws as u64);

    let mut vals: Vec<u8> = (lo..=hi).collect();
    rng.shuffle(&mut vals);
    let ops: Vec<Op> = vals.iter().map(|&b| Op::Driver(b as i8)).collect();
    assert_same(&format!("{label}/batched-shuffled"), &ops, &Cfg::default());

    let span = (hi as usize) - (lo as usize) + 1;
    let mut batch: Vec<Op> = Vec::new();
    for i in 0..draws {
        let b = lo as usize + rng.below(span);
        batch.push(Op::Driver(b as u8 as i8));
        if batch.len() == 32 || i + 1 == draws {
            assert_same(&format!("{label}/random-batch@{i}"), &batch, &Cfg::default());
            batch.clear();
        }
    }
}

#[test]
fn cfg_01_cntrl_plain() {
    // 0x00..=0x08: control, nothing else.
    check_range("cfg_01", 0x00, 0x08, 64);
}

#[test]
fn cfg_02_tab() {
    // 0x09: the only blank control character.
    for _ in 0..16 {
        assert_driver_same("cfg_02/tab", 0x09);
    }
    assert_same(
        "cfg_02/tab-batched",
        &[Op::Driver(0x09); 8],
        &Cfg::default(),
    );
}

#[test]
fn cfg_03_cntrl_space() {
    // 0x0A..=0x0D: control + space, not blank.
    check_range("cfg_03", 0x0A, 0x0D, 32);
}

#[test]
fn cfg_04_cntrl_upper() {
    check_range("cfg_04", 0x0E, 0x1F, 64);
}

#[test]
fn cfg_05_space() {
    // 0x20: space + blank + print, but not graph.
    for _ in 0..16 {
        assert_driver_same("cfg_05/space", 0x20);
    }
}

#[test]
fn cfg_06_punct_low() {
    check_range("cfg_06", 0x21, 0x2F, 64);
}

#[test]
fn cfg_07_digits() {
    check_range("cfg_07", 0x30, 0x39, 64);
}

#[test]
fn cfg_08_punct_mid() {
    check_range("cfg_08", 0x3A, 0x40, 64);
}

#[test]
fn cfg_09_upper_hex() {
    check_range("cfg_09", 0x41, 0x46, 64);
}

#[test]
fn cfg_10_upper_nonhex() {
    check_range("cfg_10", 0x47, 0x5A, 64);
}

#[test]
fn cfg_11_punct_high() {
    check_range("cfg_11", 0x5B, 0x60, 64);
}

#[test]
fn cfg_12_lower_hex() {
    check_range("cfg_12", 0x61, 0x66, 64);
}

#[test]
fn cfg_13_lower_nonhex() {
    check_range("cfg_13", 0x67, 0x7A, 64);
}

#[test]
fn cfg_14_punct_top() {
    check_range("cfg_14", 0x7B, 0x7E, 64);
}

#[test]
fn cfg_15_del() {
    for _ in 0..16 {
        assert_driver_same("cfg_15/del", 0x7Fu8 as i8);
    }
}

#[test]
fn cfg_16_negative_chars_random() {
    // 0x80..=0xFF -- negative `char` values.
    let mut rng = Rng::new(SEED ^ 0x8080_8080);
    let mut batch: Vec<Op> = Vec::new();
    for i in 0..200 {
        let b = 0x80u8 + rng.below(0x80) as u8;
        assert_driver_same(&format!("cfg_16/single 0x{b:02x}"), b as i8);
        batch.push(Op::Driver(b as i8));
        if batch.len() == 25 || i == 199 {
            assert_same(&format!("cfg_16/batch@{i}"), &batch, &Cfg::default());
            batch.clear();
        }
    }
}

#[test]
fn cfg_17_exhaustive_all_chars() {
    // All 256 `char` values, isolated, in randomised order.
    let mut vals: Vec<u8> = (0..=255u8).collect();
    let mut rng = Rng::new(SEED ^ 0xFFFF);
    rng.shuffle(&mut vals);
    for b in vals.iter().copied() {
        assert_driver_same(&format!("cfg_17/0x{b:02x}"), b as i8);
    }
    // ... and all of them inside one single process.
    let ops: Vec<Op> = vals.iter().map(|&b| Op::Driver(b as i8)).collect();
    for chunk in ops.chunks(64) {
        assert_same("cfg_17/one-process", chunk, &Cfg::default());
    }
}

#[test]
fn cfg_18_ffi_int_low_byte_only() {
    // The argument crosses the boundary in a register through a
    // `void driver(int)` prototype: only the low byte may be significant.
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    let mut values: Vec<i32> = vec![
        0,
        1,
        -1,
        0xFF,
        0x100,
        0x1FF,
        0x1234_5641,
        -0x80,
        0xFFFF_FF80u32 as i32,
        i32::MIN,
        i32::MAX,
        0x7FFF_FF00,
    ];
    for _ in 0..128 {
        values.push(rng.next_i32());
    }

    for v in values {
        let label = format!("cfg_18/int 0x{:08x}", v as u32);
        let wide = assert_same(&label, &[Op::DriverInt(v)], &Cfg::default());
        // Both implementations must also agree with the plain `char` call for
        // the low byte of the value.
        let narrow = assert_driver_same(&label, (v & 0xFF) as u8 as i8);
        assert_eq!(
            wide.stdout,
            narrow.stdout,
            "int argument 0x{:08x} must behave like char 0x{:02x}",
            v as u32,
            (v & 0xFF) as u8
        );
    }
}

#[test]
fn cfg_19_many_calls_one_process() {
    // Repeated calls in one process: repeated setlocale(), shared stdio.
    let mut rng = Rng::new(SEED ^ 0xABCD);
    for round in 0..8 {
        let ops: Vec<Op> = (0..64).map(|_| Op::Driver(rng.byte() as i8)).collect();
        assert_same(&format!("cfg_19/round {round}"), &ops, &Cfg::default());
    }
}

#[test]
fn cfg_20_host_locale_variants() {
    // The caller has a different locale active; driver() forces "C".
    let mut rng = Rng::new(SEED ^ 0x10CA1E);
    for loc in ["C", "POSIX", "C.utf8", "en_US.utf8", "en_US.iso88591", "de_DE.iso88591"] {
        if !locale_available(loc) {
            continue;
        }
        let cfg = Cfg::default().with_locale(loc);
        for _ in 0..24 {
            let b = rng.byte();
            assert_same(
                &format!("cfg_20/{loc} 0x{b:02x}"),
                &[Op::Driver(b as i8)],
                &cfg,
            );
        }
        // High-bit bytes are alphabetic in the ISO-8859 locales; the forced "C"
        // locale must win.
        let ops: Vec<Op> = (0x80u8..=0xFF).step_by(7).map(|b| Op::Driver(b as i8)).collect();
        assert_same(&format!("cfg_20/{loc}/high"), &ops, &cfg);
    }
}

#[test]
fn cfg_21_stdout_is_pipe() {
    // Non-seekable stdout: different stdio buffering path.
    let mut rng = Rng::new(SEED ^ 0x9191);
    let cfg = Cfg::default().with_stdout(StdoutSpec::Pipe);
    for _ in 0..64 {
        let b = rng.byte();
        assert_same(
            &format!("cfg_21/pipe 0x{b:02x}"),
            &[Op::Driver(b as i8)],
            &cfg,
        );
    }
    let ops: Vec<Op> = (0..48).map(|_| Op::Driver(rng.byte() as i8)).collect();
    assert_same("cfg_21/pipe-batched", &ops, &cfg);
}
