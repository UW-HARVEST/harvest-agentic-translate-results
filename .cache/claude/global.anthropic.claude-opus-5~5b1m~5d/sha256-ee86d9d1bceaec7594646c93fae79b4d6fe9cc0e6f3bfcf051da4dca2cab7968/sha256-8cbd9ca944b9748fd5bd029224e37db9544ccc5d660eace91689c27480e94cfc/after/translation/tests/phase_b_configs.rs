//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are reached only through `dlopen`ed C symbols.

mod common;

use common::*;
use std::ffi::{c_int, CString};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Dimensions only — used for `allocate_matrix`, whose cells are *uninitialised*
/// `malloc` memory and therefore legitimately differ between the two calls.
unsafe fn dims(m: *const MatrixT) -> Option<(c_int, c_int)> {
    if m.is_null() {
        None
    } else {
        Some(((*m).width, (*m).height))
    }
}

fn alloc_free_row(tag: &str, cases: &[(c_int, c_int)]) {
    let _g = global_lock();
    let (c, r) = both();
    for &(w, h) in cases {
        let (cd, ce) = capture_stderr(&format!("{tag}_c"), || unsafe {
            let m = (c.allocate_matrix)(w, h);
            let d = dims(m);
            (c.free_matrix)(m);
            d
        });
        let (rd, re) = capture_stderr(&format!("{tag}_r"), || unsafe {
            let m = (r.allocate_matrix)(w, h);
            let d = dims(m);
            (r.free_matrix)(m);
            d
        });
        assert_eq!(cd, rd, "{tag}: allocate_matrix({w},{h}) dims/NULL-ness");
        assert_eq_stderr(&format!("{tag}: allocate_matrix({w},{h})"), &ce, &re);
    }
}

/// Drive `initialize_matrix_from_string` + `matrix_to_string` on both libs and
/// compare snapshot, rendered bytes and stderr.
fn init_row(tag: &str, input: &str, w: c_int, h: c_int) {
    let _g = global_lock();
    let s = cs(input);
    let (c, r) = both();
    let (cs_, cstr_, cerr) = init_and_render(c, &format!("{tag}_c"), &s, w, h);
    let (rs_, rstr_, rerr) = init_and_render(r, &format!("{tag}_r"), &s, w, h);
    let ctx = format!("{tag}: init({:?},{w},{h})", input);
    assert_eq_snap(&ctx, &cs_, &rs_);
    assert_eq_bytes(&ctx, &cstr_, &rstr_);
    assert_eq_stderr(&ctx, &cerr, &rerr);
}

/// Build `a` and `b` from strings with the *same* library, multiply, render.
struct MulOut {
    a: Option<Snap>,
    b: Option<Snap>,
    res: Option<Snap>,
    rendered: Option<Vec<u8>>,
}

fn mul_with(
    api: &Api,
    tag: &str,
    ia: &CString,
    wa: c_int,
    ha: c_int,
    ib: &CString,
    wb: c_int,
    hb: c_int,
    render_result: bool,
) -> (MulOut, Vec<u8>) {
    capture_stderr(tag, || unsafe {
        let a = (api.initialize_matrix_from_string)(ia.as_ptr(), wa, ha);
        let b = (api.initialize_matrix_from_string)(ib.as_ptr(), wb, hb);
        let res = (api.multiply_matrices)(a, b);
        let out = MulOut {
            a: snapshot(a),
            b: snapshot(b),
            res: snapshot(res),
            rendered: if render_result {
                take_cstring((api.matrix_to_string)(res))
            } else {
                None
            },
        };
        (api.free_matrix)(res);
        (api.free_matrix)(b);
        (api.free_matrix)(a);
        out
    })
}

#[allow(clippy::too_many_arguments)]
fn mul_row(
    tag: &str,
    ia: &str,
    wa: c_int,
    ha: c_int,
    ib: &str,
    wb: c_int,
    hb: c_int,
    render_result: bool,
) {
    let _g = global_lock();
    let (c, r) = both();
    let sa = cs(ia);
    let sb = cs(ib);
    let (co, ce) = mul_with(c, &format!("{tag}_c"), &sa, wa, ha, &sb, wb, hb, render_result);
    let (ro, re) = mul_with(r, &format!("{tag}_r"), &sa, wa, ha, &sb, wb, hb, render_result);
    let ctx = format!("{tag}: mul(({wa}x{ha}),({wb}x{hb}))");
    assert_eq_snap(&format!("{ctx} a"), &co.a, &ro.a);
    assert_eq_snap(&format!("{ctx} b"), &co.b, &ro.b);
    assert_eq_snap(&format!("{ctx} res"), &co.res, &ro.res);
    assert_eq_bytes(&format!("{ctx} rendered"), &co.rendered, &ro.rendered);
    assert_eq_stderr(&ctx, &ce, &re);
}

fn write_row(tag: &str, filename: &std::path::Path, content: &[u8]) {
    let _g = global_lock();
    let (c, r) = both();
    let content = cs_bytes(content);

    let cfile = filename.with_extension("c.out");
    let rfile = filename.with_extension("rust.out");
    let cname = cs(cfile.to_str().unwrap());
    let rname = cs(rfile.to_str().unwrap());

    let (crc, cerr) = capture_stderr(&format!("{tag}_c"), || unsafe {
        (c.write_to_file)(cname.as_ptr(), content.as_ptr())
    });
    let (rrc, rerr) = capture_stderr(&format!("{tag}_r"), || unsafe {
        (r.write_to_file)(rname.as_ptr(), content.as_ptr())
    });

    assert_eq!(crc, rrc, "{tag}: write_to_file return code");
    assert_eq_stderr(&format!("{tag}: write_to_file"), &cerr, &rerr);
    let cbytes = std::fs::read(&cfile).unwrap_or_default();
    let rbytes = std::fs::read(&rfile).unwrap_or_default();
    assert!(
        cbytes == rbytes,
        "{tag}: file contents differ\n  C   : {}\n  Rust: {}",
        show(&cbytes),
        show(&rbytes)
    );
}

/// Run `driver` in `dir` with both libs; compare rc, stderr and `matrix.txt`.
#[allow(clippy::too_many_arguments)]
fn driver_row(tag: &str, wa: c_int, ha: c_int, ma: &str, wb: c_int, hb: c_int, mb: &str) {
    let _g = global_lock();
    let (c, r) = both();
    let dir = scratch_dir(tag);
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&dir).expect("chdir scratch");

    let sa = cs(ma);
    let sb = cs(mb);

    let (crc, cerr) = capture_stderr(&format!("{tag}_c"), || unsafe {
        (c.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
    });
    let cout = std::fs::read("matrix.txt").ok();
    let _ = std::fs::remove_file("matrix.txt");

    let (rrc, rerr) = capture_stderr(&format!("{tag}_r"), || unsafe {
        (r.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
    });
    let rout = std::fs::read("matrix.txt").ok();

    std::env::set_current_dir(&prev).expect("restore cwd");

    assert_eq!(crc, rrc, "{tag}: driver return code");
    assert_eq_stderr(&format!("{tag}: driver"), &cerr, &rerr);
    assert!(
        cout == rout,
        "{tag}: matrix.txt differs\n  C   : {:?}\n  Rust: {:?}",
        cout.as_deref().map(show),
        rout.as_deref().map(show)
    );
}

// ===========================================================================
// Row 1 — allocate_matrix/free_matrix 1x1
// ===========================================================================

#[test]
fn cfg_01_alloc_free_1x1() {
    alloc_free_row("cfg01", &[(1, 1)]);
}

// ===========================================================================
// Row 2 — height == 0
// ===========================================================================

#[test]
fn cfg_02_alloc_zero_height() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases = vec![(0, 0)];
    for _ in 0..64 {
        cases.push((rng.range_i32(0, 64), 0));
    }
    alloc_free_row("cfg02", &cases);
}

// ===========================================================================
// Row 3 — width == 0
// ===========================================================================

#[test]
fn cfg_03_alloc_zero_width() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut cases = vec![];
    for _ in 0..64 {
        cases.push((0, rng.range_i32(1, 64)));
    }
    alloc_free_row("cfg03", &cases);
}

// ===========================================================================
// Row 4 — randomized small dims
// ===========================================================================

#[test]
fn cfg_04_alloc_random() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases = vec![];
    for _ in 0..256 {
        cases.push((rng.range_i32(1, 64), rng.range_i32(1, 64)));
    }
    alloc_free_row("cfg04", &cases);
}

// ===========================================================================
// Row 5 — large-but-plausible dims
// ===========================================================================

#[test]
fn cfg_05_alloc_large() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut cases = vec![(4096, 4096), (1, 100_000), (100_000, 1), (1 << 20, 1)];
    for _ in 0..8 {
        cases.push((rng.range_i32(1, 4096), rng.range_i32(1, 4096)));
    }
    alloc_free_row("cfg05", &cases);
}

// ===========================================================================
// Row 6 — exact-fit input, randomized
// ===========================================================================

#[test]
fn cfg_06_init_exact_fit() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..200 {
        let w = rng.range_usize(1, 12);
        let h = rng.range_usize(1, 12);
        let cells = random_cells(&mut rng, w * h, -10_000, 10_000);
        let input = render(&cells, w, h);
        init_row("cfg06", &input, w as c_int, h as c_int);
    }
}

// ===========================================================================
// Row 7 — trailing newline
// ===========================================================================

#[test]
fn cfg_07_init_trailing_newline() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..100 {
        let w = rng.range_usize(1, 8);
        let h = rng.range_usize(1, 8);
        let cells = random_cells(&mut rng, w * h, -9_999, 9_999);
        let mut input = render(&cells, w, h);
        input.push('\n');
        init_row("cfg07", &input, w as c_int, h as c_int);
    }
    init_row("cfg07b", "1 2\n3 4\n", 2, 2);
}

// ===========================================================================
// Row 8 — surplus rows and columns are ignored
// ===========================================================================

#[test]
fn cfg_08_init_surplus_tokens() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..100 {
        let w = rng.range_usize(1, 6);
        let h = rng.range_usize(1, 6);
        let extra_w = rng.range_usize(1, 4);
        let extra_h = rng.range_usize(1, 4);
        let cells = random_cells(&mut rng, (w + extra_w) * (h + extra_h), -5_000, 5_000);
        let input = render(&cells, w + extra_w, h + extra_h);
        init_row("cfg08", &input, w as c_int, h as c_int);
    }
    init_row("cfg08b", "1 2 3\n4 5 6\n7 8 9", 2, 2);
}

// ===========================================================================
// Row 9 — consecutive / leading / trailing delimiters
// ===========================================================================

#[test]
fn cfg_09_init_collapsed_delimiters() {
    for (input, w, h) in [
        ("1  2\n3  4", 2, 2),
        ("  1 2  \n  3 4  ", 2, 2),
        ("1 2\n\n\n3 4", 2, 2),
        ("\n\n1 2\n3 4", 2, 2),
        ("1 2\n3 4\n\n\n", 2, 2),
        ("1\t2\n3\t4", 1, 2),
        ("     ", 1, 1),
        ("\n\n\n", 1, 1),
        ("1          2", 2, 1),
        (" ", 1, 1),
        ("", 1, 1),
        ("", 0, 0),
    ] {
        init_row("cfg09", input, w, h);
    }

    // randomized delimiter noise
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..150 {
        let w = rng.range_usize(1, 5);
        let h = rng.range_usize(1, 5);
        let cells = random_cells(&mut rng, w * h, -100, 100);
        let mut input = String::new();
        for _ in 0..rng.range_usize(0, 2) {
            input.push('\n');
        }
        for i in 0..h {
            for _ in 0..rng.range_usize(0, 3) {
                input.push(' ');
            }
            for j in 0..w {
                if j > 0 {
                    for _ in 0..rng.range_usize(1, 4) {
                        input.push(' ');
                    }
                }
                input.push_str(&cells[i * w + j].to_string());
            }
            for _ in 0..rng.range_usize(0, 3) {
                input.push(' ');
            }
            for _ in 0..rng.range_usize(1, 3) {
                input.push('\n');
            }
        }
        init_row("cfg09r", &input, w as c_int, h as c_int);
    }
}

// ===========================================================================
// Row 10 — width == 0
// ===========================================================================

#[test]
fn cfg_10_init_zero_width() {
    for (input, h) in [("a\nb\nc", 3), ("1 2\n3 4", 2), ("\n\n", 1), ("x", 1)] {
        init_row("cfg10", input, 0, h);
    }
}

// ===========================================================================
// Row 11 — height == 0
// ===========================================================================

#[test]
fn cfg_11_init_zero_height() {
    for (input, w) in [("", 0), ("", 5), ("1 2 3", 3), ("nonsense", 100)] {
        init_row("cfg11", input, w, 0);
    }
}

// ===========================================================================
// Row 12 — atoi edge tokens
// ===========================================================================

const ATOI_EDGES: &[&str] = &[
    "0",
    "-0",
    "+0",
    "+5",
    "-5",
    "007",
    "2147483647",
    "-2147483648",
    "2147483646",
    "-2147483647",
    "12abc",
    "abc",
    "-",
    "+",
    "0x10",
    "1e3",
    "3.9",
    "-3.9",
    "999999999",
    "-999999999",
    "1000000000",
    "--7",
    "7-",
    ".5",
    "z",
];

#[test]
fn cfg_12_init_atoi_edges() {
    // width == 1 so that `matrix_to_string`'s sizing formula is exact even for
    // 11-character renderings (see CONFIGS.md axis H).
    for tok in ATOI_EDGES {
        init_row("cfg12", tok, 1, 1);
    }
    // Several tokens per row, values kept short so rendering stays in bounds.
    let short: Vec<&str> = ATOI_EDGES
        .iter()
        .copied()
        .filter(|t| t.len() <= 4)
        .collect();
    let joined = short.join(" ");
    init_row("cfg12b", &joined, short.len() as c_int, 1);

    // atoi inputs one step past the int range — glibc atoi saturates; whatever
    // it does, both builds call the same atoi and must agree.
    for tok in [
        "2147483648",
        "-2147483649",
        "4294967296",
        "99999999999999999999",
        "-99999999999999999999",
    ] {
        init_row("cfg12c", tok, 1, 1);
    }
}

// ===========================================================================
// Row 13 — randomized token-text fuzz
// ===========================================================================

#[test]
fn cfg_13_init_token_fuzz() {
    let mut rng = Rng::new(SEED ^ 13);
    const ALPHA: &[u8] = b"0123456789+-abcXZ. ";
    for _ in 0..300 {
        let h = rng.range_usize(1, 4);
        let mut input = String::new();
        for i in 0..h {
            if i > 0 {
                input.push('\n');
            }
            let n = rng.range_usize(1, 10);
            for _ in 0..n {
                input.push(*rng.pick(ALPHA) as char);
            }
        }
        // width 1 keeps rendering exact regardless of the parsed magnitude
        init_row("cfg13", &input, 1, h as c_int);
    }
}

// ===========================================================================
// Row 14 — matrix_to_string with width == 1 and extreme values
// ===========================================================================

#[test]
fn cfg_14_to_string_width1_extremes() {
    let vals = [
        "-2147483648",
        "2147483647",
        "0",
        "-1",
        "1",
        "-1000000000",
        "1000000000",
    ];
    for v in vals {
        init_row("cfg14", v, 1, 1);
    }
    // A tall single-column matrix of 11-character renderings: the C formula
    // gives 12 bytes per row, which is exactly what is needed when width == 1.
    let tall = vals
        .iter()
        .cycle()
        .take(40)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    init_row("cfg14b", &tall, 1, 40);
}

// ===========================================================================
// Row 15 — matrix_to_string with width == 0
// ===========================================================================

#[test]
fn cfg_15_to_string_zero_width() {
    for h in [1, 2, 5, 17] {
        let input = std::iter::repeat("x").take(h).collect::<Vec<_>>().join("\n");
        init_row("cfg15", &input, 0, h as c_int);
    }
}

// ===========================================================================
// Row 16 — matrix_to_string with height == 0
// ===========================================================================

#[test]
fn cfg_16_to_string_zero_height() {
    for w in [0, 1, 3, 50] {
        init_row("cfg16", "", w, 0);
    }
}

// ===========================================================================
// Row 17 — matrix_to_string, width > 1, randomized (renderings <= 10 chars)
// ===========================================================================

#[test]
fn cfg_17_to_string_random() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..200 {
        let w = rng.range_usize(2, 10);
        let h = rng.range_usize(1, 10);
        // |v| <= 999_999_999 renders in at most 10 characters, which is the
        // largest width the C sizing formula actually accommodates.
        let cells = random_cells(&mut rng, w * h, -999_999_999, 999_999_999);
        let input = render(&cells, w, h);
        init_row("cfg17", &input, w as c_int, h as c_int);
    }
}

// ===========================================================================
// Row 18 — square x square
// ===========================================================================

#[test]
fn cfg_18_mul_square() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..120 {
        let n = rng.range_usize(1, 12);
        let a = random_cells(&mut rng, n * n, -100, 100);
        let b = random_cells(&mut rng, n * n, -100, 100);
        mul_row(
            "cfg18",
            &render(&a, n, n),
            n as c_int,
            n as c_int,
            &render(&b, n, n),
            n as c_int,
            n as c_int,
            true,
        );
    }
}

// ===========================================================================
// Row 19 — non-square compatible
// ===========================================================================

#[test]
fn cfg_19_mul_nonsquare() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..120 {
        let ha = rng.range_usize(1, 10);
        let wa = rng.range_usize(1, 10);
        let wb = rng.range_usize(1, 10);
        let hb = wa; // required for compatibility
        let a = random_cells(&mut rng, wa * ha, -100, 100);
        let b = random_cells(&mut rng, wb * hb, -100, 100);
        mul_row(
            "cfg19",
            &render(&a, wa, ha),
            wa as c_int,
            ha as c_int,
            &render(&b, wb, hb),
            wb as c_int,
            hb as c_int,
            true,
        );
    }
}

// ===========================================================================
// Row 20 — inner dimension 0
// ===========================================================================

#[test]
fn cfg_20_mul_zero_inner() {
    for (ha, wb) in [(1usize, 1usize), (3, 4), (5, 1), (1, 6)] {
        let ia = std::iter::repeat("x").take(ha).collect::<Vec<_>>().join("\n");
        mul_row(
            "cfg20",
            &ia,
            0,
            ha as c_int,
            "",
            wb as c_int,
            0,
            true,
        );
    }
}

// ===========================================================================
// Row 21 — 1xn . nx1 dot product
// ===========================================================================

#[test]
fn cfg_21_mul_dot() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..100 {
        let n = rng.range_usize(1, 16);
        let a = random_cells(&mut rng, n, -1000, 1000);
        let b = random_cells(&mut rng, n, -1000, 1000);
        mul_row(
            "cfg21",
            &render(&a, n, 1),
            n as c_int,
            1,
            &render(&b, 1, n),
            1,
            n as c_int,
            true,
        );
    }
}

// ===========================================================================
// Row 22 — nx1 . 1xm outer product
// ===========================================================================

#[test]
fn cfg_22_mul_outer() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..100 {
        let n = rng.range_usize(1, 10);
        let m = rng.range_usize(1, 10);
        let a = random_cells(&mut rng, n, -1000, 1000);
        let b = random_cells(&mut rng, m, -1000, 1000);
        mul_row(
            "cfg22",
            &render(&a, 1, n),
            1,
            n as c_int,
            &render(&b, m, 1),
            m as c_int,
            1,
            true,
        );
    }
}

// ===========================================================================
// Row 23 — empty result dimension
// ===========================================================================

#[test]
fn cfg_23_mul_empty_result() {
    // height_a == 0  => result is (width_b x 0)
    for (wa, wb, hb) in [(0i32, 3i32, 0i32), (2, 3, 2), (1, 1, 1)] {
        let ib = if hb == 0 {
            String::new()
        } else {
            let cells: Vec<i32> = (0..(wb * hb)).collect();
            render(&cells, wb as usize, hb as usize)
        };
        mul_row("cfg23a", "", wa, 0, &ib, wb, hb, true);
    }
    // width_b == 0 => result is (0 x height_a)
    for (wa, ha) in [(1i32, 3i32), (2, 2), (0, 4)] {
        let cells: Vec<i32> = (0..(wa * ha)).collect();
        let ia = if wa == 0 || ha == 0 {
            std::iter::repeat("x")
                .take(ha as usize)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            render(&cells, wa as usize, ha as usize)
        };
        let ib = std::iter::repeat("x")
            .take(wa.max(0) as usize)
            .collect::<Vec<_>>()
            .join("\n");
        mul_row("cfg23b", &ia, wa, ha, &ib, 0, wa, true);
    }
}

// ===========================================================================
// Row 24 — overflowing products / accumulations (no rendering: results may
// need 11 characters, which the C sizing formula cannot hold for width > 1)
// ===========================================================================

#[test]
fn cfg_24_mul_overflow() {
    let mut rng = Rng::new(SEED ^ 24);
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1,
        1,
        0,
        65536,
        -65536,
        46341,
        -46341,
        1 << 30,
    ];
    for _ in 0..150 {
        let n = rng.range_usize(1, 6);
        let a: Vec<i32> = (0..n * n).map(|_| *rng.pick(&extremes)).collect();
        let b: Vec<i32> = (0..n * n).map(|_| *rng.pick(&extremes)).collect();
        mul_row(
            "cfg24",
            &render(&a, n, n),
            n as c_int,
            n as c_int,
            &render(&b, n, n),
            n as c_int,
            n as c_int,
            false,
        );
    }
    // width_b == 1 -> the rendered result is safe, so render it too.
    for _ in 0..60 {
        let n = rng.range_usize(1, 6);
        let a: Vec<i32> = (0..n * n).map(|_| *rng.pick(&extremes)).collect();
        let b: Vec<i32> = (0..n).map(|_| *rng.pick(&extremes)).collect();
        mul_row(
            "cfg24b",
            &render(&a, n, n),
            n as c_int,
            n as c_int,
            &render(&b, 1, n),
            1,
            n as c_int,
            true,
        );
    }
}

// ===========================================================================
// Row 25 — full pipeline init -> multiply -> to_string
// ===========================================================================

#[test]
fn cfg_25_pipeline_random() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..200 {
        let ha = rng.range_usize(1, 8);
        let wa = rng.range_usize(1, 8);
        let wb = rng.range_usize(1, 8);
        // |cell| <= 1000 and inner dim <= 8 keeps |result| <= 8e6 (7 chars)
        let a = random_cells(&mut rng, wa * ha, -1000, 1000);
        let b = random_cells(&mut rng, wb * wa, -1000, 1000);
        mul_row(
            "cfg25",
            &render(&a, wa, ha),
            wa as c_int,
            ha as c_int,
            &render(&b, wb, wa),
            wb as c_int,
            wa as c_int,
            true,
        );
    }
}

// ===========================================================================
// Row 26 — write_to_file, fresh file
// ===========================================================================

#[test]
fn cfg_26_write_fresh() {
    let dir = scratch_dir("cfg26");
    write_row("cfg26", &dir.join("out"), b"hello world\n");
}

// ===========================================================================
// Row 27 — mode "w" truncates an existing longer file
// ===========================================================================

#[test]
fn cfg_27_write_truncates() {
    let dir = scratch_dir("cfg27");
    let base = dir.join("out");
    std::fs::write(base.with_extension("c.out"), vec![b'Z'; 4096]).unwrap();
    std::fs::write(base.with_extension("rust.out"), vec![b'Z'; 4096]).unwrap();
    write_row("cfg27", &base, b"short");
    // and again, now shrinking from the previous 5 bytes to 0
    write_row("cfg27b", &base, b"");
}

// ===========================================================================
// Row 28 — empty content
// ===========================================================================

#[test]
fn cfg_28_write_empty() {
    let dir = scratch_dir("cfg28");
    write_row("cfg28", &dir.join("out"), b"");
}

// ===========================================================================
// Row 29 — printf specifiers in the content must stay literal
// ===========================================================================

#[test]
fn cfg_29_write_percent_literal() {
    let dir = scratch_dir("cfg29");
    for (i, s) in [
        &b"100%"[..],
        b"%s %d %n %p",
        b"%%",
        b"%s%s%s%s%s%s%s%s",
        b"a%",
        b"%",
    ]
    .iter()
    .enumerate()
    {
        write_row(&format!("cfg29_{i}"), &dir.join(format!("out{i}")), s);
    }
}

// ===========================================================================
// Row 30 — random byte content, lengths crossing BUFSIZ
// ===========================================================================

#[test]
fn cfg_30_write_random_bytes() {
    let dir = scratch_dir("cfg30");
    let mut rng = Rng::new(SEED ^ 30);
    for i in 0..40 {
        let n = rng.range_usize(0, 8192);
        // any byte except NUL (a C string cannot carry an interior NUL)
        let content: Vec<u8> = (0..n).map(|_| rng.range_i32(1, 255) as u8).collect();
        write_row(&format!("cfg30_{i}"), &dir.join(format!("out{i}")), &content);
    }
    // exactly around BUFSIZ (4096) and 8192
    for (i, n) in [4095usize, 4096, 4097, 8191, 8192, 8193].iter().enumerate() {
        let content: Vec<u8> = (0..*n).map(|k| ((k % 255) + 1) as u8).collect();
        write_row(&format!("cfg30b_{i}"), &dir.join(format!("bs{i}")), &content);
    }
}

// ===========================================================================
// Row 31 — odd but valid filenames
// ===========================================================================

#[test]
fn cfg_31_write_odd_filenames() {
    let dir = scratch_dir("cfg31");
    for (i, name) in ["a b c", "dot.ted.name", "ünïcødé", "-leading-dash", "..dots"]
        .iter()
        .enumerate()
    {
        write_row(&format!("cfg31_{i}"), &dir.join(name), b"payload\n");
    }
}

// ===========================================================================
// Row 32 — driver success, randomized
// ===========================================================================

#[test]
fn cfg_32_driver_success_random() {
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..60 {
        let ha = rng.range_usize(1, 6);
        let wa = rng.range_usize(1, 6);
        let wb = rng.range_usize(1, 6);
        let a = random_cells(&mut rng, wa * ha, -1000, 1000);
        let b = random_cells(&mut rng, wb * wa, -1000, 1000);
        driver_row(
            "cfg32",
            wa as c_int,
            ha as c_int,
            &render(&a, wa, ha),
            wb as c_int,
            wa as c_int,
            &render(&b, wb, wa),
        );
    }
}

// ===========================================================================
// Row 33 — degenerate dimensions through driver
// ===========================================================================

#[test]
fn cfg_33_driver_degenerate_dims() {
    // inner dimension 0: a is (0 x ha), b is (wb x 0)
    driver_row("cfg33a", 0, 3, "x\ny\nz", 4, 0, "");
    driver_row("cfg33b", 0, 1, "x", 1, 0, "");
    // height_a == 0: result is (wb x 0) -> empty string written
    driver_row("cfg33c", 2, 0, "", 3, 2, "1 2 3\n4 5 6");
    // width_b == 0: result is (0 x ha)
    driver_row("cfg33d", 2, 2, "1 2\n3 4", 0, 2, "x\ny");
    // both zero
    driver_row("cfg33e", 0, 0, "", 0, 0, "");
}

// ===========================================================================
// Row 34 — 1x1 driver including overflow
// ===========================================================================

#[test]
fn cfg_34_driver_1x1_overflow() {
    for (a, b) in [
        ("1", "1"),
        ("-1", "2147483647"),
        ("2147483647", "2147483647"),
        ("-2147483648", "-2147483648"),
        ("65536", "65536"),
        ("0", "-2147483648"),
        ("-2147483648", "-1"),
    ] {
        driver_row("cfg34", 1, 1, a, 1, 1, b);
    }
}

// ===========================================================================
// Row 35 — cross-library ABI interop on matrix_t
// ===========================================================================

#[test]
fn cfg_35_cross_library_interop() {
    let _g = global_lock();
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 35);

    for _ in 0..80 {
        let w = rng.range_usize(1, 8);
        let h = rng.range_usize(1, 8);
        let cells = random_cells(&mut rng, w * h, -999_999, 999_999);
        let input = cs(&render(&cells, w, h));

        // C-built matrix rendered by both implementations
        let (pair, err) = capture_stderr("cfg35_cbuilt", || unsafe {
            let m = (c.initialize_matrix_from_string)(input.as_ptr(), w as c_int, h as c_int);
            let via_c = take_cstring((c.matrix_to_string)(m));
            let via_r = take_cstring((r.matrix_to_string)(m));
            (c.free_matrix)(m);
            (via_c, via_r)
        });
        assert!(err.is_empty(), "cfg35: unexpected stderr {}", show(&err));
        assert_eq_bytes("cfg35 C-built matrix", &pair.0, &pair.1);

        // Rust-built matrix rendered by both implementations
        let (pair, err) = capture_stderr("cfg35_rbuilt", || unsafe {
            let m = (r.initialize_matrix_from_string)(input.as_ptr(), w as c_int, h as c_int);
            let via_c = take_cstring((c.matrix_to_string)(m));
            let via_r = take_cstring((r.matrix_to_string)(m));
            (r.free_matrix)(m);
            (via_c, via_r)
        });
        assert!(err.is_empty(), "cfg35: unexpected stderr {}", show(&err));
        assert_eq_bytes("cfg35 Rust-built matrix", &pair.0, &pair.1);
    }

    // multiply_matrices fed a C-built `a` and a Rust-built `b`, and vice versa
    for _ in 0..60 {
        let n = rng.range_usize(1, 6);
        let a = render(&random_cells(&mut rng, n * n, -500, 500), n, n);
        let b = render(&random_cells(&mut rng, n * n, -500, 500), n, n);
        let (sa, sb) = (cs(&a), cs(&b));
        let n = n as c_int;

        let (outs, err) = capture_stderr("cfg35_mixed", || unsafe {
            let ac = (c.initialize_matrix_from_string)(sa.as_ptr(), n, n);
            let ar = (r.initialize_matrix_from_string)(sa.as_ptr(), n, n);
            let bc = (c.initialize_matrix_from_string)(sb.as_ptr(), n, n);
            let br = (r.initialize_matrix_from_string)(sb.as_ptr(), n, n);

            let mut v = Vec::new();
            for (mul, x, y) in [
                (c.multiply_matrices, ac, bc),
                (c.multiply_matrices, ar, br),
                (c.multiply_matrices, ac, br),
                (c.multiply_matrices, ar, bc),
                (r.multiply_matrices, ac, bc),
                (r.multiply_matrices, ar, br),
                (r.multiply_matrices, ac, br),
                (r.multiply_matrices, ar, bc),
            ] {
                let res = mul(x, y);
                v.push(take_cstring((c.matrix_to_string)(res)));
                (c.free_matrix)(res);
            }

            (c.free_matrix)(ac);
            (r.free_matrix)(ar);
            (c.free_matrix)(bc);
            (r.free_matrix)(br);
            v
        });
        assert!(err.is_empty(), "cfg35: unexpected stderr {}", show(&err));
        for (i, out) in outs.iter().enumerate() {
            assert_eq_bytes(&format!("cfg35 mixed multiply #{i}"), &outs[0], out);
        }
    }
}

// ===========================================================================
// Row 36 — cross-library allocate/free on the shared malloc arena
// ===========================================================================

#[test]
fn cfg_36_cross_library_alloc_free() {
    let _g = global_lock();
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 36);
    for _ in 0..200 {
        let w = rng.range_i32(0, 32);
        let h = rng.range_i32(0, 32);
        let (dims_pair, err) = capture_stderr("cfg36", || unsafe {
            let mc = (c.allocate_matrix)(w, h);
            let mr = (r.allocate_matrix)(w, h);
            let d = (dims(mc), dims(mr));
            // freed by the *other* library
            (r.free_matrix)(mc);
            (c.free_matrix)(mr);
            d
        });
        assert!(err.is_empty(), "cfg36: unexpected stderr {}", show(&err));
        assert_eq!(
            dims_pair.0, dims_pair.1,
            "cfg36: allocate_matrix({w},{h}) dims"
        );
    }
}
