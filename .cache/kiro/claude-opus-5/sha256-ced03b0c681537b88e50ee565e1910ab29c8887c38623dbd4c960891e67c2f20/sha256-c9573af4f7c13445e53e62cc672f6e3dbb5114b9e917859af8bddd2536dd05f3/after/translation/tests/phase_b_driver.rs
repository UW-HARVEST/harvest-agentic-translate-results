//! Phase B — valid-path differential tests for the top-level `driver`
//! (rows 41–48 of `CONFIGS.md`).
//!
//! `driver` hard-codes its output to `OUT_FILE = "matrix.txt"`, resolved
//! against the process cwd, so every test here runs inside a private temp
//! directory. `set_current_dir` and the shared `matrix.txt` are process-wide,
//! so the whole file is serialised behind one mutex.

mod common;

use common::*;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static CWD_LOCK: Mutex<()> = Mutex::new(());
static DIR: OnceLock<PathBuf> = OnceLock::new();

fn enter() -> MutexGuard<'static, ()> {
    let g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("difftest-driver-b-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    });
    std::env::set_current_dir(d).unwrap();
    g
}

const OUT: &str = "matrix.txt";

struct Outcome {
    rc: c_int,
    out: Option<Vec<u8>>,
    err: Vec<u8>,
}

fn run_driver(api: &Api, wa: c_int, ha: c_int, a: &str, wb: c_int, hb: c_int, bb: &str) -> Outcome {
    let sa = cs(a);
    let sb = cs(bb);
    let _ = std::fs::remove_file(OUT);
    let (rc, err) = capture_stderr(|| unsafe {
        (api.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
    });
    let out = std::fs::read(OUT).ok();
    let _ = std::fs::remove_file(OUT);
    Outcome { rc, out, err }
}

fn check_driver(b: &Both, wa: c_int, ha: c_int, a: &str, wb: c_int, hb: c_int, bb: &str) {
    let oc = run_driver(&b.c, wa, ha, a, wb, hb, bb);
    let or = run_driver(&b.rs, wa, ha, a, wb, hb, bb);
    let ctx = format!("driver({wa},{ha},{a:?},{wb},{hb},{bb:?})");
    assert_eq!(oc.rc, or.rc, "{ctx} return code mismatch");
    assert_eq!(oc.out, or.out, "{ctx} matrix.txt mismatch");
    assert_eq!(
        String::from_utf8_lossy(&oc.err),
        String::from_utf8_lossy(&or.err),
        "{ctx} stderr mismatch"
    );
}

#[test]
fn row41_driver_1x1() {
    let _g = enter();
    let b = load_both();
    check_driver(&b, 1, 1, "6\n", 1, 1, "7\n");
    check_driver(&b, 1, 1, "-6", 1, 1, "7");
    check_driver(&b, 1, 1, "0\n", 1, 1, "0\n");
}

#[test]
fn row42_driver_square_3x3() {
    let _g = enter();
    let b = load_both();
    check_driver(
        &b,
        3,
        3,
        "1 2 3\n4 5 6\n7 8 9\n",
        3,
        3,
        "9 8 7\n6 5 4\n3 2 1\n",
    );
    check_driver(
        &b,
        3,
        3,
        "-1 0 1\n2 -2 0\n0 3 -3\n",
        3,
        3,
        "1 1 1\n1 1 1\n1 1 1\n",
    );
}

#[test]
fn row43_driver_general_rectangle() {
    let _g = enter();
    let b = load_both();
    // A is 4 wide x 2 high; B is 3 wide x 4 high -> result 3 wide x 2 high.
    check_driver(
        &b,
        4,
        2,
        "1 2 3 4\n5 6 7 8\n",
        3,
        4,
        "1 0 0\n0 1 0\n0 0 1\n2 2 2\n",
    );
    // Row vector times matrix, and matrix times column vector.
    check_driver(&b, 3, 1, "1 2 3\n", 2, 3, "1 4\n2 5\n3 6\n");
    check_driver(&b, 2, 3, "1 2\n3 4\n5 6\n", 1, 2, "7\n8\n");
}

#[test]
fn row44_driver_inner_dim_zero() {
    let _g = enter();
    let b = load_both();
    // width_a == 0 == height_b: multiplication is legal, every cell is 0.
    // Rows must still be present as tokens, so non-empty row text is used.
    check_driver(&b, 0, 2, "x\ny\n", 3, 0, "");
    check_driver(&b, 0, 1, "x", 1, 0, "");
    check_driver(&b, 0, 0, "", 0, 0, "");
}

#[test]
fn row45_driver_zero_output_dims() {
    let _g = enter();
    let b = load_both();
    // height_a == 0 -> zero rows of output (empty file).
    check_driver(&b, 2, 0, "", 3, 2, "1 2 3\n4 5 6\n");
    // width_b == 0 -> rows containing only newlines.
    check_driver(&b, 2, 2, "1 2\n3 4\n", 0, 2, "p\nq\n");
}

#[test]
fn row46_driver_messy_input_forms() {
    let _g = enter();
    let b = load_both();
    // Surplus rows/columns, separator runs, blank lines, non-numeric tokens.
    check_driver(
        &b,
        2,
        2,
        "1   2  9 9\n\n3\t3 4 7\n99 99\n",
        2,
        2,
        "  5 6 \n7 8\n",
    );
    check_driver(&b, 2, 2, "abc 12abc\n0x10 +5\n", 2, 2, "-0 007\n--3 .5\n");
    check_driver(&b, 1, 1, "2147483647\n", 1, 1, "2\n");
    check_driver(&b, 1, 1, "-2147483648\n", 1, 1, "-1\n");
}

#[test]
fn row47_driver_randomized() {
    let _g = enter();
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0047);
    for _ in 0..150 {
        let ha = rng.range(0, 6) as usize;
        let wa = rng.range(0, 6) as usize; // == height of B
        let wb = rng.range(0, 6) as usize;
        // Bound magnitudes so the product's decimal form stays inside the
        // buffer `matrix_to_string` allocates (<= 10 chars) — beyond that the C
        // itself overflows its buffer and has no defined output.
        let bound = 400i64;
        let va: Vec<c_int> = (0..ha * wa).map(|_| rng.range(-bound, bound) as c_int).collect();
        let vb: Vec<c_int> = (0..wa * wb).map(|_| rng.range(-bound, bound) as c_int).collect();
        let ta = render_matrix_text(wa, ha, &va);
        let tb = render_matrix_text(wb, wa, &vb);
        check_driver(&b, wa as c_int, ha as c_int, &ta, wb as c_int, wa as c_int, &tb);
    }
}

#[test]
fn row48_driver_truncates_existing_output() {
    let _g = enter();
    let b = load_both();
    let run = |api: &Api| {
        std::fs::write(OUT, vec![b'Z'; 8000]).unwrap();
        let sa = cs("1 2\n3 4\n");
        let sb = cs("5 6\n7 8\n");
        let (rc, err) = capture_stderr(|| unsafe {
            (api.driver)(2, 2, sa.as_ptr(), 2, 2, sb.as_ptr())
        });
        let out = std::fs::read(OUT).ok();
        let _ = std::fs::remove_file(OUT);
        (rc, out, err)
    };
    let (rc_c, out_c, err_c) = run(&b.c);
    let (rc_r, out_r, err_r) = run(&b.rs);
    assert_eq!(rc_c, rc_r);
    assert_eq!(out_c, out_r);
    assert_eq!(String::from_utf8_lossy(&err_c), String::from_utf8_lossy(&err_r));
    assert_eq!(rc_c, 0);
    assert_eq!(out_c.as_deref(), Some(&b"19 22\n43 50\n"[..]));
}
