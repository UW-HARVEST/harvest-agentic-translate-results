//! Level 3 — the printing helpers `helper_ptr`, `helper_call`, `use_generated`.
//!
//! Each one writes to `stdout` *and* returns a value, so both halves are
//! compared: the return value and the exact bytes reaching file descriptor 1.
//!
//! `harness = false` (see `Cargo.toml`): the checks below temporarily point fd 1
//! at a file, and libtest's parallel progress output would otherwise land in the
//! captured bytes. Everything here runs sequentially and reports itself.

mod common;

use common::{Impl, REPEAT, accum_inputs, capture_stdout, operand_pairs, show};
use std::ffi::c_int;

/// `helper_ptr` prints `helper.ptr=%d`.
fn helper_ptr_matches(c: &Impl, r: &Impl) -> Result<(), String> {
    let (cf, rf) = (c.fn2("helper_ptr"), r.fn2("helper_ptr"));
    for (a, b) in operand_pairs() {
        let (cv, cout) = capture_stdout(|| cf(a, b));
        let (rv, rout) = capture_stdout(|| rf(a, b));
        if cv != rv {
            return Err(format!("helper_ptr({a}, {b}) returned C={cv} Rust={rv}"));
        }
        if cout != rout {
            return Err(format!(
                "helper_ptr({a}, {b}) stdout C={} Rust={}",
                show(&cout),
                show(&rout)
            ));
        }
    }
    Ok(())
}

/// `helper_call` prints `helper.call=%d helper.acc=%d` and returns `r + acc`,
/// where `acc` is the unrolled `REP<REPEAT>` chain.
fn helper_call_matches(c: &Impl, r: &Impl) -> Result<(), String> {
    let (cf, rf) = (c.fn2("helper_call"), r.fn2("helper_call"));
    for (a, b) in operand_pairs() {
        let (cv, cout) = capture_stdout(|| cf(a, b));
        let (rv, rout) = capture_stdout(|| rf(a, b));
        if cv != rv {
            return Err(format!("helper_call({a}, {b}) returned C={cv} Rust={rv}"));
        }
        if cout != rout {
            return Err(format!(
                "helper_call({a}, {b}) stdout C={} Rust={}",
                show(&cout),
                show(&rout)
            ));
        }
    }
    Ok(())
}

/// `use_generated` prints `gen.acc=%d`. Its `n` reaches `DISPATCH_REP`, whose
/// `switch` only handles `0..=6`; every other value hits `default:` and leaves
/// the accumulator at `INIT_FOR(OP)`.
fn use_generated_matches(c: &Impl, r: &Impl) -> Result<(), String> {
    let (cf, rf) = (c.fn1("use_generated"), r.fn1("use_generated"));
    let mut ns = accum_inputs();
    ns.extend(-40..=40);
    ns.extend([i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX, 7, 8, 1 << 30]);
    for n in ns {
        let (cv, cout) = capture_stdout(|| cf(n));
        let (rv, rout) = capture_stdout(|| rf(n));
        if cv != rv {
            return Err(format!("use_generated({n}) returned C={cv} Rust={rv}"));
        }
        if cout != rout {
            return Err(format!(
                "use_generated({n}) stdout C={} Rust={}",
                show(&cout),
                show(&rout)
            ));
        }
    }
    Ok(())
}

/// The whole `mdcore.c` surface driven in the order `mdmain.c` uses it, so the
/// combined stdout stream (not just per-call output) is compared too.
fn mdmain_call_sequence_matches(c: &Impl, r: &Impl) -> Result<(), String> {
    let run = |i: &Impl, a: c_int, b: c_int| {
        let hc = i.fn2("helper_call");
        let hp = i.fn2("helper_ptr");
        let ug = i.fn1("use_generated");
        let g = i.g_op();
        capture_stdout(move || {
            let x1 = hc(a, b);
            let x2 = hp(a, b);
            let x3 = ug(REPEAT);
            let gv = g(a, b);
            (x1, x2, x3, gv)
        })
    };
    for (a, b) in operand_pairs() {
        let (cv, cout) = run(c, a, b);
        let (rv, rout) = run(r, a, b);
        if cv != rv {
            return Err(format!("call sequence ({a}, {b}) returned C={cv:?} Rust={rv:?}"));
        }
        if cout != rout {
            return Err(format!(
                "call sequence ({a}, {b}) stdout C={} Rust={}",
                show(&cout),
                show(&rout)
            ));
        }
    }
    Ok(())
}

type Check = (&'static str, fn(&Impl, &Impl) -> Result<(), String>);

fn main() {
    let (c, r) = Impl::pair();
    let checks: [Check; 4] = [
        ("helper_ptr_matches", helper_ptr_matches),
        ("helper_call_matches", helper_call_matches),
        ("use_generated_matches", use_generated_matches),
        ("mdmain_call_sequence_matches", mdmain_call_sequence_matches),
    ];

    println!("running {} tests ({})", checks.len(), common::config_name());
    let mut failed = 0;
    for (name, f) in checks {
        match f(&c, &r) {
            Ok(()) => println!("test {name} ... ok"),
            Err(e) => {
                failed += 1;
                println!("test {name} ... FAILED\n    {e}");
            }
        }
    }
    println!(
        "test result: {}. {} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" },
        checks.len() - failed
    );
    if failed != 0 {
        std::process::exit(1);
    }
}
