//! Harness self-check: both `.so`s load, both export `helloworld`, and the
//! fd-level capture machinery actually observes the emitted bytes.

mod common;

use common::*;

#[test]
fn both_libraries_load_and_export_helloworld() {
    let c = c_lib_path();
    let r = rust_lib_path();
    assert!(c.exists(), "missing {}", c.display());
    assert!(r.exists(), "missing {}", r.display());
    // `resolve` panics if the symbol is absent from either `.so`.
    for w in Which::BOTH {
        let _f: HelloFn = resolve(w);
    }
}

#[test]
fn capture_machinery_sees_the_line() {
    for w in Which::BOTH {
        let run = run_captured(Sink::File, Buffering::Default, |_| unsafe { hello(w)() });
        assert_eq!(run.value, 0, "{} returned non-zero", w.name());
        assert_eq!(
            run.bytes,
            LINE,
            "{} emitted {:?}",
            w.name(),
            show(&run.bytes)
        );
    }
}
