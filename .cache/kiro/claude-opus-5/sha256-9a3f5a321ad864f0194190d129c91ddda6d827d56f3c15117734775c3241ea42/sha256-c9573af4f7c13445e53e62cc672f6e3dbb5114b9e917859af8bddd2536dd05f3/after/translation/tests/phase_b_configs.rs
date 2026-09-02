//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` (loaded with `libloading`) with many randomized inputs from a
//! fixed-seed PRNG and compares the results byte-for-byte.

mod harness;

use harness::*;
use std::ffi::c_char;

const SEED: u64 = 0x5DEECE66D;
const ITERS: usize = 512;

// ===========================================================================
// extractFilename (the low-level entry point) — rows 1..10
// ===========================================================================

#[test]
fn cfg_01_extract_no_separator() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..ITERS {
        let len = rng.range(0, 64);
        let path = cstr(&rand_bytes_without(&mut rng, len, b'/'));
        diff_extract(&path, b'/', &format!("row1 iter{i} len={len}"));
    }
}

#[test]
fn cfg_02_extract_one_separator() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..ITERS {
        let len = rng.range(1, 64);
        let mut b = rand_bytes_without(&mut rng, len, b'/');
        let at = rng.below(len);
        b[at] = b'/';
        let path = cstr(&b);
        diff_extract(&path, b'/', &format!("row2 iter{i} len={len} at={at}"));
    }
}

#[test]
fn cfg_03_extract_many_separators() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..ITERS {
        let len = rng.range(2, 64);
        let mut b = rand_bytes_without(&mut rng, len, b'/');
        let count = rng.range(2, len);
        for _ in 0..count {
            let at = rng.below(len);
            b[at] = b'/';
        }
        let path = cstr(&b);
        diff_extract(&path, b'/', &format!("row3 iter{i} len={len} n={count}"));
    }
}

#[test]
fn cfg_04_extract_trailing_separator() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..ITERS {
        let len = rng.range(0, 32);
        let mut b = rand_bytes(&mut rng, len);
        b.push(b'/');
        let path = cstr(&b);
        diff_extract(&path, b'/', &format!("row4 iter{i} len={len}"));
    }
}

#[test]
fn cfg_05_extract_leading_separator() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..ITERS {
        let len = rng.range(0, 48);
        let mut b = vec![b'/'];
        b.extend(rand_bytes_without(&mut rng, len, b'/'));
        let path = cstr(&b);
        diff_extract(&path, b'/', &format!("row5 iter{i} len={len}"));
    }
}

#[test]
fn cfg_06_extract_empty_path() {
    let path = cstr(b"");
    diff_extract(&path, b'/', "row6 empty path, sep '/'");
    // The whole separator domain against an empty path.
    for sep in 1u8..=255 {
        diff_extract(&path, sep, &format!("row6 empty path, sep {sep:#04x}"));
    }
}

#[test]
fn cfg_07_extract_random_ascii_separator() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..ITERS {
        let sep = (rng.range(1, 0x7F)) as u8;
        let len = rng.range(0, 64);
        let mut b = rand_bytes(&mut rng, len);
        // Half the time make sure the separator really occurs.
        if len > 0 && rng.bool() {
            let at = rng.below(len);
            b[at] = sep;
        }
        let path = cstr(&b);
        diff_extract(&path, sep, &format!("row7 iter{i} sep={sep:#04x} len={len}"));
    }
}

#[test]
fn cfg_08_extract_highbit_separator() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..ITERS {
        let sep = (rng.range(0x80, 0xFF)) as u8;
        let len = rng.range(0, 64);
        // Bias the payload toward high-bit bytes so matches actually happen.
        let mut b: Vec<u8> = (0..len)
            .map(|_| {
                if rng.bool() {
                    (rng.range(0x80, 0xFF)) as u8
                } else {
                    rng.nonzero_byte()
                }
            })
            .collect();
        if len > 0 && rng.bool() {
            let at = rng.below(len);
            b[at] = sep;
        }
        let path = cstr(&b);
        diff_extract(&path, sep, &format!("row8 iter{i} sep={sep:#04x} len={len}"));
    }
}

#[test]
fn cfg_09_extract_nul_separator() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..ITERS {
        let len = rng.range(1, 64);
        let path = cstr(&rand_bytes(&mut rng, len));
        // separator == '\0' matches the terminator: result is one-past-the-end.
        diff_extract(&path, 0, &format!("row9 iter{i} len={len}"));
    }
}

#[test]
fn cfg_10_extract_long_paths() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..128 {
        let len = rng.range(256, 1024);
        let b: Vec<u8> = (0..len)
            .map(|_| if rng.below(4) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let path = cstr(&b);
        diff_extract(&path, b'/', &format!("row10 iter{i} len={len}"));
    }
}

// ===========================================================================
// FIO_createFilename_fromOutDir — rows 11..24
// ===========================================================================

/// Random non-empty `outDirName` whose last byte is (or is not) `'/'`.
fn rand_out_dir(rng: &mut Rng, ends_with_slash: bool, min: usize, max: usize) -> Vec<u8> {
    let len = rng.range(min.max(1), max);
    let mut b = rand_bytes_without(rng, len - 1, b'/');
    b.push(if ends_with_slash {
        b'/'
    } else {
        rng.nonzero_byte_except(b'/')
    });
    cstr(&b)
}

/// Random `path` with no separator at all.
fn rand_plain_path(rng: &mut Rng, min: usize, max: usize) -> Vec<u8> {
    let len = rng.range(min, max);
    cstr(&rand_bytes_without(rng, len, b'/'))
}

/// Random `path` containing separators.
fn rand_nested_path(rng: &mut Rng, min: usize, max: usize) -> Vec<u8> {
    let len = rng.range(min.max(2), max);
    let mut b = rand_bytes_without(rng, len, b'/');
    let n = rng.range(1, 4);
    for _ in 0..n {
        let at = rng.below(len);
        b[at] = b'/';
    }
    cstr(&b)
}

#[test]
fn cfg_11_fio_outdir_slash_path_plain_suffix0() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..ITERS {
        let out = rand_out_dir(&mut rng, true, 1, 32);
        let path = rand_plain_path(&mut rng, 0, 32);
        diff_fio(&path, &out, 0, &format!("row11 iter{i}"));
    }
}

#[test]
fn cfg_12_fio_outdir_slash_path_nested_suffix0() {
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..ITERS {
        let out = rand_out_dir(&mut rng, true, 1, 32);
        let path = rand_nested_path(&mut rng, 2, 48);
        diff_fio(&path, &out, 0, &format!("row12 iter{i}"));
    }
}

#[test]
fn cfg_13_fio_outdir_plain_path_plain_suffix0() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..ITERS {
        let out = rand_out_dir(&mut rng, false, 1, 32);
        let path = rand_plain_path(&mut rng, 0, 32);
        diff_fio(&path, &out, 0, &format!("row13 iter{i}"));
    }
}

#[test]
fn cfg_14_fio_outdir_plain_path_nested_suffix0() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..ITERS {
        let out = rand_out_dir(&mut rng, false, 1, 32);
        let path = rand_nested_path(&mut rng, 2, 48);
        diff_fio(&path, &out, 0, &format!("row14 iter{i}"));
    }
}

#[test]
fn cfg_15_fio_empty_basename() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..ITERS {
        let ends = rng.bool();
        let out = rand_out_dir(&mut rng, ends, 1, 32);
        let n = rng.range(0, 24);
        let mut b = rand_bytes_without(&mut rng, n, b'/');
        b.push(b'/');
        let path = cstr(&b);
        let sfx = rng.range(0, 8);
        diff_fio(&path, &out, sfx, &format!("row15 iter{i} sfx={sfx}"));
    }
}

#[test]
fn cfg_16_fio_outdir_len1_both_branches() {
    let mut rng = Rng::new(SEED ^ 16);
    let slash = cstr(b"/");
    for i in 0..ITERS {
        let path = if rng.bool() {
            rand_plain_path(&mut rng, 0, 32)
        } else {
            rand_nested_path(&mut rng, 2, 32)
        };
        let sfx = rng.range(0, 8);
        diff_fio(&path, &slash, sfx, &format!("row16 iter{i} outdir=\"/\" sfx={sfx}"));
        let other = cstr(&[rng.nonzero_byte_except(b'/')]);
        diff_fio(&path, &other, sfx, &format!("row16 iter{i} outdir=1byte sfx={sfx}"));
    }
}

#[test]
fn cfg_17_fio_outdir_multi_component() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..ITERS {
        let parts = rng.range(2, 5);
        let mut b: Vec<u8> = Vec::new();
        for p in 0..parts {
            if p > 0 {
                b.push(b'/');
            }
            let n = rng.range(1, 8);
            b.extend(rand_bytes_without(&mut rng, n, b'/'));
        }
        if rng.bool() {
            b.push(b'/');
        }
        let out = cstr(&b);
        let path = if rng.bool() {
            rand_plain_path(&mut rng, 0, 32)
        } else {
            rand_nested_path(&mut rng, 2, 32)
        };
        let sfx = rng.range(0, 8);
        diff_fio(&path, &out, sfx, &format!("row17 iter{i} parts={parts} sfx={sfx}"));
    }
}

#[test]
fn cfg_18_fio_suffixlen_one() {
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..ITERS {
        for ends in [true, false] {
            let out = rand_out_dir(&mut rng, ends, 1, 32);
            let path = rand_nested_path(&mut rng, 2, 32);
            diff_fio(&path, &out, 1, &format!("row18 iter{i} ends={ends}"));
        }
    }
}

#[test]
fn cfg_19_fio_suffixlen_sweep() {
    let mut rng = Rng::new(SEED ^ 19);
    for sfx in 0..=64usize {
        for i in 0..16 {
            for ends in [true, false] {
                let out = rand_out_dir(&mut rng, ends, 1, 24);
                let path = if rng.bool() {
                    rand_plain_path(&mut rng, 0, 24)
                } else {
                    rand_nested_path(&mut rng, 2, 24)
                };
                diff_fio(&path, &out, sfx, &format!("row19 sfx={sfx} iter{i} ends={ends}"));
            }
        }
    }
}

#[test]
fn cfg_20_fio_suffixlen_large() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..16 {
        for ends in [true, false] {
            let sfx = rng.range(1 << 20, 4 << 20);
            let out = rand_out_dir(&mut rng, ends, 1, 32);
            let path = rand_nested_path(&mut rng, 2, 32);
            diff_fio(&path, &out, sfx, &format!("row20 iter{i} ends={ends} sfx={sfx}"));
        }
    }
}

#[test]
fn cfg_21_fio_empty_path() {
    let mut rng = Rng::new(SEED ^ 21);
    let empty = cstr(b"");
    for i in 0..ITERS {
        for ends in [true, false] {
            let out = rand_out_dir(&mut rng, ends, 1, 32);
            let sfx = rng.range(0, 16);
            diff_fio(&empty, &out, sfx, &format!("row21 iter{i} ends={ends} sfx={sfx}"));
        }
    }
}

#[test]
fn cfg_22_fio_highbit_bytes() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..ITERS {
        let hb = |rng: &mut Rng, n: usize| -> Vec<u8> {
            (0..n).map(|_| (rng.range(0x80, 0xFF)) as u8).collect()
        };
        for ends in [true, false] {
            let on = rng.range(1, 24);
            let mut o = hb(&mut rng, on);
            if ends {
                o.push(b'/');
            }
            let out = cstr(&o);
            let pn = rng.range(1, 32);
            let mut pb = hb(&mut rng, pn);
            if rng.bool() {
                let at = rng.below(pb.len());
                pb[at] = b'/';
            }
            let path = cstr(&pb);
            let sfx = rng.range(0, 16);
            diff_fio(&path, &out, sfx, &format!("row22 iter{i} ends={ends} sfx={sfx}"));
        }
    }
}

#[test]
fn cfg_23_fio_long_inputs() {
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..128 {
        let olen = rng.range(128, 512);
        let mut o: Vec<u8> = (0..olen)
            .map(|_| if rng.below(6) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let last = o.len() - 1;
        o[last] = if rng.bool() { b'/' } else { rng.nonzero_byte_except(b'/') };
        let out = cstr(&o);
        let plen = rng.range(128, 512);
        let p: Vec<u8> = (0..plen)
            .map(|_| if rng.below(5) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let path = cstr(&p);
        let sfx = rng.range(0, 4096);
        diff_fio(&path, &out, sfx, &format!("row23 iter{i} olen={olen} plen={plen} sfx={sfx}"));
    }
}

#[test]
fn cfg_24_fio_full_random_fuzz() {
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..4096 {
        // Fully random over every axis at once.
        let olen = rng.range(1, 40);
        let o: Vec<u8> = (0..olen)
            .map(|_| if rng.below(5) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let out = cstr(&o);
        let plen = rng.range(0, 40);
        let p: Vec<u8> = (0..plen)
            .map(|_| if rng.below(5) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let path = cstr(&p);
        let sfx = rng.range(0, 256);
        diff_fio(&path, &out, sfx, &format!("row24 iter{i}"));
    }
}

#[test]
fn cfg_25_pipeline_extract_then_fio() {
    // A real consumer composing the two entry points: take the basename with
    // the low-level `extractFilename`, then feed that pointer straight into
    // `FIO_createFilename_fromOutDir`. Both libraries must agree at each hop,
    // and the second hop is driven with the pointer *the C library returned*.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..ITERS {
        let plen = rng.range(1, 48);
        let pb: Vec<u8> = (0..plen)
            .map(|_| if rng.below(4) == 0 { b'/' } else { rng.nonzero_byte() })
            .collect();
        let path = cstr(&pb);
        let sep = if rng.bool() { b'/' } else { rng.nonzero_byte() };
        diff_extract(&path, sep, &format!("row25 iter{i} hop1"));

        let mid_c = unsafe { (p.c.extract_filename)(path.as_ptr() as *const c_char, sep as c_char) };
        let mid_r =
            unsafe { (p.rs.extract_filename)(path.as_ptr() as *const c_char, sep as c_char) };
        assert_eq!(mid_c as usize, mid_r as usize, "row25 iter{i}: hop1 pointer");

        let ends = rng.bool();
        let out = rand_out_dir(&mut rng, ends, 1, 24);
        let sfx = rng.range(0, 32);
        // Size of the allocation the C code will make for this exact input.
        let fstart = unsafe { (p.c.extract_filename)(mid_c, b'/' as c_char) };
        let n = unsafe {
            let odl = c_strlen(out.as_ptr() as *const c_char);
            odl + 1 + c_strlen(fstart) + sfx + 1
        };
        diff_fio_ptr(mid_c, out.as_ptr() as *const c_char, sfx, n, &format!("row25 iter{i} hop2"));
    }
}
