//! Throwaway diagnostic: outcome histogram + per-case timing for the random
//! corpus, so the suite's runtime profile is understood rather than guessed.

mod harness;

use core::ffi::c_void;
use harness::{pair, run, Rng, OFF_IN, OFF_OUT};

#[test]
#[ignore]
fn histogram() {
    let p = pair();
    let sh = &p.shared;
    let mut rng = Rng::new(0xE247_0000_0000_0007);
    let mut hist = std::collections::BTreeMap::<String, usize>::new();
    let t0 = std::time::Instant::now();
    let n_cases = 300;
    for i in 0..n_cases {
        let n = 1 + rng.below(40);
        let mut input = rng.bytes(n);
        if i % 3 == 0 {
            input[0] = (input[0] & !0x06) | ((rng.below(3) as u8) << 1);
        }
        let align = i % 4;
        let out_bytes = [0i32, 1, 17, 4096][i % 4];
        for lib in [&p.c, &p.rust] {
            sh.fill_pattern();
            sh.write(OFF_IN + align, &input);
            let o = run(sh, lib, (OFF_OUT, 64), |l| unsafe {
                (l.cp_inflate)(
                    sh.in_ptr(align) as *mut c_void,
                    n as i32,
                    sh.out_ptr(0) as *mut c_void,
                    out_bytes,
                )
            });
            let key = if o.completed {
                format!("{} ret={}", lib.name, o.ret)
            } else {
                format!("{} signal={}", lib.name, harness::signame(o.signal))
            };
            *hist.entry(key).or_default() += 1;
        }
    }
    let el = t0.elapsed();
    eprintln!("--- {n_cases} cases, {:?} total, {:?}/case ---", el, el / n_cases as u32);
    for (k, v) in &hist {
        eprintln!("{k:30} {v}");
    }
}

/// How long does a *trivial* fork round-trip take? Isolates harness overhead
/// from library behaviour.
#[test]
#[ignore]
fn fork_overhead() {
    let p = pair();
    let sh = &p.shared;
    let t0 = std::time::Instant::now();
    let n = 200;
    for _ in 0..n {
        sh.fill_pattern();
        let _ = run(sh, &p.c, (OFF_OUT, 64), |l| unsafe {
            (l.unfilter)(4, 1, 3, sh.out_ptr(0))
        });
    }
    let el = t0.elapsed();
    eprintln!("fork round-trip: {:?} total, {:?} each", el, el / n);
}
