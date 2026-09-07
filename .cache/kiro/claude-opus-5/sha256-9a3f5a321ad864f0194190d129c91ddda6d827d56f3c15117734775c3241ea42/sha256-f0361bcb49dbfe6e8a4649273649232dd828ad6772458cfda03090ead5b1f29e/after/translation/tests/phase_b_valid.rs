//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//! Every row drives BOTH `.so`s through `libloading` and compares outputs
//! byte-for-byte over many randomized inputs (fixed seed, reproducible).

mod common;

use common::*;
use std::os::raw::c_int;

const ITERS: usize = 2000;

/// Alphabet with a healthy '/' density.
const ASCII_SLASH: &[u8] = b"abcdef/ghij/klmno/pqrs";
/// Alphabet with no '/' at all.
const ASCII_NOSLASH: &[u8] = b"abcdefghijklmnopqrstuvwxyz.-_0123456789";
/// Alphabet with no '\\' at all.
const ASCII_NOBACKSLASH: &[u8] = b"abcdefghijklmnop/qrstuvwxyz.-_";

fn rand_with_slash(rng: &mut Rng, len: usize) -> Vec<u8> {
    // Guarantee at least one '/' somewhere.
    let mut v = rand_from(rng, len.max(1), ASCII_NOSLASH);
    let at = rng.below(v.len());
    v[at] = b'/';
    v
}

fn rand_with_slash_range(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range(lo, hi);
    rand_with_slash(rng, len)
}

// =========================== extractFilename ===============================

// C1
#[test]
fn c1_extract_slash_present() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let path = cstr(&rand_with_slash(&mut rng, len));
        let off = diff_extract(&path, b'/');
        // sanity: must be just past the last '/'
        let last = path[..path.len() - 1].iter().rposition(|&c| c == b'/').unwrap();
        assert_eq!(off, last as isize + 1);
    }
    // plus a pass over the mixed-density alphabet
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let path = cstr(&rand_from(&mut rng, len, ASCII_SLASH));
        diff_extract(&path, b'/');
    }
}

// C2
#[test]
fn c2_extract_slash_absent_returns_same_pointer() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let path = cstr(&rand_from(&mut rng, len, ASCII_NOSLASH));
        let off = diff_extract(&path, b'/');
        assert_eq!(off, 0, "no-match branch must return the input pointer");
    }
}

// C3
#[test]
fn c3_extract_empty_path() {
    let path = cstr(b"");
    assert_eq!(diff_extract(&path, b'/'), 0);
    assert_eq!(diff_extract(&path, b'a'), 0);
    assert_eq!(diff_extract(&path, b'\\'), 0);
}

// C4
#[test]
fn c4_extract_len1() {
    assert_eq!(diff_extract(&cstr(b"/"), b'/'), 1);
    let mut rng = Rng::new(0xC004);
    for _ in 0..256 {
        let b = rng.nonnul_byte();
        if b == b'/' {
            continue;
        }
        let path = cstr(&[b]);
        assert_eq!(diff_extract(&path, b'/'), 0);
        assert_eq!(diff_extract(&path, b), 1);
    }
}

// C5
#[test]
fn c5_extract_leading_separator_only() {
    let mut rng = Rng::new(0xC005);
    for _ in 0..ITERS {
        let len = rng.range(0, 32);
        let mut v = vec![b'/'];
        v.extend_from_slice(&rand_from(&mut rng, len.max(1), ASCII_NOSLASH));
        let path = cstr(&v);
        assert_eq!(diff_extract(&path, b'/'), 1);
    }
}

// C6
#[test]
fn c6_extract_trailing_separator() {
    let mut rng = Rng::new(0xC006);
    for _ in 0..ITERS {
        let len = rng.range(0, 32);
        let mut v = rand_from(&mut rng, len, ASCII_NOSLASH);
        v.push(b'/');
        let path = cstr(&v);
        let off = diff_extract(&path, b'/');
        assert_eq!(off, path.len() as isize - 1, "empty filename expected");
    }
}

// C7
#[test]
fn c7_extract_many_and_adjacent_separators() {
    for p in [
        &b"a//b///c"[..],
        b"//",
        b"///",
        b"a//",
        b"//a",
        b"////////",
        b"/a/b/c/d/e/f/",
        b"a/b//c///d////e",
    ] {
        diff_extract(&cstr(p), b'/');
    }
    let mut rng = Rng::new(0xC007);
    for _ in 0..ITERS {
        let len = rng.range(1, 48);
        // heavy '/' density, guaranteeing adjacent runs
        let v: Vec<u8> = (0..len)
            .map(|_| if rng.below(3) == 0 { b'x' } else { b'/' })
            .collect();
        diff_extract(&cstr(&v), b'/');
    }
}

// C8
#[test]
fn c8_extract_long_paths() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..500 {
        let len = rng.range(256, 1024);
        let density = rng.range(1, 40);
        let v: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(density) == 0 {
                    b'/'
                } else {
                    ASCII_NOSLASH[rng.below(ASCII_NOSLASH.len())]
                }
            })
            .collect();
        diff_extract(&cstr(&v), b'/');
    }
}

// C9
#[test]
fn c9_extract_backslash_separator() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        // no backslash present
        let path = cstr(&rand_from(&mut rng, len, ASCII_NOBACKSLASH));
        assert_eq!(diff_extract(&path, b'\\'), 0);
    }
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let mut v = rand_from(&mut rng, len, ASCII_NOBACKSLASH);
        let at = rng.below(v.len());
        v[at] = b'\\';
        diff_extract(&cstr(&v), b'\\');
    }
    for p in [&b"C:\\dir\\file.txt"[..], b"\\\\server\\share\\f", b"a\\", b"\\"] {
        diff_extract(&cstr(p), b'\\');
        diff_extract(&cstr(p), b'/');
    }
}

// C10
#[test]
fn c10_extract_nul_separator() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let body = rand_from(&mut rng, len, ASCII_SLASH);
        let path = cstr(&body);
        let off = diff_extract(&path, 0);
        // strrchr finds the terminator; result is one byte past it.
        assert_eq!(off, body.len() as isize + 1);
    }
}

// C11
#[test]
fn c11_extract_full_separator_sweep() {
    // A path containing every non-NUL byte value, twice, so `strrchr` has a
    // non-trivial "last occurrence" for each of the 255 possible matches.
    let mut body: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    body.extend((1u16..=255).rev().map(|b| b as u8));
    let path = cstr(&body);
    for s in -128i32..=127 {
        diff_extract_int(&path, s);
        diff_extract(&path, s as u8);
    }
    // and against a path that contains none of the high bytes
    let ascii = cstr(b"plain/ascii/path");
    for s in -128i32..=127 {
        diff_extract_int(&ascii, s);
    }
}

// C12
#[test]
fn c12_extract_high_bit_bytes() {
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    let mut rng = Rng::new(0xC012);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let path = cstr(&rand_from(&mut rng, len, &high));
        let sep = high[rng.below(high.len())];
        diff_extract(&path, sep);
        diff_extract(&path, b'/');
    }
}

// C13
#[test]
fn c13_extract_out_of_char_range_int_separator() {
    let mut body: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    body.extend_from_slice(b"/tail/name");
    let path = cstr(&body);
    let mut cases: Vec<c_int> = vec![
        256,
        257,
        -129,
        -130,
        0x1FF,
        0x100,
        0x12F, // 0x2F == '/'
        0xFFFF_FF2F_u32 as c_int,
        c_int::MIN,
        c_int::MAX,
        -256,
        -257,
        1000,
        -1000,
        65536,
        0x7F00,
    ];
    // every value congruent to '/' mod 256, and a sweep of whole-byte offsets
    for k in 1..=32 {
        cases.push(k * 256 + 0x2F);
        cases.push(-(k * 256) + 0x2F);
        cases.push(k * 256);
        cases.push(-(k * 256));
    }
    for s in cases {
        diff_extract_int(&path, s);
    }
    let empty = cstr(b"");
    for s in [256, -129, c_int::MIN, c_int::MAX, 0x100] {
        diff_extract_int(&empty, s);
    }
}

// C14
#[test]
fn c14_extract_raw_random_bytes() {
    let mut rng = Rng::new(0xC014);
    for _ in 0..ITERS * 2 {
        let len = rng.range(0, 96);
        let body = rand_raw(&mut rng, len);
        let path = cstr(&body);
        // a separator that is present (when possible) and one that is random
        if !body.is_empty() {
            let sep = body[rng.below(body.len())];
            diff_extract(&path, sep);
        }
        diff_extract(&path, rng.byte());
    }
}

// ==================== FIO_createFilename_fromOutDir ========================

fn outdir_ending_slash(rng: &mut Rng) -> Vec<u8> {
    let len = rng.range(0, 24);
    let mut v = rand_from(rng, len, ASCII_NOSLASH);
    v.push(b'/');
    v
}

fn outdir_not_ending_slash(rng: &mut Rng) -> Vec<u8> {
    let len = rng.range(0, 24);
    let mut v = rand_from(rng, len, ASCII_SLASH);
    v.push(ASCII_NOSLASH[rng.below(ASCII_NOSLASH.len())]);
    v
}

// C15
#[test]
fn c15_create_outdir_slash_path_with_slash() {
    let mut rng = Rng::new(0xC015);
    for _ in 0..ITERS {
        let out = cstr(&outdir_ending_slash(&mut rng));
        let path = cstr(&rand_with_slash_range(&mut rng, 1, 48));
        diff_create(&path, &out, 0);
    }
}

// C16
#[test]
fn c16_create_outdir_slash_path_without_slash() {
    let mut rng = Rng::new(0xC016);
    for _ in 0..ITERS {
        let out = cstr(&outdir_ending_slash(&mut rng));
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_NOSLASH));
        diff_create(&path, &out, 0);
    }
}

// C17
#[test]
fn c17_create_outdir_noslash_path_with_slash() {
    let mut rng = Rng::new(0xC017);
    for _ in 0..ITERS {
        let out = cstr(&outdir_not_ending_slash(&mut rng));
        let path = cstr(&rand_with_slash_range(&mut rng, 1, 48));
        diff_create(&path, &out, 0);
    }
}

// C18
#[test]
fn c18_create_outdir_noslash_path_without_slash() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..ITERS {
        let out = cstr(&outdir_not_ending_slash(&mut rng));
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_NOSLASH));
        diff_create(&path, &out, 0);
    }
}

// C19
#[test]
fn c19_create_suffixlen_one() {
    let mut rng = Rng::new(0xC019);
    for _ in 0..ITERS {
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_SLASH));
        diff_create(&path, &out, 1);
    }
}

// C20
#[test]
fn c20_create_suffixlen_small_range() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..ITERS {
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_SLASH));
        let suffix = rng.range(2, 4096);
        let got = diff_create(&path, &out, suffix);
        // calloc guarantees the tail is zero-filled in both implementations
        assert!(got[got.len() - suffix..].iter().all(|&b| b == 0));
    }
}

// C21
#[test]
fn c21_create_suffixlen_large() {
    let mut rng = Rng::new(0xC021);
    for _ in 0..32 {
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_SLASH));
        diff_create(&path, &out, 1 << 20);
    }
}

// C22
#[test]
fn c22_create_outdir_is_single_slash() {
    let out = cstr(b"/");
    let mut rng = Rng::new(0xC022);
    for _ in 0..ITERS {
        let path = cstr(&rand_from_range(&mut rng, 0, 48, ASCII_SLASH));
        diff_create(&path, &out, rng.below(8));
    }
}

// C23
#[test]
fn c23_create_outdir_len1_nonslash() {
    let mut rng = Rng::new(0xC023);
    for _ in 0..ITERS {
        let b = ASCII_NOSLASH[rng.below(ASCII_NOSLASH.len())];
        let out = cstr(&[b]);
        let path = cstr(&rand_from_range(&mut rng, 0, 48, ASCII_SLASH));
        diff_create(&path, &out, rng.below(8));
    }
}

// C24
#[test]
fn c24_create_long_outdir() {
    let mut rng = Rng::new(0xC024);
    for _ in 0..300 {
        let len = rng.range(256, 1024);
        let mut out = rand_from(&mut rng, len, ASCII_SLASH);
        if rng.bool() {
            *out.last_mut().unwrap() = b'/';
        } else {
            *out.last_mut().unwrap() = b'z';
        }
        let out = cstr(&out);
        let path = cstr(&rand_from_range(&mut rng, 1, 512, ASCII_SLASH));
        diff_create(&path, &out, rng.below(2048));
    }
}

// C25
#[test]
fn c25_create_empty_path() {
    let mut rng = Rng::new(0xC025);
    let path = cstr(b"");
    for _ in 0..ITERS {
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        diff_create(&path, &out, rng.below(64));
    }
}

// C26
#[test]
fn c26_create_path_trailing_slash() {
    let mut rng = Rng::new(0xC026);
    for _ in 0..ITERS {
        let mut v = rand_from_range(&mut rng, 0, 32, ASCII_NOSLASH);
        v.push(b'/');
        let path = cstr(&v);
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        diff_create(&path, &out, rng.below(64));
    }
}

// C27
#[test]
fn c27_create_path_single_slash() {
    let mut rng = Rng::new(0xC027);
    let path = cstr(b"/");
    for _ in 0..ITERS {
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        diff_create(&path, &out, rng.below(64));
    }
    diff_create(&path, &cstr(b"/"), 0);
    diff_create(&path, &cstr(b"//"), 0);
}

// C28
#[test]
fn c28_create_path_many_slashes() {
    let mut rng = Rng::new(0xC028);
    for _ in 0..ITERS {
        let len = rng.range(1, 48);
        let v: Vec<u8> = (0..len)
            .map(|_| if rng.below(3) == 0 { b'q' } else { b'/' })
            .collect();
        let path = cstr(&v);
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        diff_create(&path, &out, rng.below(32));
    }
}

// C29
#[test]
fn c29_create_outdir_high_bit_bytes() {
    let mut rng = Rng::new(0xC029);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    for _ in 0..ITERS {
        let mut out = rand_from_range(&mut rng, 1, 32, &high);
        if rng.bool() {
            *out.last_mut().unwrap() = b'/';
        }
        let out = cstr(&out);
        let path = cstr(&rand_from_range(&mut rng, 1, 48, ASCII_SLASH));
        diff_create(&path, &out, rng.below(32));
    }
}

// C30
#[test]
fn c30_create_path_high_bit_bytes() {
    let mut rng = Rng::new(0xC030);
    let high: Vec<u8> = (0x80u16..=0xFF).map(|b| b as u8).collect();
    for _ in 0..ITERS {
        let mut v = rand_from_range(&mut rng, 1, 48, &high);
        if rng.bool() {
            let at = rng.below(v.len());
            v[at] = b'/';
        }
        let path = cstr(&v);
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        diff_create(&path, &out, rng.below(32));
    }
    // 0xFF == -1 as signed char, the value most likely to trip sign-extension
    let path = cstr(&[0xFF, b'/', 0xFF, 0xFE]);
    diff_create(&path, &cstr(&[0xFF]), 4);
    diff_create(&path, &cstr(&[0xFF, b'/']), 4);
}

// C31 (also ERRORS.md E8): empty outDirName => outDirName[-1] out-of-bounds read
#[test]
fn c31_create_empty_outdir_sentinel_before_buffer() {
    use std::os::raw::c_char;
    let mut rng = Rng::new(0xC031);
    for sentinel in [b'/', b'x', 0u8, 0xFFu8, b'\\'] {
        for _ in 0..200 {
            // buf = [sentinel, 0]; outDirName points at buf[1] (an empty string),
            // so outDirName[-1] deterministically reads `sentinel`.
            let buf = [sentinel, 0u8];
            let out_ptr = unsafe { (buf.as_ptr() as *const c_char).add(1) };
            let path = cstr(&rand_from_range(&mut rng, 0, 32, ASCII_SLASH));
            diff_create_ptr(
                path.as_ptr() as *const c_char,
                out_ptr,
                rng.below(16),
                "<random>",
                &format!("<empty, preceded by 0x{sentinel:02x}>"),
            );
        }
    }
}

// C32
#[test]
fn c32_create_full_random_fuzz() {
    let mut rng = Rng::new(0xC032);
    for _ in 0..20_000 {
        let path = cstr(&rand_raw_range(&mut rng, 0, 40));
        // outDirName must be non-empty here (empty is covered by C31 with a
        // controlled sentinel; with an uncontrolled one the C itself reads
        // indeterminate memory and no two calls need agree).
        let out = cstr(&rand_raw_range(&mut rng, 1, 40));
        diff_create(&path, &out, rng.below(64));
    }
}

// C33
#[test]
fn c33_composed_pipeline_cross_library() {
    use std::ffi::c_void;
    use std::os::raw::c_char;
    let p = pair();
    let mut rng = Rng::new(0xC033);
    for _ in 0..ITERS {
        let path = cstr(&rand_from_range(&mut rng, 1, 64, ASCII_SLASH));
        let out = if rng.bool() {
            cstr(&outdir_ending_slash(&mut rng))
        } else {
            cstr(&outdir_not_ending_slash(&mut rng))
        };
        let suffix = rng.below(16);

        // Stage 1: extractFilename output (must agree) fed back in as a path.
        let off = diff_extract(&path, b'/');
        let stage1 = unsafe { (path.as_ptr() as *const c_char).add(off as usize) };
        diff_create_ptr(stage1, out.as_ptr() as *const c_char, suffix, "<stage1>", "<outdir>");

        // Stage 2: the result of C's create fed into BOTH extractFilename impls,
        // and the result of Rust's create fed into BOTH as well.
        unsafe {
            for maker in [p.c.create_filename, p.rs.create_filename] {
                let r = maker(path.as_ptr() as *const c_char, out.as_ptr() as *const c_char, suffix);
                assert!(!r.is_null());
                let oc = (p.c.extract_filename)(r, b'/' as i8 as c_char) as isize - r as isize;
                let or = (p.rs.extract_filename)(r, b'/' as i8 as c_char) as isize - r as isize;
                assert_eq!(oc, or, "round-trip extractFilename divergence");
                // and feed it back into create, on both sides
                diff_create_ptr(r, out.as_ptr() as *const c_char, suffix, "<created>", "<outdir>");
                free(r as *mut c_void);
            }
        }
    }
}
