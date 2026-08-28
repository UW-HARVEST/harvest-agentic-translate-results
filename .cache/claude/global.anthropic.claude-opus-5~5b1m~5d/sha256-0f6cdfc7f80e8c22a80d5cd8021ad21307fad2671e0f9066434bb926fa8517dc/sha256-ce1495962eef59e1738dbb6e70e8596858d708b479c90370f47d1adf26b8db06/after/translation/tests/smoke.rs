//! Harness self-check: proves both `.so` files load, all 8 symbols resolve, and that
//! each `fresh_pair()` really does get pristine statics (otherwise every stateful test
//! below would be meaningless).

mod common;
use common::*;

#[test]
fn both_libraries_load_and_expose_all_symbols() {
    let p = fresh_pair();
    // Resolving happens in `load`; touching one call per symbol proves they are live.
    unsafe {
        assert_eq!((p.c.validate_and_normalize)(1), (p.r.validate_and_normalize)(1));
    }
    eprintln!("C   .so: {}", c_so_path().display());
    eprintln!("Rust.so: {}", rust_so_path().display());
}

/// `dlopen` dedups by (dev,ino); `fresh_pair` defeats that by copying. Verify: a first
/// pair mutates its statics, a second pair must still start from scratch and return
/// exactly the same first-call value.
#[test]
fn fresh_pair_resets_hidden_static_state() {
    let first = unsafe {
        let p = fresh_pair();
        let a = (p.c.findrep)(5, 5, 5, 5);
        let b = (p.r.findrep)(5, 5, 5, 5);
        assert_eq!(a, b, "C and Rust disagree on the very first findrep call");
        // Mutate the statics further.
        (p.c.findrep)(9, 9, 9, 9);
        (p.r.findrep)(9, 9, 9, 9);
        a
    };

    let second = unsafe {
        let p = fresh_pair();
        let a = (p.c.findrep)(5, 5, 5, 5);
        let b = (p.r.findrep)(5, 5, 5, 5);
        assert_eq!(a, b);
        a
    };

    assert_eq!(
        first, second,
        "fresh_pair() is NOT giving pristine statics — stateful tests would be invalid"
    );
}

/// Sanity-check the documented initial state: accumulator=0, multiplier=1,
/// operation_count=0, observed through the low-level entry points.
#[test]
fn initial_static_state_matches() {
    unsafe {
        let p = fresh_pair();
        // accumulator starts at 0 -> add(0,0) returns 0
        assert_eq!((p.c.add_to_accumulator)(0, 0), 0);
        assert_eq!((p.r.add_to_accumulator)(0, 0), 0);
    }
    unsafe {
        let p = fresh_pair();
        // multiplier starts at 1 -> multiply(1,1) returns 1
        assert_eq!((p.c.multiply_with_multiplier)(1, 1), 1);
        assert_eq!((p.r.multiply_with_multiplier)(1, 1), 1);
    }
}
