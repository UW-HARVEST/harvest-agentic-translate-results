//! Phase B — valid-path differential tests.
//! One test per row of `CONFIGS.md` (C1..C27), each driven with many
//! randomized inputs from a fixed seed.

mod harness;
use harness::*;

const N: usize = 400; // randomized cases per row

// ---------------------------------------------------------------------------
// Random input generators
// ---------------------------------------------------------------------------

/// Random ASCII string of length `len` that never contains `sep` and never NUL.
fn rand_ascii_without(rng: &mut Rng, len: usize, sep: u8) -> Vec<u8> {
    (0..len).map(|_| rng.ascii_except(sep)).collect()
}

/// Random byte string (full 0x01..0xFF range) never containing `sep`.
fn rand_bytes_without(rng: &mut Rng, len: usize, sep: u8) -> Vec<u8> {
    (0..len).map(|_| rng.byte_except(sep)).collect()
}

/// Random path with exactly `k` occurrences of `sep` at random positions
/// (never as the last byte unless `allow_trailing`).
fn rand_path_with_seps(rng: &mut Rng, len: usize, sep: u8, k: usize) -> Vec<u8> {
    let mut v = rand_ascii_without(rng, len, sep);
    if v.is_empty() {
        return v;
    }
    for _ in 0..k {
        let i = rng.below(v.len());
        v[i] = sep;
    }
    v
}

// ===========================================================================
// C1..C9 — extractFilename (lowest level entry point)
// ===========================================================================

#[test]
fn c1_separator_present_interior() {
    let mut rng = Rng::seeded(SEED ^ 1);
    for i in 0..N {
        let len = rng.range(2, 64);
        let mut v = rand_ascii_without(&mut rng, len, SEP);
        let idx = rng.range(0, len - 2); // never the last byte
        v[idx] = SEP;
        diff_extract(&v, SEP, &format!("C1[{i}]"));
    }
}

#[test]
fn c2_separator_absent() {
    let mut rng = Rng::seeded(SEED ^ 2);
    for i in 0..N {
        let len = rng.range(0, 64);
        let v = rand_ascii_without(&mut rng, len, SEP);
        diff_extract(&v, SEP, &format!("C2[{i}]"));
    }
}

#[test]
fn c3_separator_is_last_byte() {
    let mut rng = Rng::seeded(SEED ^ 3);
    for i in 0..N {
        let len = rng.range(1, 64);
        let mut v = rand_ascii_without(&mut rng, len, SEP);
        *v.last_mut().unwrap() = SEP;
        diff_extract(&v, SEP, &format!("C3[{i}]"));
    }
}

#[test]
fn c4_separator_is_first_byte_only() {
    let mut rng = Rng::seeded(SEED ^ 4);
    for i in 0..N {
        let len = rng.range(1, 64);
        let mut v = rand_ascii_without(&mut rng, len, SEP);
        v[0] = SEP;
        diff_extract(&v, SEP, &format!("C4[{i}]"));
    }
}

#[test]
fn c5_many_separators_picks_last() {
    let mut rng = Rng::seeded(SEED ^ 5);
    for i in 0..N {
        let len = rng.range(4, 64);
        let k = rng.range(2, 12);
        let v = rand_path_with_seps(&mut rng, len, SEP, k);
        diff_extract(&v, SEP, &format!("C5[{i}]"));
    }
}

#[test]
fn c6_nul_separator() {
    let mut rng = Rng::seeded(SEED ^ 6);
    for i in 0..N {
        let len = rng.range(0, 48);
        let v = rand_bytes_without(&mut rng, len, 0);
        diff_extract(&v, 0, &format!("C6[{i}]"));
    }
}

#[test]
fn c7_high_bit_separator() {
    let mut rng = Rng::seeded(SEED ^ 7);
    for i in 0..N {
        let sep = (0x80 + rng.below(0x80)) as u8;
        let len = rng.range(1, 48);
        let mut v = rand_bytes_without(&mut rng, len, sep);
        // sprinkle 0..3 occurrences of the high-bit separator
        for _ in 0..rng.below(4) {
            let j = rng.below(v.len());
            v[j] = sep;
        }
        diff_extract(&v, sep, &format!("C7[{i}] sep={sep:#04x}"));
    }
}

#[test]
fn c8_backslash_separator_windows_arm() {
    let mut rng = Rng::seeded(SEED ^ 8);
    for i in 0..N {
        let len = rng.range(1, 48);
        let mut v = rand_ascii_without(&mut rng, len, b'\\');
        // mix in both '/' and '\\'
        for _ in 0..rng.below(5) {
            let j = rng.below(v.len());
            v[j] = if rng.below(2) == 0 { b'/' } else { b'\\' };
        }
        diff_extract(&v, b'\\', &format!("C8[{i}]"));
        diff_extract(&v, b'/', &format!("C8'[{i}]"));
    }
}

#[test]
fn c9_full_byte_range_property_sweep() {
    let mut rng = Rng::seeded(SEED ^ 9);
    for i in 0..(N * 4) {
        let len = rng.range(0, 32);
        let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let sep = rng.below(256) as u8;
        diff_extract(&v, sep, &format!("C9[{i}] sep={sep:#04x}"));
    }
}

// ===========================================================================
// C10..C27 — FIO_createFilename_fromOutDir
// ===========================================================================

/// Random outDirName that does NOT end in `/` (false arm of lib.c:45).
fn rand_dir_no_trailing_sep(rng: &mut Rng) -> Vec<u8> {
    let depth = rng.range(1, 4);
    let mut v = Vec::new();
    for d in 0..depth {
        if d > 0 {
            v.push(SEP);
        }
        let seg = rng.range(1, 8);
        v.extend(rand_ascii_without(rng, seg, SEP));
    }
    v
}

/// Random outDirName that DOES end in `/` (true arm of lib.c:45).
fn rand_dir_trailing_sep(rng: &mut Rng) -> Vec<u8> {
    let mut v = rand_dir_no_trailing_sep(rng);
    v.push(SEP);
    v
}

/// Random path containing at least one separator (not trailing).
fn rand_path_with_sep(rng: &mut Rng) -> Vec<u8> {
    let depth = rng.range(2, 5);
    let mut v = Vec::new();
    for d in 0..depth {
        if d > 0 {
            v.push(SEP);
        }
        let seg = rng.range(1, 10);
        v.extend(rand_ascii_without(rng, seg, SEP));
    }
    v
}

/// Random path with no separator at all.
fn rand_path_no_sep(rng: &mut Rng) -> Vec<u8> {
    let len = rng.range(0, 20);
    rand_ascii_without(rng, len, SEP)
}

#[test]
fn c10_dir_no_trailing_sep_path_with_sep_suffix0() {
    let mut rng = Rng::seeded(SEED ^ 10);
    for i in 0..N {
        let d = rand_dir_no_trailing_sep(&mut rng);
        let p = rand_path_with_sep(&mut rng);
        diff_create(&p, &d, 0, &format!("C10[{i}]"));
    }
}

#[test]
fn c11_dir_trailing_sep_path_with_sep_suffix0() {
    let mut rng = Rng::seeded(SEED ^ 11);
    for i in 0..N {
        let d = rand_dir_trailing_sep(&mut rng);
        let p = rand_path_with_sep(&mut rng);
        diff_create(&p, &d, 0, &format!("C11[{i}]"));
    }
}

#[test]
fn c12_dir_no_trailing_sep_path_without_sep() {
    let mut rng = Rng::seeded(SEED ^ 12);
    for i in 0..N {
        let d = rand_dir_no_trailing_sep(&mut rng);
        let p = rand_path_no_sep(&mut rng);
        diff_create(&p, &d, 0, &format!("C12[{i}]"));
    }
}

#[test]
fn c13_dir_trailing_sep_path_without_sep() {
    let mut rng = Rng::seeded(SEED ^ 13);
    for i in 0..N {
        let d = rand_dir_trailing_sep(&mut rng);
        let p = rand_path_no_sep(&mut rng);
        diff_create(&p, &d, 0, &format!("C13[{i}]"));
    }
}

#[test]
fn c14_dir_is_single_separator() {
    let mut rng = Rng::seeded(SEED ^ 14);
    for i in 0..N {
        let p = if rng.below(2) == 0 {
            rand_path_with_sep(&mut rng)
        } else {
            rand_path_no_sep(&mut rng)
        };
        let n = rng.range(0, 8);
        diff_create(&p, b"/", n, &format!("C14[{i}]"));
    }
}

#[test]
fn c15_dir_no_trailing_sep_small_suffixlen() {
    let mut rng = Rng::seeded(SEED ^ 15);
    for i in 0..N {
        let d = rand_dir_no_trailing_sep(&mut rng);
        let p = rand_path_with_sep(&mut rng);
        let n = rng.range(1, 16);
        diff_create(&p, &d, n, &format!("C15[{i}] suffix={n}"));
    }
}

#[test]
fn c16_dir_trailing_sep_small_suffixlen() {
    let mut rng = Rng::seeded(SEED ^ 16);
    for i in 0..N {
        let d = rand_dir_trailing_sep(&mut rng);
        let p = rand_path_with_sep(&mut rng);
        let n = rng.range(1, 16);
        diff_create(&p, &d, n, &format!("C16[{i}] suffix={n}"));
    }
}

#[test]
fn c17_large_but_allocatable_suffixlen() {
    let mut rng = Rng::seeded(SEED ^ 17);
    for i in 0..24 {
        let d = if i % 2 == 0 {
            rand_dir_no_trailing_sep(&mut rng)
        } else {
            rand_dir_trailing_sep(&mut rng)
        };
        let p = rand_path_with_sep(&mut rng);
        let n = rng.range(1 << 20, 8 << 20);
        diff_create(&p, &d, n, &format!("C17[{i}] suffix={n}"));
    }
}

#[test]
fn c18_path_trailing_separator() {
    let mut rng = Rng::seeded(SEED ^ 18);
    for i in 0..N {
        let mut p = rand_path_with_sep(&mut rng);
        p.push(SEP);
        let d0 = rand_dir_no_trailing_sep(&mut rng);
        let d1 = rand_dir_trailing_sep(&mut rng);
        let n = rng.range(0, 8);
        diff_create(&p, &d0, n, &format!("C18a[{i}]"));
        diff_create(&p, &d1, n, &format!("C18b[{i}]"));
    }
}

#[test]
fn c19_path_all_separators() {
    let mut rng = Rng::seeded(SEED ^ 19);
    for i in 0..N {
        let k = rng.range(1, 6);
        let p = vec![SEP; k];
        let d0 = rand_dir_no_trailing_sep(&mut rng);
        let d1 = rand_dir_trailing_sep(&mut rng);
        let n = rng.range(0, 8);
        diff_create(&p, &d0, n, &format!("C19a[{i}] k={k}"));
        diff_create(&p, &d1, n, &format!("C19b[{i}] k={k}"));
    }
}

#[test]
fn c20_empty_path() {
    let mut rng = Rng::seeded(SEED ^ 20);
    for i in 0..N {
        let d0 = rand_dir_no_trailing_sep(&mut rng);
        let d1 = rand_dir_trailing_sep(&mut rng);
        for &n in &[0usize, 5] {
            diff_create(b"", &d0, n, &format!("C20a[{i}] n={n}"));
            diff_create(b"", &d1, n, &format!("C20b[{i}] n={n}"));
        }
    }
}

#[test]
fn c21_empty_outdir_controlled_preceding_byte() {
    let mut rng = Rng::seeded(SEED ^ 21);
    for i in 0..N {
        let p = if rng.below(2) == 0 {
            rand_path_with_sep(&mut rng)
        } else {
            rand_path_no_sep(&mut rng)
        };
        let n = rng.range(0, 8);
        // preceding byte == '/'  -> true arm of lib.c:45
        diff_create_raw_outdir(&p, &[SEP, 0], 1, n, &format!("C21-sep[{i}]"));
        // preceding byte == 'X'  -> false arm
        diff_create_raw_outdir(&p, &[b'X', 0], 1, n, &format!("C21-x[{i}]"));
        // preceding byte == '\\' -> false arm on POSIX
        diff_create_raw_outdir(&p, &[b'\\', 0], 1, n, &format!("C21-bs[{i}]"));
    }
}

#[test]
fn c22_deep_multi_component() {
    let mut rng = Rng::seeded(SEED ^ 22);
    for i in 0..N {
        let depth = rng.range(1, 8);
        let mut d = Vec::new();
        for k in 0..depth {
            if k > 0 {
                d.push(SEP);
            }
            let seg = rng.range(1, 6);
            d.extend(rand_ascii_without(&mut rng, seg, SEP));
        }
        let mut p = Vec::new();
        let pdepth = rng.range(1, 8);
        for k in 0..pdepth {
            if k > 0 {
                p.push(SEP);
            }
            let seg = rng.range(1, 6);
            p.extend(rand_ascii_without(&mut rng, seg, SEP));
        }
        let n = rng.range(0, 12);
        diff_create(&p, &d, n, &format!("C22a[{i}]"));
        let mut d2 = d.clone();
        d2.push(SEP);
        diff_create(&p, &d2, n, &format!("C22b[{i}]"));
    }
}

#[test]
fn c23_non_ascii_and_invalid_utf8_bytes() {
    let mut rng = Rng::seeded(SEED ^ 23);
    for i in 0..N {
        let dlen = rng.range(1, 20);
        let mut d = rand_bytes_without(&mut rng, dlen, 0);
        // make sure the last byte is not the separator (false arm)
        if *d.last().unwrap() == SEP {
            *d.last_mut().unwrap() = 0x80;
        }
        let plen = rng.range(0, 24);
        let mut p = rand_bytes_without(&mut rng, plen, 0);
        for _ in 0..rng.below(3) {
            if !p.is_empty() {
                let j = rng.below(p.len());
                p[j] = SEP;
            }
        }
        let n = rng.range(0, 8);
        diff_create(&p, &d, n, &format!("C23a[{i}]"));
        let mut d2 = d.clone();
        d2.push(SEP);
        diff_create(&p, &d2, n, &format!("C23b[{i}]"));
    }
}

#[test]
fn c24_outdir_ends_with_backslash() {
    let mut rng = Rng::seeded(SEED ^ 24);
    for i in 0..N {
        let mut d = rand_dir_no_trailing_sep(&mut rng);
        d.push(b'\\');
        let p = rand_path_with_sep(&mut rng);
        let n = rng.range(0, 8);
        let out = diff_create(&p, &d, n, &format!("C24[{i}]"));
        // On POSIX the separator is '/', so the backslash must NOT be treated
        // as a separator: the '/' must have been inserted.
        assert_eq!(
            out[d.len()], SEP,
            "C24[{i}]: expected inserted '/' after outDirName ending in backslash"
        );
    }
}

#[test]
fn c25_composed_pipeline_consistency() {
    let p = pair();
    let mut rng = Rng::seeded(SEED ^ 25);
    for i in 0..N {
        let path = if rng.below(2) == 0 {
            rand_path_with_sep(&mut rng)
        } else {
            rand_path_no_sep(&mut rng)
        };
        let dir = if rng.below(2) == 0 {
            rand_dir_no_trailing_sep(&mut rng)
        } else {
            rand_dir_trailing_sep(&mut rng)
        };
        let n = rng.range(0, 8);

        // low-level result from BOTH .so's
        let bp = CBuf::new(&path);
        let oc = unsafe { (p.c.extract)(bp.ptr(), SEP as i8) } as usize - bp.ptr() as usize;
        let or = unsafe { (p.rs.extract)(bp.ptr(), SEP as i8) } as usize - bp.ptr() as usize;
        assert_eq!(oc, or, "C25[{i}]: low-level offsets differ");

        let out = diff_create(&path, &dir, n, &format!("C25[{i}]"));
        let fname = &path[oc..];
        let ins = if dir.last() == Some(&SEP) { 0 } else { 1 };
        assert_eq!(
            &out[dir.len() + ins..dir.len() + ins + fname.len()],
            fname,
            "C25[{i}]: composed result does not embed the low-level filename"
        );
        // the calloc zero padding
        assert!(
            out[dir.len() + ins + fname.len()..].iter().all(|&b| b == 0),
            "C25[{i}]: trailing bytes are not zero"
        );
    }
}

#[test]
fn c26_suffixlen_boundaries() {
    let mut rng = Rng::seeded(SEED ^ 26);
    for i in 0..N {
        let d0 = rand_dir_no_trailing_sep(&mut rng);
        let d1 = rand_dir_trailing_sep(&mut rng);
        let p = rand_path_with_sep(&mut rng);
        let flen = ref_filename_len(&p, SEP);
        for dir in [&d0, &d1] {
            for &target in &[1usize << 12, 1 << 16] {
                // choose suffixLen so that the alloc size is exactly `target`
                let base = dir.len() + 1 + flen + 1;
                if target > base {
                    let n = target - base;
                    let out = diff_create(&p, dir, n, &format!("C26[{i}] size={target}"));
                    assert_eq!(out.len(), target);
                }
            }
            diff_create(&p, dir, 0, &format!("C26-0[{i}]"));
            diff_create(&p, dir, 1, &format!("C26-1[{i}]"));
        }
    }
}

#[test]
fn c27_repeated_invocation_no_shared_state() {
    let p = pair();
    let path = b"/some/deep/dir/file.tar";
    let dir = b"out";
    let n = 4usize;
    let bp = CBuf::new(path);
    let bd = CBuf::new(dir);
    let size = ref_alloc_size(dir.len(), ref_filename_len(path, SEP), n);
    let mut live_c = Vec::new();
    let mut live_r = Vec::new();
    let mut first: Option<Vec<u8>> = None;
    for i in 0..200 {
        let oc = unsafe { (p.c.create)(bp.ptr(), bd.ptr(), n) };
        let or = unsafe { (p.rs.create)(bp.ptr(), bd.ptr(), n) };
        assert!(!oc.is_null() && !or.is_null());
        let sc = unsafe { std::slice::from_raw_parts(oc as *const u8, size) }.to_vec();
        let sr = unsafe { std::slice::from_raw_parts(or as *const u8, size) }.to_vec();
        assert_eq!(sc, sr, "C27[{i}]: content mismatch");
        match &first {
            None => first = Some(sc.clone()),
            Some(f) => assert_eq!(f, &sc, "C27[{i}]: result not stable across calls"),
        }
        // distinct allocations each time
        assert!(!live_c.contains(&(oc as usize)), "C27[{i}]: C reused a live pointer");
        assert!(!live_r.contains(&(or as usize)), "C27[{i}]: Rust reused a live pointer");
        live_c.push(oc as usize);
        live_r.push(or as usize);
    }
    for q in live_c {
        unsafe { free(q as *mut _) };
    }
    for q in live_r {
        unsafe { free(q as *mut _) };
    }
}
