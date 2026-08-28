//! Randomised differential fuzzing of the whole public API.
//!
//! Deterministic (fixed-seed xorshift) so failures are reproducible.

mod common;

use common::*;
use std::ffi::c_int;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// A value whose decimal form is at most 10 characters, so
    /// `matrix_to_string` stays inside the buffer the C code allocates.
    fn narrow_value(&mut self) -> c_int {
        let magnitude = match self.below(6) {
            0 => 1,
            1 => 10,
            2 => 1000,
            3 => 100_000,
            4 => 10_000_000,
            _ => 999_999_999,
        };
        let v = (self.next() % (magnitude as u64 + 1)) as i64;
        if self.below(2) == 0 { -(v as c_int) } else { v as c_int }
    }
    /// An arbitrary token for the parser, including non-numeric junk.
    fn token(&mut self) -> String {
        match self.below(10) {
            0 => "abc".to_string(),
            1 => "".to_string(),
            2 => format!("{}x", self.below(100)),
            3 => format!("+{}", self.below(1000)),
            4 => "2147483647".to_string(),
            5 => "-2147483648".to_string(),
            6 => "99999999999999999999".to_string(),
            7 => "-99999999999999999999".to_string(),
            8 => format!("00{}", self.below(1000)),
            _ => format!("{}", self.narrow_value()),
        }
    }
}

/// Builds a row-major text matrix from random tokens.
fn random_input(rng: &mut Rng, w: c_int, h: c_int) -> String {
    let mut s = String::new();
    for _ in 0..h.max(0) {
        let cols = w.max(0);
        let parts: Vec<String> = (0..cols).map(|_| rng.token()).collect();
        s.push_str(&parts.join(" "));
        s.push('\n');
    }
    s
}

#[test]
fn fuzz_initialize_matrix_from_string() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    for iter in 0..1500 {
        let w = rng.below(7) as c_int;
        let h = rng.below(7) as c_int;
        // Sometimes provide fewer rows/cols than requested to hit error paths.
        let gen_w = if rng.below(4) == 0 { w.saturating_sub(1) } else { w };
        let gen_h = if rng.below(4) == 0 { h.saturating_sub(1) } else { h };
        let input = random_input(&mut rng, gen_w, gen_h);
        let s = cstr(&input);
        unsafe {
            let cm = (p.c.initialize_matrix_from_string)(s.as_ptr(), w, h);
            let rm = (p.rs.initialize_matrix_from_string)(s.as_ptr(), w, h);
            assert_eq!(
                cm.is_null(),
                rm.is_null(),
                "iter {iter}: init({input:?},{w},{h}) nullness differs"
            );
            assert_eq!(
                snapshot(cm, true),
                snapshot(rm, true),
                "iter {iter}: init({input:?},{w},{h}) contents differ"
            );
            (p.c.free_matrix)(cm);
            (p.rs.free_matrix)(rm);
        }
    }
}

#[test]
fn fuzz_multiply_and_to_string() {
    let p = pair();
    let mut rng = Rng::new(0xDEADBEEF_9999);
    for iter in 0..1200 {
        let ha = 1 + rng.below(5) as c_int;
        let wa = 1 + rng.below(5) as c_int;
        // Usually make the shapes compatible; occasionally not.
        let hb = if rng.below(5) == 0 {
            1 + rng.below(5) as c_int
        } else {
            wa
        };
        let wb = 1 + rng.below(5) as c_int;

        let a: Vec<c_int> = (0..(wa * ha)).map(|_| rng.narrow_value()).collect();
        let b: Vec<c_int> = (0..(wb * hb)).map(|_| rng.narrow_value()).collect();

        unsafe {
            let ca = make_matrix(&p.c, wa, ha, &a);
            let cb = make_matrix(&p.c, wb, hb, &b);
            let ra = make_matrix(&p.rs, wa, ha, &a);
            let rb = make_matrix(&p.rs, wb, hb, &b);

            let cres = (p.c.multiply_matrices)(ca, cb);
            let rres = (p.rs.multiply_matrices)(ra, rb);
            assert_eq!(
                cres.is_null(),
                rres.is_null(),
                "iter {iter}: multiply nullness differs ({wa}x{ha} * {wb}x{hb})"
            );
            assert_eq!(
                snapshot(cres, true),
                snapshot(rres, true),
                "iter {iter}: multiply results differ ({wa}x{ha} * {wb}x{hb})\na={a:?}\nb={b:?}"
            );

            if !cres.is_null() {
                // Only stringify when the C size estimate provably suffices.
                let vals = snapshot(cres, true).unwrap();
                let sum: usize = vals
                    .cells
                    .iter()
                    .flatten()
                    .map(|v| v.to_string().len())
                    .sum();
                let limit = 10 * (vals.width as usize) * (vals.height as usize)
                    + vals.height as usize;
                if sum <= limit {
                    let cs = take_cstring((p.c.matrix_to_string)(cres));
                    let rs = take_cstring((p.rs.matrix_to_string)(rres));
                    assert_eq!(
                        cs.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                        rs.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                        "iter {iter}: matrix_to_string differs"
                    );
                }
            }

            (p.c.free_matrix)(cres);
            (p.rs.free_matrix)(rres);
            (p.c.free_matrix)(ca);
            (p.c.free_matrix)(cb);
            (p.rs.free_matrix)(ra);
            (p.rs.free_matrix)(rb);
        }
    }
}

#[test]
fn fuzz_driver_end_to_end() {
    let p = pair();
    let _g = fs_lock();
    let root = std::env::temp_dir().join(format!("driver_fuzz_{}", std::process::id()));
    let cdir = root.join("c");
    let rdir = root.join("r");
    std::fs::create_dir_all(&cdir).unwrap();
    std::fs::create_dir_all(&rdir).unwrap();
    let prev = std::env::current_dir().unwrap();

    let mut rng = Rng::new(0xABCDEF_5555);
    for iter in 0..400 {
        let ha = rng.below(5) as c_int;
        let wa = rng.below(5) as c_int;
        let hb = if rng.below(4) == 0 {
            rng.below(5) as c_int
        } else {
            wa
        };
        let wb = rng.below(5) as c_int;

        // Keep magnitudes small so the product stays inside 10 characters and
        // the C buffer estimate holds.
        let mk = |rng: &mut Rng, w: c_int, h: c_int| -> String {
            let mut s = String::new();
            for _ in 0..h {
                let parts: Vec<String> =
                    (0..w).map(|_| format!("{}", rng.below(2001) as i64 - 1000)).collect();
                s.push_str(&parts.join(" "));
                s.push('\n');
            }
            s
        };
        let a = mk(&mut rng, wa, ha);
        let b = mk(&mut rng, wb, hb);
        let sa = cstr(&a);
        let sb = cstr(&b);

        unsafe {
            std::env::set_current_dir(&cdir).unwrap();
            let _ = std::fs::remove_file("matrix.txt");
            let crc = (p.c.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr());
            let cout = std::fs::read("matrix.txt").ok();

            std::env::set_current_dir(&rdir).unwrap();
            let _ = std::fs::remove_file("matrix.txt");
            let rrc = (p.rs.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr());
            let rout = std::fs::read("matrix.txt").ok();

            std::env::set_current_dir(&prev).unwrap();

            assert_eq!(
                crc, rrc,
                "iter {iter}: driver rc differs ({wa}x{ha} * {wb}x{hb})\na={a:?}\nb={b:?}"
            );
            assert_eq!(
                cout.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                rout.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
                "iter {iter}: driver output differs ({wa}x{ha} * {wb}x{hb})\na={a:?}\nb={b:?}"
            );
        }
    }
}

#[test]
fn fuzz_write_to_file() {
    let p = pair();
    let _g = fs_lock();
    let d = std::env::temp_dir().join(format!("driver_fuzz_w_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();

    let mut rng = Rng::new(0x1357_9BDF);
    for iter in 0..200 {
        let len = rng.below(4096) as usize;
        let content: String = (0..len)
            .map(|_| (32u8 + (rng.below(95) as u8)) as char)
            .collect();
        let cpath = d.join("c.txt");
        let rpath = d.join("r.txt");
        let cn = cstr(cpath.to_str().unwrap());
        let rn = cstr(rpath.to_str().unwrap());
        let payload = cstr(&content);
        unsafe {
            let crc = (p.c.write_to_file)(cn.as_ptr(), payload.as_ptr());
            let rrc = (p.rs.write_to_file)(rn.as_ptr(), payload.as_ptr());
            assert_eq!(crc, rrc, "iter {iter}: rc differs");
            assert_eq!(
                std::fs::read(&cpath).ok(),
                std::fs::read(&rpath).ok(),
                "iter {iter}: file contents differ"
            );
        }
    }
}
