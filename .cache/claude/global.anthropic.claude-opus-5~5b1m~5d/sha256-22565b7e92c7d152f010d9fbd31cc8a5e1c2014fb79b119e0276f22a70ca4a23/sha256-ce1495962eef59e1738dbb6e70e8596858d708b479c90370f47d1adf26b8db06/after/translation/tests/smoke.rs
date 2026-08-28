// Harness smoke test + Phase D symbol-parity check performed at runtime.

mod common;

use common::*;

#[test]
fn smoke_both_libraries_load_and_expose_all_five_symbols() {
    let _g = lock();
    let (c, r) = both();
    println!("C   .so: {}", c.path.display());
    println!("Rust.so: {}", r.path.display());
    // Loading already asserted every one of the five symbols resolves in both
    // shared objects; reaching here means symbol parity holds at the dlsym
    // level for the full exported surface of the C library.
}

#[test]
fn smoke_envy_agrees_on_a_clean_environment() {
    let _g = lock();
    diff("smoke/envy(1,2,3,4)", |api| unsafe { (api.envy)(1, 2, 3, 4) });
}

/// Probe used while bringing the harness up: dump the four `struct ConfigFlags`
/// bytes produced by each implementation for a couple of environments, so a
/// layout mismatch shows up as data rather than as a mysterious value diff.
#[test]
fn smoke_config_flags_layout_probe() {
    let _g = lock();
    let (c, r) = both();
    for (label, env) in [
        ("all-unset", vec![]),
        (
            "verbose=1,debug=1,optimize=x",
            vec![
                ("PROG_VERBOSE", Some("1")),
                ("PROG_DEBUG", Some("1")),
                ("PROG_OPTIMIZE", Some("x")),
            ],
        ),
    ] {
        let mut fc = Flags4([0xAA, 0xBB, 0xCC, 0xDD]);
        env_config(&env);
        let _ = capture(|| {
            unsafe { (c.init_config_from_env)(fc.as_mut_ptr()) };
            0
        });
        let mut fr = Flags4([0xAA, 0xBB, 0xCC, 0xDD]);
        env_config(&env);
        let _ = capture(|| {
            unsafe { (r.init_config_from_env)(fr.as_mut_ptr()) };
            0
        });
        println!("{label}: C={:02x?} Rust={:02x?}", fc.0, fr.0);
        assert_eq!(fc, fr, "layout mismatch for {label}");
    }
}
