//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH `.so`s via `libloading` and compares their outputs
//! byte-for-byte over MANY randomized inputs from a fixed-seed PRNG.

mod common;

use common::*;
use std::os::raw::c_char;

const SEED: u64 = 0x5EED_1234_ABCD_0001;
const N: usize = 2000;

/// Alphabet with no `/` — guarantees "separator absent".
const NO_SEP: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-+~";

// =========================================================================
// extractFilename — the LOW-LEVEL entry point, driven directly
// =========================================================================

/// Row 1: sep `'/'`, path without any separator (`strrchr` -> NULL).
#[test]
fn cfg_01_extract_no_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let len = rng.range(0, 64);
        let path = rand_cstring(&mut rng, len, NO_SEP);
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off, 0, "separator absent must return `path` itself");
    }
}

/// Row 2: sep `'/'`, exactly one separator at a random position.
#[test]
fn cfg_02_extract_one_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let len = rng.range(1, 64);
        let mut path = rand_cstring(&mut rng, len, NO_SEP);
        let pos = rng.below(len);
        path[pos] = b'/';
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off as usize, pos + 1);
    }
}

/// Row 3: sep `'/'`, many separators at random positions.
#[test]
fn cfg_03_extract_many_separators() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let len = rng.range(1, 96);
        let mut path = rand_cstring(&mut rng, len, NO_SEP);
        let k = rng.range(1, 8);
        for _ in 0..k {
            let pos = rng.below(len);
            path[pos] = b'/';
        }
        let last = path[..len].iter().rposition(|&b| b == b'/').unwrap();
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off as usize, last + 1);
    }
}

/// Row 4: sep `'/'` as the FIRST byte.
#[test]
fn cfg_04_extract_leading_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let len = rng.range(0, 64);
        let tail = rand_cstring(&mut rng, len, NO_SEP);
        let mut path = vec![b'/'];
        path.extend_from_slice(&tail);
        diff_extract(&c, &r, &path, b'/');
    }
}

/// Row 5: sep `'/'` as the LAST byte -> empty tail.
#[test]
fn cfg_05_extract_trailing_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        let len = rng.range(0, 64);
        let mut path = rand_cstring(&mut rng, len, NO_SEP);
        path.pop(); // drop NUL
        path.push(b'/');
        path.push(0);
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off as usize, len + 1, "tail must be the empty string");
    }
}

/// Row 6: path consisting ONLY of separators.
#[test]
fn cfg_06_extract_only_separators() {
    let (c, r) = both();
    for n in 1..=64usize {
        let mut path = vec![b'/'; n];
        path.push(0);
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off as usize, n);
    }
}

/// Row 7: sep `'\\'` (the Windows separator used as a *value* on Linux).
#[test]
fn cfg_07_extract_backslash_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 7);
    let alpha = b"ab\\/cd";
    for _ in 0..N {
        let len = rng.range(0, 80);
        let path = rand_cstring(&mut rng, len, alpha);
        diff_extract(&c, &r, &path, b'\\');
        // the same buffer under the '/' separator, for good measure
        diff_extract(&c, &r, &path, b'/');
    }
}

/// Row 8: an ARBITRARY separator byte `1..=255` over full-byte-range paths.
#[test]
fn cfg_08_extract_arbitrary_separator_byte() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..(N * 4) {
        let len = rng.range(0, 48);
        let path = rand_cstring_full(&mut rng, len);
        let sep = rng.nonzero_byte();
        diff_extract(&c, &r, &path, sep);
    }
    // exhaustive over every separator byte value on one fixed buffer
    let mut rng = Rng::new(SEED ^ 0x88);
    let path = rand_cstring_full(&mut rng, 200);
    for sep in 1u16..=255 {
        diff_extract(&c, &r, &path, sep as u8);
    }
}

/// Row 9: high / negative (sign-extended) separator bytes `0x80..=0xFF`.
#[test]
fn cfg_09_extract_high_separator_bytes() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 9);
    // an alphabet packed with high bytes so hits are frequent
    let alpha: Vec<u8> = (0x7Du8..=0xFF).collect();
    for _ in 0..(N * 2) {
        let len = rng.range(0, 64);
        let path = rand_cstring(&mut rng, len, &alpha);
        let sep = rng.range(0x80, 0xFF) as u8;
        diff_extract(&c, &r, &path, sep);
    }
    // the signed-char boundary explicitly
    for sep in [0x7Eu8, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
        let mut path = vec![0x7F, 0x80, 0x81, b'a', 0xFF, 0xFE, 0x80, b'z'];
        path.push(0);
        diff_extract(&c, &r, &path, sep);
    }
}

/// Row 10: LONG paths (256..4096 bytes) with randomized separator density.
#[test]
fn cfg_10_extract_long_paths() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..300 {
        let len = rng.range(256, 4096);
        let mut path = rand_cstring(&mut rng, len, NO_SEP);
        let density = rng.range(1, 32);
        for i in 0..len {
            if rng.below(density) == 0 {
                path[i] = b'/';
            }
        }
        diff_extract(&c, &r, &path, b'/');
    }
}

/// Row 11: maximal separator density — bytes drawn only from `{'/', 'a'}`.
#[test]
fn cfg_11_extract_dense_separators() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..(N * 2) {
        let len = rng.range(0, 40);
        let path = rand_cstring(&mut rng, len, b"/a");
        diff_extract(&c, &r, &path, b'/');
    }
}

// =========================================================================
// FIO_createFilename_fromOutDir — the lib.h entry point, full pipeline
// =========================================================================

/// A random `outDirName` that does NOT end in `/` (axis E = "insert separator").
fn rand_dir_no_trailing(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range(lo, hi).max(1);
    rand_cstring(rng, len, NO_SEP)
}

/// A random `outDirName` that DOES end in `/` (axis E = "concatenate").
fn rand_dir_trailing(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range(lo, hi).max(1);
    let mut v = rand_cstring(rng, len, NO_SEP);
    v.pop();
    v.push(b'/');
    v.push(0);
    v
}

/// Row 12: dir without trailing `/`, path without separator, `suffixLen == 0`.
#[test]
fn cfg_12_create_insert_sep_plain_path() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..N {
        let dir = rand_dir_no_trailing(&mut rng, 1, 32);
        let path = rand_cstr(&mut rng, 0, 32, NO_SEP);
        diff_create(&c, &r, &path, &dir, 0);
    }
}

/// Row 13: dir without trailing `/`, path WITH separators, random `suffixLen`.
#[test]
fn cfg_13_create_insert_sep_nested_path() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        let dir = rand_dir_no_trailing(&mut rng, 1, 32);
        let path = rand_cstr(&mut rng, 0, 48, b"/abcXY.0");
        let suffix = rng.range(0, 64);
        diff_create(&c, &r, &path, &dir, suffix);
    }
}

/// Row 14: dir ending in `/`, path without separator, `suffixLen == 0`.
#[test]
fn cfg_14_create_concat_plain_path() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..N {
        let dir = rand_dir_trailing(&mut rng, 1, 32);
        let path = rand_cstr(&mut rng, 0, 32, NO_SEP);
        diff_create(&c, &r, &path, &dir, 0);
    }
}

/// Row 15: dir ending in `/`, path WITH separators, random `suffixLen`.
#[test]
fn cfg_15_create_concat_nested_path() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let dir = rand_dir_trailing(&mut rng, 1, 32);
        let path = rand_cstr(&mut rng, 0, 48, b"/abcXY.0");
        let suffix = rng.range(0, 64);
        diff_create(&c, &r, &path, &dir, suffix);
    }
}

/// Row 16: dir ending in MULTIPLE `/`.
#[test]
fn cfg_16_create_multiple_trailing_separators() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 16);
    for k in 1..=8usize {
        for _ in 0..200 {
            let mut dir = rand_cstr(&mut rng, 1, 16, NO_SEP);
            dir.pop();
            dir.extend(std::iter::repeat(b'/').take(k));
            dir.push(0);
            let path = rand_cstr(&mut rng, 0, 32, b"/abcXY");
            diff_create(&c, &r, &path, &dir, rng.range(0, 16));
        }
    }
}

/// Row 17: `outDirName == "/"` exactly — a single byte that IS the separator.
#[test]
fn cfg_17_create_dir_is_single_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 17);
    let dir = b"/\0";
    for _ in 0..N {
        let path = rand_cstr(&mut rng, 0, 40, b"/abcXY.0");
        diff_create(&c, &r, &path, dir, rng.range(0, 32));
    }
}

/// Row 18: `outDirName` == a single NON-separator byte (shortest non-empty).
#[test]
fn cfg_18_create_dir_single_byte() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 18);
    for b in 1u16..=255 {
        if b == b'/' as u16 {
            continue;
        }
        let dir = [b as u8, 0];
        let path = rand_cstr(&mut rng, 0, 24, b"/abcXY");
        diff_create(&c, &r, &path, &dir, rng.range(0, 8));
    }
}

/// Row 19: `path == ""` (empty filename) across BOTH axis-E branches.
#[test]
fn cfg_19_create_empty_path() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 19);
    let path = b"\0";
    for _ in 0..N {
        let d1 = rand_dir_no_trailing(&mut rng, 1, 24);
        let d2 = rand_dir_trailing(&mut rng, 1, 24);
        let suffix = rng.range(0, 32);
        diff_create(&c, &r, path, &d1, suffix);
        diff_create(&c, &r, path, &d2, suffix);
    }
}

/// Row 20: `path` ending in `/` -> `filenameStart` is empty, both axis-E branches.
#[test]
fn cfg_20_create_path_trailing_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..N {
        let mut path = rand_cstr(&mut rng, 0, 32, b"/abcXY");
        path.pop();
        path.push(b'/');
        path.push(0);
        let d1 = rand_dir_no_trailing(&mut rng, 1, 24);
        let d2 = rand_dir_trailing(&mut rng, 1, 24);
        let suffix = rng.range(0, 32);
        diff_create(&c, &r, &path, &d1, suffix);
        diff_create(&c, &r, &path, &d2, suffix);
    }
}

/// Row 21: `outDirName` with INNER separators, both axis-E branches.
#[test]
fn cfg_21_create_dir_inner_separators() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..N {
        // a dir full of '/' but guaranteed not to end with one
        let mut d1 = rand_cstr(&mut rng, 1, 32, b"/abc");
        let n1 = cstr_len(&d1);
        d1[n1 - 1] = b'a';
        // ... and the same shape ending with one
        let mut d2 = rand_cstr(&mut rng, 1, 32, b"/abc");
        let n2 = cstr_len(&d2);
        d2[n2 - 1] = b'/';
        let path = rand_cstr(&mut rng, 0, 40, b"/abcXY.0");
        let suffix = rng.range(0, 32);
        diff_create(&c, &r, &path, &d1, suffix);
        diff_create(&c, &r, &path, &d2, suffix);
    }
}

/// Row 22: large `suffixLen` — asserts the trailing `calloc` zero-fill matches
/// byte-for-byte, not merely the written prefix.
#[test]
fn cfg_22_create_large_suffixlen_zero_fill() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..400 {
        let dir = if rng.bool() {
            rand_dir_trailing(&mut rng, 1, 16)
        } else {
            rand_dir_no_trailing(&mut rng, 1, 16)
        };
        let path = rand_cstr(&mut rng, 0, 32, b"/abcXY");
        let suffix = rng.range(0, 4096);
        // diff_create compares the FULL allocation, i.e. incl. the zero tail
        diff_create(&c, &r, &path, &dir, suffix);
    }
}

/// Row 23: LONG `outDirName` and `path` (256..2048 bytes each).
#[test]
fn cfg_23_create_long_inputs() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..200 {
        let mut dir = rand_cstr(&mut rng, 256, 2048, NO_SEP);
        if rng.bool() {
            let n = cstr_len(&dir);
            dir[n - 1] = b'/';
        }
        let mut path = rand_cstr(&mut rng, 256, 2048, NO_SEP);
        let plen = cstr_len(&path);
        for i in 0..plen {
            if rng.below(16) == 0 {
                path[i] = b'/';
            }
        }
        diff_create(&c, &r, &path, &dir, rng.range(0, 256));
    }
}

/// Row 24: full-byte-range (`0x01..=0xFF`) content in BOTH `outDirName` and `path`.
#[test]
fn cfg_24_create_full_byte_range() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..(N * 2) {
        let mut dir = rand_cstr_full(&mut rng, 1, 32);
        if rng.below(3) == 0 {
            let n = cstr_len(&dir);
            dir[n - 1] = b'/';
        }
        let mut path = rand_cstr_full(&mut rng, 0, 40);
        let plen = cstr_len(&path);
        for i in 0..plen {
            if rng.below(6) == 0 {
                path[i] = b'/';
            }
        }
        diff_create(&c, &r, &path, &dir, rng.range(0, 48));
    }
}

/// Row 25: fully randomized fuzz across ALL axes at once.
#[test]
fn cfg_25_create_full_fuzz() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 25);
    let alphabets: [&[u8]; 5] = [NO_SEP, b"/a", b"/abcXY.0", b"ab\\/cd", b"/"];
    for _ in 0..20_000 {
        let dir_alpha = alphabets[rng.below(alphabets.len())];
        let path_alpha = alphabets[rng.below(alphabets.len())];
        let dir = if rng.below(8) == 0 {
            rand_cstr_full(&mut rng, 1, 20)
        } else {
            rand_cstr(&mut rng, 1, 20, dir_alpha)
        };
        let path = if rng.below(8) == 0 {
            rand_cstr_full(&mut rng, 0, 24)
        } else {
            rand_cstr(&mut rng, 0, 24, path_alpha)
        };
        let suffix = match rng.below(4) {
            0 => 0,
            1 => rng.range(1, 8),
            2 => rng.range(1, 512),
            _ => rng.range(1, 64),
        };
        diff_create(&c, &r, &path, &dir, suffix);
    }
}

// =========================================================================
// Composed / cross-entry-point rows
// =========================================================================

/// Row 26: the COMPOSED pipeline — the tail `extractFilename` reports must be
/// exactly the tail `FIO_createFilename_fromOutDir` appends, in both libraries.
#[test]
fn cfg_26_composed_extract_then_create() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..N {
        let path = rand_cstr(&mut rng, 0, 48, b"/abcXY.0");
        let concat = rng.bool();
        let dir = if concat {
            rand_dir_trailing(&mut rng, 1, 24)
        } else {
            rand_dir_no_trailing(&mut rng, 1, 24)
        };
        let suffix = rng.range(0, 32);

        // low-level step: both libraries agree on the tail
        let off = diff_extract(&c, &r, &path, b'/') as usize;
        let tail = &path[off..cstr_len(&path)];

        // composed step: both libraries agree on the joined result
        diff_create(&c, &r, &path, &dir, suffix);

        // and the composition is internally consistent in EACH library
        let n = alloc_size(&path, &dir, suffix);
        let dir_len = cstr_len(&dir);
        for api in [&c, &r] {
            // SAFETY: both strings are valid NUL-terminated buffers.
            let got = unsafe {
                create_bytes(
                    api,
                    path.as_ptr() as *const c_char,
                    dir.as_ptr() as *const c_char,
                    suffix,
                    n,
                )
            };
            let mut want = Vec::new();
            want.extend_from_slice(&dir[..dir_len]);
            if !concat {
                want.push(b'/');
            }
            want.extend_from_slice(tail);
            want.resize(n, 0);
            assert_eq!(
                got, want,
                "{}: composed pipeline inconsistent with extractFilename tail",
                api.name
            );
        }
    }
}

/// Row 27: cross-linked interior pointers — feed the pointer returned by ONE
/// library's `extractFilename` into the OTHER library's
/// `FIO_createFilename_fromOutDir`, both ways.
#[test]
fn cfg_27_cross_linked_interior_pointers() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..N {
        let path = rand_cstr(&mut rng, 1, 48, b"/abcXY.0");
        let dir = if rng.bool() {
            rand_dir_trailing(&mut rng, 1, 24)
        } else {
            rand_dir_no_trailing(&mut rng, 1, 24)
        };
        let suffix = rng.range(0, 32);
        let base = path.as_ptr() as *const c_char;

        // SAFETY: `path` is a valid NUL-terminated buffer; the pointers returned
        // by extractFilename point inside it, so they are valid C strings too.
        unsafe {
            let c_tail = (c.extract)(base, b'/' as i8);
            let r_tail = (r.extract)(base, b'/' as i8);
            assert_eq!(c_tail, r_tail, "interior pointers must be identical");

            // the C-derived interior pointer, driven through BOTH libraries
            let off = c_tail as usize - base as usize;
            let tail_slice = &path[off..];
            let n = alloc_size(tail_slice, &dir, suffix);
            diff_create_raw(
                &c,
                &r,
                c_tail,
                dir.as_ptr() as *const c_char,
                suffix,
                n,
                "cross-linked: C-derived interior pointer",
            );
            // and the Rust-derived one
            diff_create_raw(
                &c,
                &r,
                r_tail,
                dir.as_ptr() as *const c_char,
                suffix,
                n,
                "cross-linked: Rust-derived interior pointer",
            );
        }
    }
}

/// Row 28: the returned buffer must come from the SAME allocator in both
/// libraries — every call site above frees with libc `free()`; this row hammers
/// the allocate/free cycle so a mismatched allocator would abort the process.
#[test]
fn cfg_28_allocator_contract_free_roundtrip() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..5000 {
        let dir = if rng.bool() {
            rand_dir_trailing(&mut rng, 1, 40)
        } else {
            rand_dir_no_trailing(&mut rng, 1, 40)
        };
        let path = rand_cstr(&mut rng, 0, 40, b"/abcXY.0");
        // sizes spanning several glibc bins: fastbin, smallbin, large, mmap
        let suffix = match rng.below(4) {
            0 => rng.range(0, 8),
            1 => rng.range(64, 512),
            2 => rng.range(4096, 65536),
            _ => rng.range(200_000, 400_000),
        };
        diff_create(&c, &r, &path, &dir, suffix);
    }
}
