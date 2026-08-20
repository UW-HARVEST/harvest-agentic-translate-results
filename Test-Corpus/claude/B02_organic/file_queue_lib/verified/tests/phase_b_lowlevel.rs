//! Phase B — CONFIGS.md rows 1–9: the lowest-level entry points
//! (`os_calloc`, `os_realloc`, `os_strdup`, `merror`) driven through the
//! `.so` exports of both implementations.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CString};

// ---------------------------------------------------------------------------
// Row 1 — os_calloc over randomized (num, size)
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_os_calloc_randomized() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0101_2024);
    for _ in 0..400 {
        let num = rng.below(64) + 1;
        let size = rng.below(64) + 1;
        unsafe {
            let pc = (c.os_calloc)(num as usize, size as usize);
            let pr = (r.os_calloc)(num as usize, size as usize);
            assert!(!pc.is_null() && !pr.is_null(), "os_calloc({num},{size}) NULL");
            let n = (num * size) as usize;
            let sc = std::slice::from_raw_parts(pc as *const u8, n);
            let sr = std::slice::from_raw_parts(pr as *const u8, n);
            assert!(sc.iter().all(|&b| b == 0), "C os_calloc not zeroed");
            assert_eq!(sc, sr, "os_calloc({num},{size}) contents differ");
            free(pc);
            free(pr);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — os_calloc with a zero product
// ---------------------------------------------------------------------------

#[test]
fn cfg_02_os_calloc_zero_product() {
    let (c, r) = apis();
    for (num, size) in [(0usize, 0usize), (0, 16), (16, 0), (0, 1), (1, 0)] {
        unsafe {
            let pc = (c.os_calloc)(num, size);
            let pr = (r.os_calloc)(num, size);
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "os_calloc({num},{size}) NULL-ness differs"
            );
            if !pc.is_null() {
                free(pc);
            }
            if !pr.is_null() {
                free(pr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — os_realloc(NULL, n)
// ---------------------------------------------------------------------------

#[test]
fn cfg_03_os_realloc_from_null() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0303_2024);
    for i in 0..300 {
        // include 0 explicitly: realloc(NULL, 0) is malloc(0) -> non-NULL
        let n = if i == 0 { 0 } else { rng.below(4096) as usize };
        unsafe {
            let pc = (c.os_realloc)(std::ptr::null_mut(), n);
            let pr = (r.os_realloc)(std::ptr::null_mut(), n);
            assert!(!pc.is_null(), "C os_realloc(NULL,{n}) returned NULL");
            assert!(!pr.is_null(), "Rust os_realloc(NULL,{n}) returned NULL");
            free(pc);
            free(pr);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 4 & 5 — os_realloc grow / shrink, prefix preservation
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_05_os_realloc_grow_and_shrink() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0405_2024);
    for _ in 0..300 {
        let old = 1 + rng.below(512) as usize;
        let grow = rng.below(2) == 0;
        let new = if grow {
            old + 1 + rng.below(512) as usize
        } else {
            1 + rng.below(old as u64) as usize
        };
        let pattern: Vec<u8> = (0..old).map(|_| rng.byte()).collect();
        unsafe {
            let mut ps: Vec<*mut c_void> = Vec::new();
            for api in [c, r] {
                let p = (api.os_realloc)(std::ptr::null_mut(), old);
                assert!(!p.is_null());
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), p as *mut u8, old);
                let q = (api.os_realloc)(p, new);
                assert!(!q.is_null(), "{} os_realloc({old}->{new}) NULL", api.name);
                ps.push(q);
            }
            let keep = old.min(new);
            let sc = std::slice::from_raw_parts(ps[0] as *const u8, keep);
            let sr = std::slice::from_raw_parts(ps[1] as *const u8, keep);
            assert_eq!(sc, &pattern[..keep], "C realloc lost the prefix");
            assert_eq!(sc, sr, "os_realloc({old}->{new}) prefix differs");
            for p in ps {
                free(p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — os_strdup over randomized byte strings
// ---------------------------------------------------------------------------

#[test]
fn cfg_06_os_strdup_randomized() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0606_2024);
    for i in 0..500 {
        // len 0 first, then random lengths up to 512, all non-NUL bytes
        let len = if i == 0 { 0 } else { rng.below(513) as usize };
        let mut s = rng.wild_nl(len);
        s.push(0);
        unsafe {
            let pc = (c.os_strdup)(s.as_ptr() as *const c_char);
            let pr = (r.os_strdup)(s.as_ptr() as *const c_char);
            assert!(!pc.is_null() && !pr.is_null());
            let lc = strlen(pc);
            let lr = strlen(pr);
            assert_eq!(lc, len, "C os_strdup length");
            assert_eq!(lc, lr, "os_strdup length differs");
            let bc = std::slice::from_raw_parts(pc as *const u8, lc + 1);
            let br = std::slice::from_raw_parts(pr as *const u8, lr + 1);
            assert_eq!(bc, br, "os_strdup contents differ");
            assert_ne!(pc as usize, s.as_ptr() as usize, "must be a fresh buffer");
            free(pc as *mut c_void);
            free(pr as *mut c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7, 8, 9 — merror with each of the two library templates and with an
// arbitrary caller-supplied template.
// ---------------------------------------------------------------------------

const FSEEK_ERROR: &str = "(1116): Could not set position in file '%s' due to [(%d)-(%s)].";
const FSTAT_ERROR: &str = "(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";

fn diff_merror(tmpl: &str, file_name: &[u8], err: c_int, err_msg: &[u8]) {
    let (c, r) = apis();
    let t = CString::new(tmpl).unwrap();
    let mut fname = file_name.to_vec();
    fname.push(0);
    let mut emsg = err_msg.to_vec();
    emsg.push(0);

    let out_c = capture_stderr(|| unsafe {
        (c.merror)(
            t.as_ptr(),
            fname.as_ptr() as *const c_char,
            err,
            emsg.as_ptr() as *const c_char,
        );
    });
    let out_r = capture_stderr(|| unsafe {
        (r.merror)(
            t.as_ptr(),
            fname.as_ptr() as *const c_char,
            err,
            emsg.as_ptr() as *const c_char,
        );
    });
    assert_eq!(
        out_c,
        out_r,
        "merror stderr differs\ntmpl={tmpl:?} file={:?} err={err} msg={:?}\nC   ={:?}\nRUST={:?}",
        String::from_utf8_lossy(file_name),
        String::from_utf8_lossy(err_msg),
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
    assert!(!out_c.is_empty(), "merror produced no output at all");
}

#[test]
fn cfg_07_merror_fseek_template() {
    let mut rng = Rng::new(0x0707_2024);
    for _ in 0..80 {
        let f = rng.token_len(0, 40);
        let m = rng.token_len(0, 40);
        let e = rng.i32_any();
        diff_merror(FSEEK_ERROR, &f, e, &m);
    }
}

#[test]
fn cfg_08_merror_fstat_template() {
    let mut rng = Rng::new(0x0808_2024);
    for _ in 0..80 {
        let f = rng.token_len(0, 40);
        let m = rng.token_len(0, 40);
        let e = rng.i32_any();
        diff_merror(FSTAT_ERROR, &f, e, &m);
    }
}

#[test]
fn cfg_09_merror_arbitrary_template() {
    let mut rng = Rng::new(0x0909_2024);
    let templates = [
        "%s|%d|%s",
        "no args at all",
        "%s",
        "[%d]",
        "%s %s",              // reads err as a pointer? no: 2nd %s takes `err`
        "trailing %s %d %s !", // exercised with empty strings below
        "%-20s/%+d/%s",
        "%.3s..%5d..%s",
    ];
    // Templates whose conversions do not match the (char*, int, char*) argument
    // list would be UB in *both* implementations in the same way, but "%s %s"
    // would print an int as a pointer, so restrict to well-typed ones plus the
    // no-arg case (which is well defined).
    for t in [templates[0], templates[1], templates[2], templates[3], templates[5], templates[6], templates[7]] {
        for _ in 0..30 {
            let f = rng.token_len(0, 30);
            let m = rng.token_len(0, 30);
            let e = match rng.below(4) {
                0 => 0,
                1 => -1,
                2 => i32::MIN,
                _ => rng.i32_any(),
            };
            diff_merror(t, &f, e, &m);
        }
    }
    // explicit empty-string edge
    diff_merror(FSEEK_ERROR, b"", 0, b"");
    diff_merror(FSTAT_ERROR, b"", i32::MAX, b"");
}
