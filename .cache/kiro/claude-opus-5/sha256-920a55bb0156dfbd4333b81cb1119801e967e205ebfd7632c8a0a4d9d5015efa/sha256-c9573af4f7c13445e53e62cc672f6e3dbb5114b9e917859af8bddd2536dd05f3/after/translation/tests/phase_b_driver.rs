//! Phase B — `CONFIGS.md` rows C32–C37: the `driver` one-shot wrapper.
//!
//! `driver` hard-codes the relative output path `"matrix.txt"`, so every test
//! here runs with the process working directory pointed at a private scratch
//! directory. The working directory is process-global, so all tests in this
//! binary serialise on one mutex.

mod common;

use common::*;
use std::ffi::c_int;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn cwd_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

struct Cwd {
    original: std::path::PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl Cwd {
    fn enter(dir: &Path) -> Cwd {
        let guard = cwd_lock();
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir).expect("chdir into scratch dir");
        Cwd {
            original,
            _guard: guard,
        }
    }
}

impl Drop for Cwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

/// Runs `driver` in both libraries in the current directory, moving the
/// produced `matrix.txt` aside between runs, and returns
/// `[(rc, file_bytes), (rc, file_bytes)]` in `[C, Rust]` order.
fn run_driver_both(
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    b: &str,
) -> Vec<(c_int, Option<Vec<u8>>)> {
    let ca = cstring(a);
    let cb = cstring(b);
    let mut out = Vec::new();
    for api in both() {
        let _ = std::fs::remove_file("matrix.txt");
        let rc = unsafe { (api.driver)(wa, ha, ca.as_ptr(), wb, hb, cb.as_ptr()) };
        let bytes = std::fs::read("matrix.txt").ok();
        out.push((rc, bytes));
    }
    out
}

fn diff_driver(wa: c_int, ha: c_int, a: &str, wb: c_int, hb: c_int, b: &str) -> (c_int, Option<Vec<u8>>) {
    let out = run_driver_both(wa, ha, a, wb, hb, b);
    assert_eq!(
        out[0].0, out[1].0,
        "driver rc diverged for A({wa}x{ha})={a:?} B({wb}x{hb})={b:?}"
    );
    assert_eq!(
        out[0].1.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        out[1].1.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        "driver matrix.txt diverged for A({wa}x{ha})={a:?} B({wb}x{hb})={b:?}"
    );
    out.into_iter().next().unwrap()
}

fn rand_rows(rng: &mut Rng, h: usize, w: usize, lo: c_int, hi: c_int) -> Vec<Vec<c_int>> {
    (0..h)
        .map(|_| (0..w).map(|_| rng.i32_in(lo, hi)).collect())
        .collect()
}

/// Reference product, computed with wrapping `int` arithmetic like the C.
fn expected_product(a: &[Vec<c_int>], b: &[Vec<c_int>], n: usize) -> Vec<Vec<c_int>> {
    let m = a.len();
    let k = if m == 0 { b.len() } else { a[0].len() };
    (0..m)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let mut acc: c_int = 0;
                    for kk in 0..k {
                        acc = acc.wrapping_add(a[i][kk].wrapping_mul(b[kk][j]));
                    }
                    acc
                })
                .collect()
        })
        .collect()
}

#[test]
fn c32_driver_square_shapes() {
    let dir = scratch_dir("c32");
    let _cwd = Cwd::enter(&dir);
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..150 {
        let n = rng.range(1, 5) as usize;
        let a = rand_rows(&mut rng, n, n, -50, 50);
        let b = rand_rows(&mut rng, n, n, -50, 50);
        let (rc, bytes) = diff_driver(
            n as c_int,
            n as c_int,
            &canonical(&a),
            n as c_int,
            n as c_int,
            &canonical(&b),
        );
        assert_eq!(rc, 0);
        assert_eq!(
            bytes.unwrap(),
            canonical(&expected_product(&a, &b, n)).into_bytes()
        );
    }
}

#[test]
fn c33_driver_non_square_conformable() {
    let dir = scratch_dir("c33");
    let _cwd = Cwd::enter(&dir);
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..200 {
        let m = rng.range(1, 5) as usize;
        let k = rng.range(1, 5) as usize;
        let n = rng.range(1, 5) as usize;
        let a = rand_rows(&mut rng, m, k, -50, 50);
        let b = rand_rows(&mut rng, k, n, -50, 50);
        let (rc, bytes) = diff_driver(
            k as c_int,
            m as c_int,
            &canonical(&a),
            n as c_int,
            k as c_int,
            &canonical(&b),
        );
        assert_eq!(rc, 0);
        assert_eq!(
            bytes.unwrap(),
            canonical(&expected_product(&a, &b, n)).into_bytes()
        );
    }
}

#[test]
fn c34_driver_shared_dimension_zero() {
    let dir = scratch_dir("c34");
    let _cwd = Cwd::enter(&dir);
    for m in 1..=3usize {
        for n in 1..=3usize {
            // A is m x 0, B is 0 x n. `width_a == 0` means no column token is
            // ever read, but the row loop still needs `m` non-empty rows.
            let a_str = "0\n".repeat(m + 2);
            let (rc, bytes) = diff_driver(0, m as c_int, &a_str, n as c_int, 0, "");
            assert_eq!(rc, 0);
            let zeros: Vec<Vec<c_int>> = (0..m).map(|_| vec![0; n]).collect();
            assert_eq!(bytes.unwrap(), canonical(&zeros).into_bytes());
        }
    }
}

#[test]
fn c35_driver_all_zero_dimensions() {
    let dir = scratch_dir("c35");
    let _cwd = Cwd::enter(&dir);
    for a in ["", "\n", "1 2 3\n", "junk"] {
        for b in ["", "\n", "4 5\n", "junk"] {
            let (rc, bytes) = diff_driver(0, 0, a, 0, 0, b);
            assert_eq!(rc, 0);
            assert_eq!(bytes.unwrap(), b"");
        }
    }
}

#[test]
fn c36_driver_irregular_whitespace_and_extras() {
    let dir = scratch_dir("c36");
    let _cwd = Cwd::enter(&dir);
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..150 {
        let m = rng.range(1, 4) as usize;
        let k = rng.range(1, 4) as usize;
        let n = rng.range(1, 4) as usize;
        let extra_r = rng.range(0, 2) as usize;
        let extra_c = rng.range(0, 2) as usize;
        let a_full = rand_rows(&mut rng, m + extra_r, k + extra_c, -40, 40);
        let b_full = rand_rows(&mut rng, k + extra_r, n + extra_c, -40, 40);

        // messy rendering: leading blank lines, runs of spaces, padded rows
        let render = |rows: &Vec<Vec<c_int>>, rng: &mut Rng| {
            let mut s = String::new();
            for _ in 0..rng.range(0, 2) {
                s.push('\n');
            }
            for r in rows {
                for _ in 0..rng.range(0, 2) {
                    s.push(' ');
                }
                for (idx, v) in r.iter().enumerate() {
                    if idx > 0 {
                        for _ in 0..rng.range(1, 3) {
                            s.push(' ');
                        }
                    }
                    s.push_str(&v.to_string());
                }
                for _ in 0..rng.range(0, 2) {
                    s.push(' ');
                }
                for _ in 0..rng.range(1, 3) {
                    s.push('\n');
                }
            }
            s
        };
        let a_str = render(&a_full, &mut rng);
        let b_str = render(&b_full, &mut rng);

        let (rc, bytes) = diff_driver(
            k as c_int,
            m as c_int,
            &a_str,
            n as c_int,
            k as c_int,
            &b_str,
        );
        assert_eq!(rc, 0, "expected success for A={a_str:?} B={b_str:?}");
        let a: Vec<Vec<c_int>> = a_full[..m].iter().map(|r| r[..k].to_vec()).collect();
        let b: Vec<Vec<c_int>> = b_full[..k].iter().map(|r| r[..n].to_vec()).collect();
        assert_eq!(
            bytes.unwrap(),
            canonical(&expected_product(&a, &b, n)).into_bytes()
        );
    }
}

#[test]
fn c37_driver_wrapping_products() {
    let dir = scratch_dir("c37");
    let _cwd = Cwd::enter(&dir);
    let mut rng = Rng::new(SEED ^ 37);
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1,
        0,
        1,
        2_000_000_000,
        i32::MAX,
    ];
    // The result is `height_a x 1`; with width 1 the C's string buffer holds an
    // 11-character value plus its newline exactly, so no case here relies on
    // the overflow quirk (which is covered in `subprocess_parity.rs`).
    for _ in 0..200 {
        let m = rng.range(1, 4) as usize;
        let k = rng.range(1, 4) as usize;
        let a: Vec<Vec<c_int>> = (0..m)
            .map(|_| (0..k).map(|_| *rng.pick(&extremes)).collect())
            .collect();
        let b: Vec<Vec<c_int>> = (0..k).map(|_| vec![*rng.pick(&extremes)]).collect();
        let (rc, bytes) = diff_driver(
            k as c_int,
            m as c_int,
            &canonical(&a),
            1,
            k as c_int,
            &canonical(&b),
        );
        assert_eq!(rc, 0);
        assert_eq!(
            bytes.unwrap(),
            canonical(&expected_product(&a, &b, 1)).into_bytes()
        );
    }
}
