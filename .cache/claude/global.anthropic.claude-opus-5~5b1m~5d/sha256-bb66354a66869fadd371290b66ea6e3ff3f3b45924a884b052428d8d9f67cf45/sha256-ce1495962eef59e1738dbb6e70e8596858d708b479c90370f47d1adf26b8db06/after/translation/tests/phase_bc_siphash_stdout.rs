//! `siphash` stdout differential — CONFIGS.md rows 29-35 and ERRORS.md rows 11-13.
//!
//! `siphash` has no return value; its ONLY observable is what it `printf`s to
//! the process's stdout. Capturing that means temporarily `dup2`-ing a file
//! over fd 1, which is a *process-wide* mutation: any other thread writing to
//! stdout during the capture window (including libtest's own
//! "test foo ... ok" progress lines) would be captured too and misread as
//! library output.
//!
//! Therefore this file deliberately contains exactly ONE `#[test]`. Cargo runs
//! test binaries one at a time, and with a single test there is no sibling
//! thread that could interleave. All rows are driven as sub-cases inside it and
//! reported individually on failure.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Reproduce one line of `siphash`'s output format exactly:
/// `printf("  { ")`, then 8x `printf("0x%02x, ")`, then `printf(" },\n")`.
fn expected_line(hash: usize) -> String {
    let mut s = String::from("  { ");
    for j in 0..8u32 {
        let byte = ((hash >> (j * 8)) & 255) as u8;
        s.push_str(&format!("0x{byte:02x}, "));
    }
    s.push_str(" },\n");
    s
}

struct Failures(Vec<String>);

impl Failures {
    fn check(&mut self, row: &str, init: c_int) {
        let c_out = capture_stdout(|| unsafe { (c_lib().siphash)(init) });
        let r_out = capture_stdout(|| unsafe { (rust_lib().siphash)(init) });

        if c_out != r_out {
            let c_s = String::from_utf8_lossy(&c_out);
            let r_s = String::from_utf8_lossy(&r_out);
            let detail = c_s
                .lines()
                .zip(r_s.lines())
                .enumerate()
                .find(|(_, (a, b))| a != b)
                .map(|(i, (a, b))| format!("line {i}: C={a:?} RUST={b:?}"))
                .unwrap_or_else(|| {
                    format!("length differs: C={} RUST={} bytes", c_out.len(), r_out.len())
                });
            self.0.push(format!("[{row}] siphash({init}): {detail}"));
            return;
        }

        // Structural sanity: exactly 64 lines, and the output is non-empty.
        let nl = c_out.iter().filter(|&&b| b == b'\n').count();
        if nl != 64 {
            self.0.push(format!("[{row}] siphash({init}): {nl} lines, expected 64"));
        }
        if c_out.is_empty() {
            self.0.push(format!("[{row}] siphash({init}): no output at all"));
        }
    }

    /// Row 35: the printed table must also match the table recomputed from
    /// direct low-level `stbds_hash_bytes` calls on both libraries.
    fn check_composed(&mut self, row: &str, init: c_int) {
        let mut mem = [0u8; 64];
        let mut z: i32 = init;
        for i in 0..64usize {
            mem[i] = z as u8;
            z = z.wrapping_add(1);
        }

        let mut expected = String::new();
        for len in 0..64usize {
            let mut cb = mem;
            let mut rb = mem;
            let hc = unsafe { (c_lib().hash_bytes)(cb.as_mut_ptr() as *mut c_void, len, 0) };
            let hr = unsafe { (rust_lib().hash_bytes)(rb.as_mut_ptr() as *mut c_void, len, 0) };
            if hc != hr {
                self.0.push(format!(
                    "[{row}] low-level divergence init={init} len={len}: C={hc:#018x} RUST={hr:#018x}"
                ));
                return;
            }
            expected.push_str(&expected_line(hc));
        }

        let c_out = capture_stdout(|| unsafe { (c_lib().siphash)(init) });
        let r_out = capture_stdout(|| unsafe { (rust_lib().siphash)(init) });
        let c_s = String::from_utf8_lossy(&c_out).to_string();
        let r_s = String::from_utf8_lossy(&r_out).to_string();

        if c_s != r_s {
            self.0.push(format!("[{row}] printed table differs for init={init}"));
        }
        if c_s != expected {
            self.0.push(format!(
                "[{row}] printed table != table recomputed from low-level \
                 stbds_hash_bytes calls (init={init})"
            ));
        }
    }
}

#[test]
fn siphash_stdout_differential_all_rows() {
    let mut f = Failures(Vec::new());

    // --- CONFIGS row 29: the canonical stb_ds invocation ------------------
    f.check("cfg29 init=0", 0);

    // --- CONFIGS row 30: small positive inits -----------------------------
    for init in [1, 2, 7, 63, 64, 127] {
        f.check("cfg30 small-positive", init);
    }

    // --- CONFIGS row 31 / ERRORS row 13: high-bit crossing ----------------
    for init in [0x80 - 63, 0x80 - 8, 0x80 - 4, 0x80 - 1, 0x80, 0x81, 0xC0, 0xFF, 0x100] {
        f.check("cfg31 high-bit-crossing", init);
    }
    // Every crossing position 0..63 within mem[].
    for k in 0..64i32 {
        f.check("err13 crossing-position", 0x80 - k);
    }

    // --- CONFIGS row 32: negative inits (unsigned char truncation) --------
    for init in [-1, -2, -8, -64, -128, -255, -256, -1000] {
        f.check("cfg32 negative", init);
    }
    for k in 0..16i32 {
        f.check("err13 negative-crossing", -k);
    }

    // --- CONFIGS row 33 / ERRORS row 12: int extremes, z++ overflow -------
    for init in [i32::MAX, i32::MAX - 1, i32::MAX - 63, i32::MIN, i32::MIN + 1] {
        f.check("cfg33 int-extremes", init);
    }

    // --- ERRORS row 12: full int-range boundary probes across FFI ---------
    // (`init` is a plain `int`, not an enum, so the whole range is in-domain;
    //  these are the values an out-of-range-enum probe would use.)
    for init in [
        0i32,
        -1,
        1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -(1 << 30),
        1 << 30,
        0x7fff_ff00,
        -0x7fff_ff00,
        255,
        256,
        -255,
        -256,
        65535,
        -65536,
    ] {
        f.check("err12 ffi-int-boundary", init);
    }

    // --- ERRORS row 11: no error path; always 64 lines --------------------
    // (the line-count assertion inside `check` covers this for every init
    //  above; these are the explicit representatives)
    for init in [0i32, 1, -1, i32::MAX, i32::MIN] {
        f.check("err11 no-error-path", init);
    }

    // --- CONFIGS row 34: randomized full-range inits ----------------------
    let mut rng = Rng::new(PRNG_SEED ^ 34);
    for _ in 0..64 {
        f.check("cfg34 random", rng.next_u64() as i32);
    }

    // --- CONFIGS row 35: composed pipeline vs low-level API ---------------
    for init in [0i32, 1, 0x80, -1, i32::MAX, i32::MIN, 12345] {
        f.check_composed("cfg35 composed", init);
    }

    assert!(
        f.0.is_empty(),
        "{} siphash stdout sub-case(s) diverged:\n  {}",
        f.0.len(),
        f.0.join("\n  ")
    );
}
