//! Catch-all randomised differential fuzzing across the whole public API.
//!
//! Where `phase_b.rs` / `phase_c.rs` walk the `CONFIGS.md` / `ERRORS.md` rows
//! deliberately, this file throws thousands of *unstructured* inputs (random
//! byte soup for the matrix text, random dimensions including negative ones,
//! random file names and contents) at both libraries and compares every
//! observable: return values, matrix contents, produced strings, written file
//! bytes and `stderr`.
//!
//! Seeds are fixed, so any failure is reproducible.

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::fs;

/// The C `matrix_to_string` sizes its output buffer as
/// `height * (width * 10 + width) + height + 1`, which is only large enough
/// when the decimal renderings average ≤ 10 characters per cell.  Longer values
/// make the C code overrun its own heap buffer (undefined behaviour in the
/// original), so those shapes are excluded from the comparison instead of
/// "fixing" the C.
fn to_string_fits(width: c_int, height: c_int, cells: &[c_int]) -> bool {
    if width <= 0 || height <= 0 {
        return true; // no cell is ever rendered
    }
    let total_len: i64 = cells.iter().map(|v| v.to_string().len() as i64).sum();
    let w = width as i64;
    let h = height as i64;
    // bytes needed: digits + separators (w-1 per row) + '\n' per row + NUL
    let needed = total_len + (w - 1) * h + h + 1;
    let budget = h * (w * 10 + w) + h + 1;
    needed <= budget
}

/// Unstructured byte soup.
fn random_soup(rng: &mut Rng) -> Vec<u8> {
    let alphabet: &[u8] = b"0123456789 \n\t-+ .abcxyz\r";
    let len = rng.range(0, 60) as usize;
    (0..len).map(|_| *rng.pick(alphabet)).collect()
}

/// Well-formed text for a `width` x `height` matrix, with random amounts of
/// extra rows/columns, blank lines, whitespace runs and quirky tokens — plus a
/// chance of being replaced by pure byte soup.
fn gen_text(rng: &mut Rng, width: c_int, height: c_int) -> Vec<u8> {
    if rng.range(0, 9) == 0 {
        return random_soup(rng);
    }
    let rows = height.max(0) + rng.i32_in(0, 2);
    let cols = width.max(0) + rng.i32_in(0, 2);
    let quirky = ["abc", "12x", "-", "+7", "0x10", "3.5", "99999999999", "007"];
    let mut out = String::new();
    if rng.range(0, 4) == 0 {
        out.push('\n');
    }
    for _ in 0..rows.max(1) {
        for j in 0..cols.max(1) {
            if j > 0 {
                out.push_str(&" ".repeat(rng.range(1, 3) as usize));
            }
            if rng.range(0, 6) == 0 {
                out.push_str(*rng.pick(&quirky[..]));
            } else {
                out.push_str(&rng.i32_in(-99_999, 99_999).to_string());
            }
        }
        if rng.range(0, 6) == 0 {
            out.push(' ');
        }
        out.push('\n');
        if rng.range(0, 8) == 0 {
            out.push('\n');
        }
    }
    out.into_bytes()
}

#[test]
fn fuzz_matrix_pipeline() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF0000001);
    let mut stats = (0usize, 0usize, 0usize); // (init ok, multiplied, stringified)

    for iter in 0..4000 {
        let wa = rng.i32_in(-2, 6);
        let ha = rng.i32_in(-2, 6);
        let wb = rng.i32_in(-2, 6);
        let hb = rng.i32_in(-2, 6);
        // let the inner dimensions agree reasonably often
        let hb = if rng.range(0, 2) == 0 { wa } else { hb };
        let a_bytes = gen_text(&mut rng, wa, ha);
        let b_bytes = gen_text(&mut rng, wb, hb);
        let ctx = format!(
            "fuzz #{iter} a={wa}x{ha} b={wb}x{hb} a_text={:?} b_text={:?}",
            String::from_utf8_lossy(&a_bytes),
            String::from_utf8_lossy(&b_bytes)
        );
        let a_text = CBuf::new(a_bytes);
        let b_text = CBuf::new(b_bytes);

        // ---- initialize_matrix_from_string on both -------------------------
        let (ca, c_err_a) =
            capture_stderr(|| unsafe { (c.initialize_matrix_from_string)(a_text.as_ptr(), wa, ha) });
        let (ra, r_err_a) =
            capture_stderr(|| unsafe { (r.initialize_matrix_from_string)(a_text.as_ptr(), wa, ha) });
        assert_eq!(
            unsafe { snap_matrix(ca, true) },
            unsafe { snap_matrix(ra, true) },
            "{ctx}: matrix A mismatch"
        );
        assert_bytes_eq(&c_err_a, &r_err_a, &format!("{ctx}: A stderr"));

        let (cb, c_err_b) =
            capture_stderr(|| unsafe { (c.initialize_matrix_from_string)(b_text.as_ptr(), wb, hb) });
        let (rb, r_err_b) =
            capture_stderr(|| unsafe { (r.initialize_matrix_from_string)(b_text.as_ptr(), wb, hb) });
        assert_eq!(
            unsafe { snap_matrix(cb, true) },
            unsafe { snap_matrix(rb, true) },
            "{ctx}: matrix B mismatch"
        );
        assert_bytes_eq(&c_err_b, &r_err_b, &format!("{ctx}: B stderr"));

        if !ca.is_null() && !cb.is_null() {
            stats.0 += 1;

            // ---- matrix_to_string of the operands -------------------------
            for (cm, rm, tag) in [(ca, ra, "A"), (cb, rb, "B")] {
                let snap = unsafe { snap_matrix(cm, true) };
                if to_string_fits(snap.width, snap.height, &snap.cells) {
                    let (cs, ce) = capture_stderr(|| unsafe { (c.matrix_to_string)(cm) });
                    let (rs, re) = capture_stderr(|| unsafe { (r.matrix_to_string)(rm) });
                    let cbytes = unsafe { take_c_string(cs) };
                    let rbytes = unsafe { take_c_string(rs) };
                    assert_opt_bytes_eq(&cbytes, &rbytes, &format!("{ctx}: to_string({tag})"));
                    assert_bytes_eq(&ce, &re, &format!("{ctx}: to_string({tag}) stderr"));
                    stats.2 += 1;
                }
            }

            // ---- multiply_matrices ----------------------------------------
            let (cres, ce) = capture_stderr(|| unsafe { (c.multiply_matrices)(ca, cb) });
            let (rres, re) = capture_stderr(|| unsafe { (r.multiply_matrices)(ra, rb) });
            assert_eq!(
                unsafe { snap_matrix(cres, true) },
                unsafe { snap_matrix(rres, true) },
                "{ctx}: product mismatch"
            );
            assert_bytes_eq(&ce, &re, &format!("{ctx}: multiply stderr"));

            if !cres.is_null() {
                stats.1 += 1;
                let snap = unsafe { snap_matrix(cres, true) };
                if to_string_fits(snap.width, snap.height, &snap.cells) {
                    let (cs, ce) = capture_stderr(|| unsafe { (c.matrix_to_string)(cres) });
                    let (rs, re) = capture_stderr(|| unsafe { (r.matrix_to_string)(rres) });
                    let cbytes = unsafe { take_c_string(cs) };
                    let rbytes = unsafe { take_c_string(rs) };
                    assert_opt_bytes_eq(&cbytes, &rbytes, &format!("{ctx}: to_string(product)"));
                    assert_bytes_eq(&ce, &re, &format!("{ctx}: to_string(product) stderr"));

                    // ---- write_to_file of the product -----------------
                    if let (Some(cbytes), Some(rbytes)) = (cbytes, rbytes) {
                        let dir = unique_dir("fuzz-write");
                        let cf = dir.join("c.txt");
                        let rf = dir.join("r.txt");
                        let cpath = path_cbuf(&cf);
                        let rpath = path_cbuf(&rf);
                        let cc = CBuf::new(cbytes);
                        let rc_buf = CBuf::new(rbytes);
                        let crc =
                            unsafe { (c.write_to_file)(cpath.as_ptr(), cc.as_ptr()) };
                        let rrc =
                            unsafe { (r.write_to_file)(rpath.as_ptr(), rc_buf.as_ptr()) };
                        assert_eq!(crc, rrc, "{ctx}: write rc");
                        assert_eq!(fs::read(&cf).ok(), fs::read(&rf).ok(), "{ctx}: written bytes");
                        let _ = fs::remove_dir_all(&dir);
                    }
                }
            }
            unsafe {
                (c.free_matrix)(cres);
                (r.free_matrix)(rres);
            }
        }

        unsafe {
            (c.free_matrix)(ca);
            (r.free_matrix)(ra);
            (c.free_matrix)(cb);
            (r.free_matrix)(rb);
        }
    }
    eprintln!(
        "fuzz_matrix_pipeline coverage: {} operand pairs parsed, {} products, {} strings",
        stats.0, stats.1, stats.2
    );
    assert!(
        stats.0 > 200 && stats.1 > 50 && stats.2 > 100,
        "fuzzing did not reach enough successful paths: {stats:?}"
    );
}

#[test]
fn fuzz_write_to_file() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF0000002);
    let dir = unique_dir("fuzz-w");
    fs::create_dir_all(dir.join("sub")).unwrap();
    let existing = dir.join("exists.txt");
    fs::write(&existing, "pre-existing").unwrap();

    for iter in 0..600 {
        // random content (never containing NUL, which cannot cross the C API)
        let len = rng.range(0, 300) as usize;
        let content: Vec<u8> = (0..len)
            .map(|_| {
                let b = (rng.next_u64() % 255) as u8 + 1;
                b
            })
            .collect();
        let content = CBuf::new(content);

        // random target: fresh file / existing file / sub-directory / bad paths
        let kind = rng.range(0, 5);
        let (cname, rname) = match kind {
            0 => (dir.join("c-fresh.txt"), dir.join("r-fresh.txt")),
            1 => (existing.clone(), existing.clone()),
            2 => (dir.join("sub/c.txt"), dir.join("sub/r.txt")),
            3 => (dir.join("missing-dir/c.txt"), dir.join("missing-dir/r.txt")),
            _ => (dir.clone(), dir.clone()),
        };
        let ctx = format!("fuzz-write #{iter} kind={kind}");
        let cpath = path_cbuf(&cname);
        let rpath = path_cbuf(&rname);
        let (crc, ce) = capture_stderr(|| unsafe { (c.write_to_file)(cpath.as_ptr(), content.as_ptr()) });
        let (rrc, re) = capture_stderr(|| unsafe { (r.write_to_file)(rpath.as_ptr(), content.as_ptr()) });
        assert_eq!(crc, rrc, "{ctx}: return code");
        if cname == rname {
            // identical file name ⇒ identical diagnostics
            assert_bytes_eq(&ce, &re, &format!("{ctx}: stderr"));
        } else {
            assert_eq!(ce.is_empty(), re.is_empty(), "{ctx}: stderr presence");
            assert_eq!(fs::read(&cname).ok(), fs::read(&rname).ok(), "{ctx}: bytes");
        }
    }
}

#[test]
fn fuzz_driver() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF0000003);
    let mut ok = 0usize;
    for iter in 0..600 {
        let wa = rng.i32_in(-1, 5);
        let ha = rng.i32_in(-1, 5);
        let hb = rng.i32_in(-1, 5);
        // width_b == 1 keeps the C output buffer safe for arbitrary int values
        // (see `to_string_fits`); wider results are covered by driver.rs.
        let wb = 1;
        let hb = if rng.range(0, 2) == 0 { wa } else { hb };
        let a = CBuf::new(gen_text(&mut rng, wa, ha));
        let b = CBuf::new(gen_text(&mut rng, wb, hb));
        let ctx = format!("fuzz-driver #{iter} a={wa}x{ha} b={wb}x{hb}");

        let (crc, ce, c_file, rrc, re, r_file) = in_temp_cwd(|dir| {
            let out = dir.join("matrix.txt");
            let (crc, ce) = capture_stderr(|| unsafe {
                (c.driver)(wa, ha, a.as_ptr(), wb, hb, b.as_ptr())
            });
            let c_file = fs::read(&out).ok();
            let _ = fs::remove_file(&out);
            let (rrc, re) = capture_stderr(|| unsafe {
                (r.driver)(wa, ha, a.as_ptr(), wb, hb, b.as_ptr())
            });
            let r_file = fs::read(&out).ok();
            let _ = fs::remove_file(&out);
            (crc, ce, c_file, rrc, re, r_file)
        });
        assert_eq!(crc, rrc, "{ctx}: return code");
        assert_bytes_eq(&ce, &re, &format!("{ctx}: stderr"));
        assert_opt_bytes_eq(&c_file, &r_file, &format!("{ctx}: matrix.txt"));
        if crc == EXIT_SUCCESS {
            ok += 1;
        }
    }
    eprintln!("fuzz_driver coverage: {ok} successful end-to-end runs");
    assert!(ok > 20, "fuzz_driver never succeeded ({ok} successes)");
}

#[test]
fn fuzz_allocate_free() {
    let (c, r) = both();
    let mut rng = Rng::new(0xF0000004);
    for iter in 0..800 {
        let w = rng.i32_in(-3, 40);
        let h = rng.i32_in(-3, 40);
        let ctx = format!("fuzz-alloc #{iter} {w}x{h}");
        let (cp, ce) = capture_stderr(|| unsafe { (c.allocate_matrix)(w, h) });
        let (rp, re) = capture_stderr(|| unsafe { (r.allocate_matrix)(w, h) });
        assert_eq!(
            unsafe { snap_matrix(cp, false) },
            unsafe { snap_matrix(rp, false) },
            "{ctx}"
        );
        assert_bytes_eq(&ce, &re, &format!("{ctx}: stderr"));
        // write through every row of both matrices, then compare
        if !cp.is_null() && w > 0 && h > 0 {
            for m in [cp, rp] {
                for i in 0..h {
                    let row = unsafe { *(*m).matrix.offset(i as isize) };
                    for j in 0..w {
                        unsafe { *row.offset(j as isize) = i.wrapping_mul(1000).wrapping_add(j) };
                    }
                }
            }
            assert_eq!(
                unsafe { snap_matrix(cp, true) },
                unsafe { snap_matrix(rp, true) },
                "{ctx}: read-back"
            );
        }
        unsafe {
            (c.free_matrix)(cp);
            (r.free_matrix)(rp);
        }
    }
    // free_matrix must also tolerate a NULL pointer any number of times
    for _ in 0..100 {
        unsafe {
            (c.free_matrix)(std::ptr::null_mut());
            (r.free_matrix)(std::ptr::null_mut());
        }
    }
    let _ = c_void_unused();
}

fn c_void_unused() -> *mut c_void {
    std::ptr::null_mut()
}
