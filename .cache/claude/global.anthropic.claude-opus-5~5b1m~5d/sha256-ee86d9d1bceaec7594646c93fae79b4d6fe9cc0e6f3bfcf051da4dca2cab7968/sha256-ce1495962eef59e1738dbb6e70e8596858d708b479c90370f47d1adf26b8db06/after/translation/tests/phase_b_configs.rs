//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH shared objects through their exported symbols only
//! (loaded with `libloading`) and compares the return value, every observable
//! byte of output, and the bytes written to `stderr`.

mod harness;

use harness::*;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// differential driver
// ---------------------------------------------------------------------------

/// Run `f` against the C `.so` and then the Rust `.so`; require identical
/// results and identical `stderr` bytes.
fn run_pair<T: PartialEq + std::fmt::Debug>(tag: &str, f: impl Fn(&Api) -> T) -> T {
    let _g = lock();
    let (rc, ec) = with_captured_stderr("c", || f(c_api()));
    let (rr, er) = with_captured_stderr("r", || f(rust_api()));
    assert_eq!(
        show(&ec),
        show(&er),
        "[{tag}] stderr differs\n  C   : {:?}\n  Rust: {:?}",
        show(&ec),
        show(&er)
    );
    assert_eq!(rc, rr, "[{tag}] result differs");
    rc
}

/// Shape-only observation (freshly `malloc`ed cells are uninitialised, so they
/// must not be compared).
unsafe fn observe_shape(mat: *mut MatrixT) -> (bool, c_int, c_int, bool) {
    if mat.is_null() {
        return (true, 0, 0, false);
    }
    (false, (*mat).width, (*mat).height, !(*mat).matrix.is_null())
}

// ---------------------------------------------------------------------------
// input-string generation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct TextOpts {
    /// separator emitted between column tokens
    col_sep: &'static str,
    /// prefix emitted before the first token of each row
    row_prefix: &'static str,
    /// suffix emitted after the last token of each row
    row_suffix: &'static str,
    /// separator emitted between rows
    row_sep: &'static str,
    /// text prepended to the whole string
    prefix: &'static str,
    /// text appended to the whole string
    suffix: &'static str,
    /// extra surplus tokens appended to every row
    extra_cols: usize,
    /// extra surplus rows appended to the string
    extra_rows: usize,
}

impl Default for TextOpts {
    fn default() -> Self {
        TextOpts {
            col_sep: " ",
            row_prefix: "",
            row_suffix: "",
            row_sep: "\n",
            prefix: "",
            suffix: "",
            extra_cols: 0,
            extra_rows: 0,
        }
    }
}

fn render(cells: &[Vec<i32>], o: &TextOpts, rng: &mut Rng) -> String {
    let mut rows: Vec<String> = Vec::new();
    for row in cells {
        let mut toks: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        for _ in 0..o.extra_cols {
            toks.push(rng.safe_cell().to_string());
        }
        rows.push(format!(
            "{}{}{}",
            o.row_prefix,
            toks.join(o.col_sep),
            o.row_suffix
        ));
    }
    let ncols = cells.first().map(|r| r.len()).unwrap_or(1).max(1);
    for _ in 0..o.extra_rows {
        let toks: Vec<String> = (0..ncols).map(|_| rng.safe_cell().to_string()).collect();
        rows.push(toks.join(o.col_sep));
    }
    format!("{}{}{}", o.prefix, rows.join(o.row_sep), o.suffix)
}

fn gen_cells(rng: &mut Rng, w: usize, h: usize, full_range: bool) -> Vec<Vec<i32>> {
    (0..h)
        .map(|_| {
            (0..w)
                .map(|_| {
                    if full_range {
                        rng.i32_full()
                    } else {
                        rng.safe_cell()
                    }
                })
                .collect()
        })
        .collect()
}

/// `initialize_matrix_from_string` → observe → `free_matrix`.
fn init_observe(api: &Api, text: &str, w: c_int, h: c_int) -> MatObs {
    let c = cstr(text);
    unsafe {
        let m = (api.initialize_matrix_from_string)(c.as_ptr(), w, h);
        let o = observe(m);
        (api.free_matrix)(m);
        o
    }
}

fn init_case(tag: &str, rng: &mut Rng, w: usize, h: usize, full_range: bool, o: TextOpts) {
    let cells = gen_cells(rng, w, h, full_range);
    let text = render(&cells, &o, rng);
    run_pair(tag, |api| init_observe(api, &text, w as c_int, h as c_int));
}

// ===========================================================================
// C1 — allocate_matrix + free_matrix over a dimension grid
// ===========================================================================
#[test]
fn c1_allocate_free_grid() {
    let dims: [c_int; 6] = [0, 1, 2, 3, 7, 64];
    for &w in &dims {
        for &h in &dims {
            run_pair(&format!("c1-{w}x{h}"), |api| unsafe {
                let m = (api.allocate_matrix)(w, h);
                let s = observe_shape(m);
                (api.free_matrix)(m);
                s
            });
        }
    }
}

// ===========================================================================
// C2 — allocate_matrix: write then read back every cell (row/col layout)
// ===========================================================================
#[test]
fn c2_allocate_roundtrip_cells() {
    let mut rng = Rng::new(2);
    for iter in 0..200u64 {
        let w = rng.range(0, 128) as c_int;
        let h = rng.range(0, 128) as c_int;
        let vals: Vec<c_int> = (0..(w as i64 * h as i64))
            .map(|_| rng.i32_full())
            .collect();
        run_pair(&format!("c2-{iter}-{w}x{h}"), |api| unsafe {
            let m = (api.allocate_matrix)(w, h);
            if m.is_null() {
                return MatObs::Null;
            }
            fill(m, &vals);
            let o = observe(m);
            (api.free_matrix)(m);
            o
        });
    }
}

// ===========================================================================
// C3 — exact fit, single spaces, no trailing newline
// ===========================================================================
#[test]
fn c3_init_exact_fit() {
    let mut rng = Rng::new(3);
    for iter in 0..300u64 {
        let w = rng.range(1, 8) as usize;
        let h = rng.range(1, 8) as usize;
        init_case(
            &format!("c3-{iter}-{w}x{h}"),
            &mut rng,
            w,
            h,
            false,
            TextOpts::default(),
        );
    }
}

// ===========================================================================
// C4 — exact fit with a trailing newline
// ===========================================================================
#[test]
fn c4_init_trailing_newline() {
    let mut rng = Rng::new(4);
    for iter in 0..200u64 {
        let w = rng.range(1, 8) as usize;
        let h = rng.range(1, 8) as usize;
        init_case(
            &format!("c4-{iter}-{w}x{h}"),
            &mut rng,
            w,
            h,
            false,
            TextOpts {
                suffix: "\n",
                ..Default::default()
            },
        );
    }
}

// ===========================================================================
// C5 — surplus columns in every row
// ===========================================================================
#[test]
fn c5_init_extra_columns() {
    let mut rng = Rng::new(5);
    for iter in 0..200u64 {
        let w = rng.range(1, 6) as usize;
        let h = rng.range(1, 6) as usize;
        let extra = rng.range(1, 5) as usize;
        init_case(
            &format!("c5-{iter}-{w}x{h}+{extra}"),
            &mut rng,
            w,
            h,
            false,
            TextOpts {
                extra_cols: extra,
                ..Default::default()
            },
        );
    }
}

// ===========================================================================
// C6 — surplus rows
// ===========================================================================
#[test]
fn c6_init_extra_rows() {
    let mut rng = Rng::new(6);
    for iter in 0..200u64 {
        let w = rng.range(1, 6) as usize;
        let h = rng.range(1, 6) as usize;
        let extra = rng.range(1, 4) as usize;
        init_case(
            &format!("c6-{iter}-{w}x{h}+{extra}r"),
            &mut rng,
            w,
            h,
            false,
            TextOpts {
                extra_rows: extra,
                ..Default::default()
            },
        );
    }
}

// ===========================================================================
// C7 — runs of spaces, leading/trailing spaces per row
// ===========================================================================
#[test]
fn c7_init_space_runs() {
    let mut rng = Rng::new(7);
    let seps = ["  ", "   ", "     "];
    let prefixes = ["", " ", "   "];
    let suffixes = ["", " ", "    "];
    for iter in 0..300u64 {
        let w = rng.range(1, 6) as usize;
        let h = rng.range(1, 6) as usize;
        let sep = *rng.pick(&seps);
        let pre = *rng.pick(&prefixes);
        let suf = *rng.pick(&suffixes);
        init_case(
            &format!("c7-{iter}-{w}x{h}"),
            &mut rng,
            w,
            h,
            false,
            TextOpts {
                col_sep: sep,
                row_prefix: pre,
                row_suffix: suf,
                ..Default::default()
            },
        );
    }
}

// ===========================================================================
// C8 — leading / consecutive / trailing newlines (blank lines)
// ===========================================================================
#[test]
fn c8_init_newline_runs() {
    let mut rng = Rng::new(8);
    let rowseps = ["\n", "\n\n", "\n\n\n"];
    let prefixes = ["", "\n", "\n\n"];
    let suffixes = ["", "\n", "\n\n\n"];
    for iter in 0..300u64 {
        let w = rng.range(1, 6) as usize;
        let h = rng.range(1, 6) as usize;
        let rs = *rng.pick(&rowseps);
        let pre = *rng.pick(&prefixes);
        let suf = *rng.pick(&suffixes);
        init_case(
            &format!("c8-{iter}-{w}x{h}"),
            &mut rng,
            w,
            h,
            false,
            TextOpts {
                row_sep: rs,
                prefix: pre,
                suffix: suf,
                ..Default::default()
            },
        );
    }
}

// ===========================================================================
// C9 — width 1 (no space delimiter at all), full i32 value range
// ===========================================================================
#[test]
fn c9_init_width_one_full_range() {
    let mut rng = Rng::new(9);
    for iter in 0..300u64 {
        let h = rng.range(1, 8) as usize;
        init_case(
            &format!("c9-{iter}-1x{h}"),
            &mut rng,
            1,
            h,
            true,
            TextOpts::default(),
        );
    }
    // explicit extremes
    for v in [i32::MIN, i32::MAX, -1, 0, 1, i32::MIN + 1, i32::MAX - 1] {
        let text = format!("{v}");
        run_pair(&format!("c9-extreme-{v}"), |api| {
            init_observe(api, &text, 1, 1)
        });
    }
}

// ===========================================================================
// C10 — height 1 (no newline delimiter), wide rows
// ===========================================================================
#[test]
fn c10_init_height_one() {
    let mut rng = Rng::new(10);
    for iter in 0..200u64 {
        let w = rng.range(1, 16) as usize;
        init_case(
            &format!("c10-{iter}-{w}x1"),
            &mut rng,
            w,
            1,
            false,
            TextOpts::default(),
        );
    }
}

// ===========================================================================
// C11 — degenerate zero dimensions that still succeed
// ===========================================================================
#[test]
fn c11_init_zero_dimensions() {
    let texts = ["", "1", "1 2", "1 2\n3 4", "\n", " ", "\n\n"];
    for t in texts {
        for (w, h) in [(0, 0), (0, 1), (1, 0), (0, 3), (3, 0), (0, 8), (8, 0)] {
            let tag = format!("c11-{}-{w}x{h}", show(t.as_bytes()));
            run_pair(&tag, |api| init_observe(api, t, w, h));
        }
    }
}

// ===========================================================================
// C12 — token forms handed to atoi
// ===========================================================================
#[test]
fn c12_init_atoi_token_forms() {
    let toks = [
        "abc",
        "12abc",
        "+7",
        "-0",
        "007",
        "0x10",
        "2147483647",
        "-2147483648",
        "99999999999999999999",
        "-99999999999999999999",
        "1e3",
        ".",
        "--3",
        "-",
        "+",
        "2147483648",
        "-2147483649",
        "4294967296",
        "0",
        "9,9",
        "\t5",
        "1.9",
    ];
    // one token per cell, width 1 so the buffer arithmetic stays safe
    for t in toks {
        let tag = format!("c12-single-{}", show(t.as_bytes()));
        let text = t.to_string();
        run_pair(&tag, |api| init_observe(api, &text, 1, 1));
    }
    // and mixed rows
    let mut rng = Rng::new(12);
    for iter in 0..100u64 {
        let w = rng.range(1, 5) as usize;
        let h = rng.range(1, 5) as usize;
        let mut rows: Vec<String> = Vec::new();
        for _ in 0..h {
            let cols: Vec<String> = (0..w).map(|_| rng.pick(&toks).to_string()).collect();
            rows.push(cols.join(" "));
        }
        let text = rows.join("\n");
        run_pair(&format!("c12-mixed-{iter}"), |api| {
            init_observe(api, &text, w as c_int, h as c_int)
        });
    }
}

// ---------------------------------------------------------------------------
// helper: build a matrix with known cells and stringify it
// ---------------------------------------------------------------------------
fn to_string_of(api: &Api, w: c_int, h: c_int, cells: &[c_int]) -> (StrObs, bool) {
    unsafe {
        let m = (api.allocate_matrix)(w, h);
        if m.is_null() {
            return (StrObs::Null, true);
        }
        fill(m, cells);
        let s = (api.matrix_to_string)(m);
        let o = observe_and_free_cstring(s);
        (api.free_matrix)(m);
        (o, false)
    }
}

// ===========================================================================
// C13 — matrix_to_string, width 1, full i32 (buffer exactly tight)
// ===========================================================================
#[test]
fn c13_to_string_width_one_full_range() {
    let mut rng = Rng::new(13);
    for iter in 0..300u64 {
        let h = rng.range(1, 8) as c_int;
        let cells: Vec<c_int> = (0..h).map(|_| rng.i32_full()).collect();
        run_pair(&format!("c13-{iter}-1x{h}"), |api| {
            to_string_of(api, 1, h, &cells)
        });
    }
    for v in [i32::MIN, i32::MAX, -1, 0, 1] {
        let cells = vec![v; 4];
        run_pair(&format!("c13-extreme-{v}"), |api| {
            to_string_of(api, 1, 4, &cells)
        });
    }
}

// ===========================================================================
// C14 — matrix_to_string, width >= 2, randomized safe values
// ===========================================================================
#[test]
fn c14_to_string_wide() {
    let mut rng = Rng::new(14);
    for iter in 0..300u64 {
        let w = rng.range(2, 12) as c_int;
        let h = rng.range(1, 12) as c_int;
        let cells: Vec<c_int> = (0..(w as i64 * h as i64)).map(|_| rng.safe_cell()).collect();
        run_pair(&format!("c14-{iter}-{w}x{h}"), |api| {
            to_string_of(api, w, h, &cells)
        });
    }
}

// ===========================================================================
// C15 — matrix_to_string with degenerate shapes
// ===========================================================================
#[test]
fn c15_to_string_degenerate() {
    for (w, h) in [(0, 0), (0, 1), (1, 0), (0, 5), (5, 0), (0, 64), (64, 0)] {
        run_pair(&format!("c15-{w}x{h}"), |api| to_string_of(api, w, h, &[]));
    }
}

// ===========================================================================
// C16 — digit-count boundaries and sign patterns
// ===========================================================================
#[test]
fn c16_to_string_digit_boundaries() {
    // 1, 2, 9 and 10 character decimal forms, positive and negative
    let groups: [&[c_int]; 8] = [
        &[0, 0, 0, 0],
        &[1, 2, 3, 4],
        &[-1, -2, -3, -4],
        &[9, 10, 99, 100],
        &[999_999_999, 100_000_000, 1, 0],
        &[-999_999_999, -100_000_000, -1, 0],
        &[123_456_789, -123_456_789, 987_654_321, -987_654_32],
        &[-9, 9, -99, 99],
    ];
    for (gi, g) in groups.iter().enumerate() {
        for (w, h) in [(1, 4), (2, 2), (4, 1)] {
            let cells: Vec<c_int> = g.to_vec();
            run_pair(&format!("c16-{gi}-{w}x{h}"), |api| {
                to_string_of(api, w, h, &cells)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// helper: multiply two matrices built from known cells
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn multiply_of(
    api: &Api,
    wa: c_int,
    ha: c_int,
    ca: &[c_int],
    wb: c_int,
    hb: c_int,
    cb: &[c_int],
) -> MatObs {
    unsafe {
        let a = (api.allocate_matrix)(wa, ha);
        let b = (api.allocate_matrix)(wb, hb);
        assert!(!a.is_null() && !b.is_null(), "setup allocation failed");
        fill(a, ca);
        fill(b, cb);
        let r = (api.multiply_matrices)(a, b);
        let o = observe(r);
        (api.free_matrix)(r);
        (api.free_matrix)(a);
        (api.free_matrix)(b);
        o
    }
}

// ===========================================================================
// C17 — multiply_matrices, randomized conformant shapes, no wrap
// ===========================================================================
#[test]
fn c17_multiply_random_small() {
    let mut rng = Rng::new(17);
    for iter in 0..400u64 {
        let ha = rng.range(1, 8) as c_int;
        let k = rng.range(1, 8) as c_int;
        let wb = rng.range(1, 8) as c_int;
        let ca: Vec<c_int> = (0..(ha as i64 * k as i64))
            .map(|_| rng.range(-32, 32) as c_int)
            .collect();
        let cb: Vec<c_int> = (0..(k as i64 * wb as i64))
            .map(|_| rng.range(-32, 32) as c_int)
            .collect();
        run_pair(&format!("c17-{iter}-{ha}x{k}*{k}x{wb}"), |api| {
            multiply_of(api, k, ha, &ca, wb, k, &cb)
        });
    }
}

// ===========================================================================
// C18 — inner dimension 0 and 1
// ===========================================================================
#[test]
fn c18_multiply_inner_zero_and_one() {
    let mut rng = Rng::new(18);
    for iter in 0..100u64 {
        let ha = rng.range(1, 6) as c_int;
        let wb = rng.range(1, 6) as c_int;
        // k == 0
        run_pair(&format!("c18-k0-{iter}-{ha}x0*0x{wb}"), |api| {
            multiply_of(api, 0, ha, &[], wb, 0, &[])
        });
        // k == 1
        let ca: Vec<c_int> = (0..ha).map(|_| rng.range(-1000, 1000) as c_int).collect();
        let cb: Vec<c_int> = (0..wb).map(|_| rng.range(-1000, 1000) as c_int).collect();
        run_pair(&format!("c18-k1-{iter}-{ha}x1*1x{wb}"), |api| {
            multiply_of(api, 1, ha, &ca, wb, 1, &cb)
        });
    }
}

// ===========================================================================
// C19 — zero output dimensions
// ===========================================================================
#[test]
fn c19_multiply_zero_output_dims() {
    let mut rng = Rng::new(19);
    for k in [0, 1, 3] {
        // mat_a->height == 0
        let cb: Vec<c_int> = (0..(k * 4)).map(|_| rng.safe_cell()).collect();
        run_pair(&format!("c19-h0-k{k}"), |api| {
            multiply_of(api, k, 0, &[], 4, k, &cb)
        });
        // mat_b->width == 0
        let ca: Vec<c_int> = (0..(4 * k)).map(|_| rng.safe_cell()).collect();
        run_pair(&format!("c19-w0-k{k}"), |api| {
            multiply_of(api, k, 4, &ca, 0, k, &[])
        });
        // both
        run_pair(&format!("c19-both-k{k}"), |api| {
            multiply_of(api, k, 0, &[], 0, k, &[])
        });
    }
}

// ===========================================================================
// C20 — int overflow / wraparound in the accumulator
// ===========================================================================
#[test]
fn c20_multiply_overflow_wrap() {
    let mut rng = Rng::new(20);
    for iter in 0..300u64 {
        let ha = 1i32;
        let k = rng.range(4, 12) as c_int;
        let wb = 1i32;
        let mag = *rng.pick(&[1i64 << 15, 1 << 16, 1 << 20, 1 << 30, i32::MAX as i64]);
        let ca: Vec<c_int> = (0..k).map(|_| rng.range(-mag, mag) as c_int).collect();
        let cb: Vec<c_int> = (0..k).map(|_| rng.range(-mag, mag) as c_int).collect();
        // width of the result is 1, so matrix_to_string is not involved here and
        // the raw wrapped cell value is compared directly.
        run_pair(&format!("c20-{iter}-k{k}-mag{mag}"), |api| {
            multiply_of(api, k, ha, &ca, wb, k, &cb)
        });
    }
    // deterministic extremes
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, -1),
        (-1, i32::MIN),
    ] {
        run_pair(&format!("c20-extreme-{a}x{b}"), |api| {
            multiply_of(api, 1, 1, &[a], 1, 1, &[b])
        });
    }
}

// ===========================================================================
// C21 — dot product (1xN * Nx1) and outer product (Nx1 * 1xN)
// ===========================================================================
#[test]
fn c21_multiply_dot_and_outer() {
    let mut rng = Rng::new(21);
    for iter in 0..150u64 {
        let n = rng.range(1, 20) as c_int;
        let ca: Vec<c_int> = (0..n).map(|_| rng.range(-1000, 1000) as c_int).collect();
        let cb: Vec<c_int> = (0..n).map(|_| rng.range(-1000, 1000) as c_int).collect();
        // 1xN * Nx1 -> 1x1
        run_pair(&format!("c21-dot-{iter}-n{n}"), |api| {
            multiply_of(api, n, 1, &ca, 1, n, &cb)
        });
        // Nx1 * 1xN -> NxN
        run_pair(&format!("c21-outer-{iter}-n{n}"), |api| {
            multiply_of(api, 1, n, &ca, n, 1, &cb)
        });
    }
}

// ---------------------------------------------------------------------------
// helper: write_to_file into a scratch path, return (rc, file bytes)
// ---------------------------------------------------------------------------
fn write_case(api: &Api, path: &str, content: &str) -> (c_int, Option<Vec<u8>>) {
    let _ = std::fs::remove_file(path);
    let p = cstr(path);
    let c = cstr(content);
    let rc = unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) };
    (rc, std::fs::read(path).ok())
}

// ===========================================================================
// C22 — new file, content smaller than BUFSIZ
// ===========================================================================
#[test]
fn c22_write_small_new_file() {
    let mut rng = Rng::new(22);
    let path = scratch("c22.txt").to_str().unwrap().to_string();
    for iter in 0..200u64 {
        let n = rng.range(1, 2000) as usize;
        let content: String = (0..n)
            .map(|_| (b'a' + (rng.next_u64() % 26) as u8) as char)
            .collect();
        run_pair(&format!("c22-{iter}-{n}"), |api| {
            write_case(api, &path, &content)
        });
    }
}

// ===========================================================================
// C23 — content around and beyond BUFSIZ
// ===========================================================================
#[test]
fn c23_write_large() {
    let path = scratch("c23.txt").to_str().unwrap().to_string();
    for n in [4095usize, 4096, 4097, 8192, 65536, 300_000] {
        let content: String = std::iter::repeat('Z').take(n).collect();
        run_pair(&format!("c23-{n}"), |api| write_case(api, &path, &content));
    }
}

// ===========================================================================
// C24 — truncation of a pre-existing longer file
// ===========================================================================
#[test]
fn c24_write_truncates() {
    let path = scratch("c24.txt").to_str().unwrap().to_string();
    for (old, new) in [
        (10_000usize, 3usize),
        (5000, 0),
        (100, 99),
        (1, 4096),
        (70_000, 12),
    ] {
        let existing: String = std::iter::repeat('O').take(old).collect();
        let content: String = std::iter::repeat('N').take(new).collect();
        let p = path.clone();
        let e = existing.clone();
        run_pair(&format!("c24-{old}->{new}"), move |api| {
            std::fs::write(&p, e.as_bytes()).unwrap();
            let cp = cstr(&p);
            let cc = cstr(&content);
            let rc = unsafe { (api.write_to_file)(cp.as_ptr(), cc.as_ptr()) };
            (rc, std::fs::read(&p).ok())
        });
    }
}

// ===========================================================================
// C25 — empty content
// ===========================================================================
#[test]
fn c25_write_empty_content() {
    let path = scratch("c25.txt").to_str().unwrap().to_string();
    run_pair("c25", |api| write_case(api, &path, ""));
}

// ===========================================================================
// C26 — awkward content bytes (newlines, tabs, high bytes, % formats)
// ===========================================================================
#[test]
fn c26_write_awkward_content() {
    let path = scratch("c26.txt").to_str().unwrap().to_string();
    let contents = [
        "line1\nline2\n",
        "\t\ttabbed\t",
        "100%%s %d %s %n %p",
        "%s%s%s%s%s%s%s%s%s%s",
        "héllo wörld — ünïcøde ✓",
        "\u{1}\u{2}\u{7f}\u{80}",
        "trailing newline\n",
        "\n",
        "a",
    ];
    for (i, c) in contents.iter().enumerate() {
        run_pair(&format!("c26-{i}"), |api| write_case(api, &path, c));
    }
}

// ===========================================================================
// C27 — /dev/null target
// ===========================================================================
#[test]
fn c27_write_dev_null() {
    for content in ["", "x", "some longer content\nwith newlines\n"] {
        run_pair(&format!("c27-{}", content.len()), |api| {
            let p = cstr("/dev/null");
            let c = cstr(content);
            unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) }
        });
    }
}

// ---------------------------------------------------------------------------
// helper: run driver() and capture the file it writes
// ---------------------------------------------------------------------------
const OUT_FILE: &str = "matrix.txt";

fn driver_case(
    api: &Api,
    wa: c_int,
    ha: c_int,
    ta: &str,
    wb: c_int,
    hb: c_int,
    tb: &str,
) -> (c_int, Option<Vec<u8>>) {
    let _ = std::fs::remove_file(OUT_FILE);
    let ca = cstr(ta);
    let cb = cstr(tb);
    let rc = unsafe { (api.driver)(wa, ha, ca.as_ptr(), wb, hb, cb.as_ptr()) };
    let bytes = std::fs::read(OUT_FILE).ok();
    let _ = std::fs::remove_file(OUT_FILE);
    (rc, bytes)
}

// ===========================================================================
// C28 — driver: randomized conformant pipelines
// ===========================================================================
#[test]
fn c28_driver_random() {
    let mut rng = Rng::new(28);
    for iter in 0..300u64 {
        let ha = rng.range(1, 8) as usize;
        let k = rng.range(1, 8) as usize;
        let wb = rng.range(1, 8) as usize;
        let ca = gen_cells(&mut rng, k, ha, false)
            .iter()
            .map(|r| {
                r.iter()
                    .map(|v| (v % 1000).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let cb = gen_cells(&mut rng, wb, k, false)
            .iter()
            .map(|r| {
                r.iter()
                    .map(|v| (v % 1000).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        run_pair(&format!("c28-{iter}-{ha}x{k}*{k}x{wb}"), |api| {
            driver_case(
                api,
                k as c_int,
                ha as c_int,
                &ca,
                wb as c_int,
                k as c_int,
                &cb,
            )
        });
    }
}

// ===========================================================================
// C29 — driver with degenerate but valid shapes
// ===========================================================================
#[test]
fn c29_driver_degenerate() {
    let cases: [(c_int, c_int, &str, c_int, c_int, &str); 6] = [
        (0, 0, "", 0, 0, ""),
        (0, 0, "1 2", 0, 0, "3 4"),
        (0, 3, "1\n2\n3", 4, 0, ""),
        (2, 0, "", 3, 2, "1 2 3\n4 5 6"),
        (0, 2, "1\n2", 0, 0, ""),
        (1, 0, "", 1, 1, "7"),
    ];
    for (i, (wa, ha, ta, wb, hb, tb)) in cases.iter().enumerate() {
        run_pair(&format!("c29-{i}"), |api| {
            driver_case(api, *wa, *ha, ta, *wb, *hb, tb)
        });
    }
}

// ===========================================================================
// C30 — driver 1x1, dot product and outer product shapes
// ===========================================================================
#[test]
fn c30_driver_vector_shapes() {
    let mut rng = Rng::new(30);
    for iter in 0..100u64 {
        let n = rng.range(1, 10) as usize;
        let col: String = (0..n)
            .map(|_| rng.range(-50, 50).to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let row: String = (0..n)
            .map(|_| rng.range(-50, 50).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        // 1xN * Nx1
        run_pair(&format!("c30-dot-{iter}-{n}"), |api| {
            driver_case(api, n as c_int, 1, &row, 1, n as c_int, &col)
        });
        // Nx1 * 1xN
        run_pair(&format!("c30-outer-{iter}-{n}"), |api| {
            driver_case(api, 1, n as c_int, &col, n as c_int, 1, &row)
        });
    }
    run_pair("c30-1x1", |api| driver_case(api, 1, 1, "6", 1, 1, "7"));
}

// ===========================================================================
// C31 — the pipeline composed manually from the low-level exports
// ===========================================================================
#[test]
fn c31_manual_pipeline() {
    let mut rng = Rng::new(31);
    let out = scratch("c31.txt").to_str().unwrap().to_string();
    for iter in 0..150u64 {
        let ha = rng.range(1, 6) as usize;
        let k = rng.range(1, 6) as usize;
        let wb = rng.range(1, 6) as usize;
        let ta = gen_cells(&mut rng, k, ha, false)
            .iter()
            .map(|r| {
                r.iter()
                    .map(|v| (v % 500).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let tb = gen_cells(&mut rng, wb, k, false)
            .iter()
            .map(|r| {
                r.iter()
                    .map(|v| (v % 500).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let outp = out.clone();
        run_pair(&format!("c31-{iter}"), move |api| unsafe {
            let _ = std::fs::remove_file(&outp);
            let sa = cstr(&ta);
            let sb = cstr(&tb);
            let a = (api.initialize_matrix_from_string)(sa.as_ptr(), k as c_int, ha as c_int);
            let b = (api.initialize_matrix_from_string)(sb.as_ptr(), wb as c_int, k as c_int);
            let oa = observe(a);
            let ob = observe(b);
            let r = (api.multiply_matrices)(a, b);
            let orr = observe(r);
            let s = (api.matrix_to_string)(r);
            let os = if s.is_null() {
                StrObs::Null
            } else {
                StrObs::Bytes(
                    std::ffi::CStr::from_ptr(s as *const c_char)
                        .to_bytes()
                        .to_vec(),
                )
            };
            let cp = cstr(&outp);
            let rc = if s.is_null() {
                -999
            } else {
                (api.write_to_file)(cp.as_ptr(), s)
            };
            let bytes = std::fs::read(&outp).ok();
            if !s.is_null() {
                libc::free(s as *mut libc::c_void);
            }
            (api.free_matrix)(r);
            (api.free_matrix)(a);
            (api.free_matrix)(b);
            (oa, ob, orr, os, rc, bytes)
        });
    }
}

// ===========================================================================
// C32 — larger stress shapes across all entry points
// ===========================================================================
#[test]
fn c32_stress_larger_shapes() {
    let mut rng = Rng::new(32);
    for iter in 0..40u64 {
        let ha = rng.range(1, 64) as c_int;
        let k = rng.range(1, 64) as c_int;
        let wb = rng.range(1, 64) as c_int;
        let ca: Vec<c_int> = (0..(ha as i64 * k as i64))
            .map(|_| rng.range(-256, 256) as c_int)
            .collect();
        let cb: Vec<c_int> = (0..(k as i64 * wb as i64))
            .map(|_| rng.range(-256, 256) as c_int)
            .collect();
        run_pair(&format!("c32-{iter}-{ha}x{k}*{k}x{wb}"), |api| unsafe {
            let a = (api.allocate_matrix)(k, ha);
            let b = (api.allocate_matrix)(wb, k);
            fill(a, &ca);
            fill(b, &cb);
            let r = (api.multiply_matrices)(a, b);
            let om = observe(r);
            let s = (api.matrix_to_string)(r);
            let os = observe_and_free_cstring(s);
            (api.free_matrix)(r);
            (api.free_matrix)(a);
            (api.free_matrix)(b);
            (om, os)
        });
    }
}
