//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared libraries
//! through their exported C symbols (loaded with `libloading`) and compares the
//! results byte-for-byte. Randomized rows use the fixed seed below so failures
//! are reproducible.

mod common;

use common::*;
use std::ffi::c_char;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// extractFilename — the low-level entry point, called directly (not only
// through the FIO_createFilename_fromOutDir wrapper).
// ===========================================================================

/// Row 1: separator `'/'`, path contains no occurrence of it.
#[test]
fn cfg_01_extract_sep_absent() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..500 {
        let len = rng.below(65);
        let mut buf = rng.path_like(len, 0, false); // sep_density 0 ⇒ no '/'
        assert!(!buf.contains(&b'/'));
        buf.push(0);
        let off = diff_extract_filename(&buf, b'/', &format!("row1 iter{i}"));
        assert_eq!(off, 0, "C contract: absent separator returns `path`");
    }
}

/// Row 2: separator `'/'`, exactly one occurrence at a random position.
#[test]
fn cfg_02_extract_sep_once() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..500 {
        let len = rng.range(1, 64);
        let mut buf = rng.path_like(len, 0, false);
        let pos = rng.below(len);
        buf[pos] = b'/';
        buf.push(0);
        let off = diff_extract_filename(&buf, b'/', &format!("row2 iter{i}"));
        assert_eq!(off as usize, pos + 1);
    }
}

/// Row 3: separator `'/'`, many occurrences (last one wins — `strrchr`).
#[test]
fn cfg_03_extract_sep_many() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..500 {
        let len = rng.range(4, 64);
        let mut buf = rng.path_like(len, 0, false);
        let n = rng.range(2, 8);
        let mut last = 0usize;
        for _ in 0..n {
            let pos = rng.below(len);
            buf[pos] = b'/';
            if pos > last {
                last = pos;
            }
        }
        let expect = buf.iter().rposition(|&b| b == b'/').unwrap();
        buf.push(0);
        let off = diff_extract_filename(&buf, b'/', &format!("row3 iter{i}"));
        assert_eq!(off as usize, expect + 1);
    }
}

/// Row 4: path ends with the separator (empty trailing component).
#[test]
fn cfg_04_extract_sep_trailing() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..300 {
        let len = rng.below(48);
        let mut buf = rng.path_like(len, 20, false);
        buf.push(b'/');
        let expect = buf.len();
        buf.push(0);
        let off = diff_extract_filename(&buf, b'/', &format!("row4 iter{i}"));
        assert_eq!(off as usize, expect, "points at the NUL terminator");
    }
}

/// Row 5: leading separator, and paths made of separators only.
#[test]
fn cfg_05_extract_sep_leading_and_only() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..200 {
        let len = rng.below(32);
        let mut buf = vec![b'/'];
        buf.extend(rng.path_like(len, 0, false));
        buf.push(0);
        diff_extract_filename(&buf, b'/', &format!("row5 leading iter{i}"));
    }
    for n in 1..=16usize {
        let mut buf = vec![b'/'; n];
        buf.push(0);
        let off = diff_extract_filename(&buf, b'/', &format!("row5 only{n}"));
        assert_eq!(off as usize, n);
    }
}

/// Row 6: empty path against several separators.
#[test]
fn cfg_06_extract_empty_path() {
    let buf = [0u8];
    for sep in [b'/', b'a', 0u8, 0xFFu8] {
        let off = diff_extract_filename(&buf, sep, &format!("row6 sep=0x{sep:02x}"));
        if sep == 0 {
            assert_eq!(off, 1, "NUL is 'found' by strrchr");
        } else {
            assert_eq!(off, 0);
        }
    }
}

/// Row 7: `separator == '\0'` — `strrchr` finds the terminator, so the result is
/// a one-past-the-end pointer rather than `path`.
#[test]
fn cfg_07_extract_nul_separator() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..300 {
        let len = rng.below(64);
        let mut buf = rng.path_like(len, 25, false);
        let expect = buf.len() + 1;
        buf.push(0);
        let off = diff_extract_filename(&buf, 0, &format!("row7 iter{i}"));
        assert_eq!(off as usize, expect);
    }
}

/// Row 8: separators with the high bit set (negative `c_char`) — checks the
/// `(char)c` conversion `strrchr` performs.
#[test]
fn cfg_08_extract_high_bit_separator() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..500 {
        let len = rng.range(1, 48);
        let mut buf = rng.path_like(len, 0, true); // all bytes >= 0x80
        let sep = 0x80u8 | (rng.byte() & 0x7f);
        if rng.bool() {
            let pos = rng.below(len);
            buf[pos] = sep; // guarantee a hit sometimes
        }
        buf.push(0);
        diff_extract_filename(&buf, sep, &format!("row8 iter{i}"));
    }
}

/// Row 9: exhaustive over all 256 `separator` byte values.
#[test]
fn cfg_09_extract_all_separators_exhaustive() {
    let mut rng = Rng::new(SEED ^ 9);
    for sep in 0..=255u8 {
        for i in 0..8 {
            let len = rng.below(49);
            let mut buf: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
            // make sure `sep` really occurs in some of the buffers
            if sep != 0 && len > 0 && rng.bool() {
                let pos = rng.below(len);
                buf[pos] = sep;
            }
            buf.push(0);
            diff_extract_filename(&buf, sep, &format!("row9 sep=0x{sep:02x} iter{i}"));
        }
    }
}

/// Row 10: long random paths with random separators.
#[test]
fn cfg_10_extract_long_random() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..400 {
        let len = rng.range(256, 1024);
        let mut buf: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
        let sep = if rng.below(4) == 0 { rng.byte() } else { b'/' };
        let holes = rng.below(20);
        for _ in 0..holes {
            let pos = rng.below(len);
            buf[pos] = sep;
        }
        buf.push(0);
        diff_extract_filename(&buf, sep, &format!("row10 iter{i}"));
    }
}

// ===========================================================================
// FIO_createFilename_fromOutDir — the wrapper, driven end to end.
// ===========================================================================

/// Row 11: outDir without trailing `'/'`, plain filename, `suffixLen == 0`.
#[test]
fn cfg_11_fio_nosep_dir_plain_file() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..400 {
        let dir = rng.path_r(1, 32, 0, false);
        let file = rng.path_r(1, 32, 0, false);
        let out = diff_create_filename(&file, &dir, 0, &format!("row11 iter{i}"));
        let mut expect = dir.clone();
        expect.push(b'/');
        expect.extend(&file);
        expect.push(0);
        assert_eq!(out, expect, "row11 iter{i}");
    }
}

/// Row 12: outDir without trailing `'/'`, nested path with separators.
#[test]
fn cfg_12_fio_nosep_dir_nested_path() {
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..400 {
        let mut dir = rng.path_r(1, 24, 15, false);
        if dir.last() == Some(&b'/') {
            dir.pop();
            dir.push(b'x');
        }
        let file = rng.path_r(2, 48, 25, false);
        diff_create_filename(&file, &dir, 0, &format!("row12 iter{i}"));
    }
}

/// Row 13: outDir WITH trailing `'/'` (no extra separator must be inserted).
#[test]
fn cfg_13_fio_trailing_sep_dir_plain_file() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..400 {
        let mut dir = rng.path_r(0, 32, 10, false);
        dir.push(b'/');
        let file = rng.path_r(1, 32, 0, false);
        let out = diff_create_filename(&file, &dir, 0, &format!("row13 iter{i}"));
        let mut expect = dir.clone();
        expect.extend(&file);
        expect.push(0);
        expect.push(0); // the buffer is 1 byte longer than the trailing-sep payload
        assert_eq!(out, expect, "row13 iter{i}");
    }
}

/// Row 14: outDir WITH trailing `'/'` and nested path.
#[test]
fn cfg_14_fio_trailing_sep_dir_nested_path() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..400 {
        let mut dir = rng.path_r(0, 24, 20, false);
        dir.push(b'/');
        let file = rng.path_r(2, 48, 25, false);
        diff_create_filename(&file, &dir, 0, &format!("row14 iter{i}"));
    }
}

/// Row 15: separator-only output directories.
#[test]
fn cfg_15_fio_separator_only_outdir() {
    let mut rng = Rng::new(SEED ^ 15);
    for n in 1..=4usize {
        let dir = vec![b'/'; n];
        for i in 0..50 {
            let file = rng.path_b(32, 20, false);
            for suffix in [0usize, 1, 7] {
                diff_create_filename(&file, &dir, suffix, &format!("row15 n={n} iter{i}"));
            }
        }
    }
}

/// Row 16: `outDirName == ""` with the byte *preceding* it pinned to `'/'`.
/// The C reads `outDirName[strlen(outDirName)-1]` == `outDirName[SIZE_MAX]`,
/// i.e. the byte before the buffer; both libraries must read the same address.
#[test]
fn cfg_16_fio_empty_outdir_prev_sep() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..200 {
        // [ '/' , '\0' ] — pass a pointer to the NUL, so outDir is "" and
        // outDir[-1] == '/'.
        let dir_buf: Vec<u8> = vec![b'/', 0];
        let mut file = rng.path_b(32, 20, false);
        file.push(0);
        let suffix = rng.below(8);
        let flen = filename_component_len(&file[..file.len() - 1]);
        let size = expected_alloc_size(0, flen, suffix);
        let out = diff_create_filename_ptrs(
            file.as_ptr() as *const c_char,
            unsafe { (dir_buf.as_ptr() as *const c_char).add(1) },
            suffix,
            size,
            &format!("row16 iter{i}"),
        );
        // trailing-separator branch ⇒ no '/' inserted
        let tail = &file[file.len() - 1 - flen..file.len() - 1];
        assert_eq!(&out[..flen], tail, "row16 iter{i}");
    }
}

/// Row 17: `outDirName == ""` with the preceding byte pinned to a non-`'/'`
/// value (selects the separator-inserting branch).
#[test]
fn cfg_17_fio_empty_outdir_prev_nonsep() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..200 {
        let prev = rng.plain_byte();
        let dir_buf: Vec<u8> = vec![prev, 0];
        let mut file = rng.path_b(32, 20, false);
        file.push(0);
        let suffix = rng.below(8);
        let flen = filename_component_len(&file[..file.len() - 1]);
        let size = expected_alloc_size(0, flen, suffix);
        let out = diff_create_filename_ptrs(
            file.as_ptr() as *const c_char,
            unsafe { (dir_buf.as_ptr() as *const c_char).add(1) },
            suffix,
            size,
            &format!("row17 iter{i} prev=0x{prev:02x}"),
        );
        assert_eq!(out[0], b'/', "row17: separator inserted");
    }
}

/// Row 18: `path == ""` (empty filename component) against both outDir branches.
#[test]
fn cfg_18_fio_empty_path() {
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..200 {
        let mut dir = rng.path_r(1, 24, 15, false);
        if rng.bool() {
            dir.push(b'/');
        } else if dir.last() == Some(&b'/') {
            dir.pop();
            dir.push(b'y');
        }
        for suffix in [0usize, 1, 5] {
            diff_create_filename(b"", &dir, suffix, &format!("row18 iter{i}"));
        }
    }
}

/// Row 19: path ends with `'/'` ⇒ empty `filenameStart`.
#[test]
fn cfg_19_fio_path_trailing_sep() {
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..200 {
        let mut dir = rng.path_r(1, 24, 15, false);
        if rng.bool() {
            dir.push(b'/');
        } else if dir.last() == Some(&b'/') {
            dir.pop();
            dir.push(b'z');
        }
        let mut file = rng.path_r(1, 32, 20, false);
        file.push(b'/');
        for suffix in [0usize, 3] {
            diff_create_filename(&file, &dir, suffix, &format!("row19 iter{i}"));
        }
    }
}

/// Row 20: small non-zero `suffixLen` — the zero-padded tail must match too.
#[test]
fn cfg_20_fio_small_suffixlen() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..500 {
        let mut dir = rng.path_r(1, 24, 10, false);
        if rng.bool() {
            dir.push(b'/');
        }
        let file = rng.path_r(0, 32, 20, false);
        let suffix = rng.range(1, 32);
        diff_create_filename(&file, &dir, suffix, &format!("row20 iter{i}"));
    }
}

/// Row 21: large `suffixLen`.
#[test]
fn cfg_21_fio_large_suffixlen() {
    let mut rng = Rng::new(SEED ^ 21);
    for suffix in [1024usize, 4096, 65536] {
        for i in 0..20 {
            let mut dir = rng.path_r(1, 24, 10, false);
            if i % 2 == 0 {
                dir.push(b'/');
            }
            let file = rng.path_r(0, 32, 20, false);
            diff_create_filename(&file, &dir, suffix, &format!("row21 suffix={suffix} iter{i}"));
        }
    }
}

/// Row 22: non-UTF-8 / high-bit bytes in both arguments, including a high-bit
/// last byte of outDir (sign-extension in the `char` comparison on line 45).
#[test]
fn cfg_22_fio_high_bit_bytes() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..500 {
        let mut dir = rng.path_r(1, 24, 10, true);
        // ensure the last byte is a high-bit byte in half the cases
        if rng.bool() {
            let b = rng.plain_byte() | 0x80;
            *dir.last_mut().unwrap() = b;
        } else {
            dir.push(b'/');
        }
        let mut file = rng.path_r(1, 32, 20, true);
        if rng.bool() {
            file.push(b'/');
        }
        let suffix = rng.below(9);
        diff_create_filename(&file, &dir, suffix, &format!("row22 iter{i}"));
    }
}

/// Row 23: long inputs.
#[test]
fn cfg_23_fio_long_inputs() {
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..150 {
        let dir = rng.path_rand(1, 512);
        let file = rng.path_rand(1, 512);
        let suffix = rng.below(64);
        diff_create_filename(&file, &dir, suffix, &format!("row23 iter{i}"));
    }
}

/// Row 24: full randomized cross-product property test.
#[test]
fn cfg_24_fio_random_property() {
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2000 {
        let dir_len = match rng.below(6) {
            0 => 1,
            1 => rng.range(1, 4),
            2 => rng.range(1, 16),
            3 => rng.range(1, 64),
            4 => rng.range(1, 128),
            _ => rng.range(1, 8),
        };
        let high = rng.below(3) == 0;
        let mut dir = rng.path_d(dir_len, 40, high);
        match rng.below(4) {
            0 => dir.push(b'/'),
            1 => {
                if dir.last() == Some(&b'/') {
                    *dir.last_mut().unwrap() = b'q';
                }
            }
            _ => {}
        }
        let file_len = rng.below(96);
        let mut file = rng.path_dh(file_len, 50);
        if rng.below(5) == 0 {
            file.push(b'/');
        }
        let suffix = match rng.below(4) {
            0 => 0,
            1 => rng.below(8),
            2 => rng.below(128),
            _ => rng.below(2048),
        };
        diff_create_filename(&file, &dir, suffix, &format!("row24 iter{i}"));
    }
}

/// Row 25: composed pipeline — the low-level `extractFilename` result must be
/// consistent with the wrapper's output, cross-checked between the two
/// libraries in both directions (C's `extractFilename` vs Rust's wrapper and
/// vice versa).
#[test]
fn cfg_25_composed_pipeline_consistency() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..500 {
        let mut dir = { let h = rng.below(4) == 0; rng.path_r(1, 24, 10, h) };
        if rng.bool() {
            dir.push(b'/');
        }
        let mut file = { let h = rng.below(4) == 0; rng.path_r(0, 48, 25, h) };
        file.push(0);

        // low-level, both libraries
        let base = file.as_ptr() as *const c_char;
        let (c_ret, r_ret) = unsafe {
            (
                (p.c.extract_filename)(base, b'/' as c_char),
                (p.rust.extract_filename)(base, b'/' as c_char),
            )
        };
        let c_off = (c_ret as isize) - (base as isize);
        let r_off = (r_ret as isize) - (base as isize);
        assert_eq!(c_off, r_off, "row25 iter{i}: extractFilename offset");

        let flen = (file.len() - 1) - (c_off as usize);
        assert_eq!(flen, filename_component_len(&file[..file.len() - 1]));

        // full pipeline, both libraries, byte-identical
        let suffix = rng.below(16);
        let size = expected_alloc_size(dir.len(), flen, suffix);
        let mut dir_c = dir.clone();
        dir_c.push(0);
        let out = diff_create_filename_ptrs(
            base,
            dir_c.as_ptr() as *const c_char,
            suffix,
            size,
            &format!("row25 iter{i}"),
        );

        // the wrapper's payload must end with exactly the component the
        // low-level function pointed at
        let component = &file[c_off as usize..file.len() - 1];
        let payload_end = if dir.last() == Some(&b'/') {
            dir.len() + flen
        } else {
            dir.len() + 1 + flen
        };
        assert_eq!(&out[payload_end - flen..payload_end], component, "row25 iter{i}");
    }
}
