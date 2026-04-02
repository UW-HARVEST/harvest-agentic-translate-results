use libloading::{Library, Symbol};
use next_double_lib::{cn_rnd_t, next_double};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

#[test]
fn test_next_double_matches_c() {
    let lib = unsafe { Library::new(C_LIB_PATH) }.expect("Failed to load C library");
    let c_next_double: Symbol<unsafe extern "C" fn(*mut cn_rnd_t) -> f64> =
        unsafe { lib.get(b"next_double") }.expect("Failed to find next_double");

    // Test with multiple initial states
    let seeds: &[[u64; 2]] = &[
        [0, 0],
        [1, 2],
        [0xdeadbeef, 0xcafebabe],
        [u64::MAX, u64::MAX],
        [1, 0],
        [0, 1],
        [0x123456789abcdef0, 0x0fedcba987654321],
    ];

    for seed in seeds {
        let mut c_state = cn_rnd_t { state: *seed };
        let mut rs_state = cn_rnd_t { state: *seed };

        // Call multiple times to test state evolution
        for i in 0..100 {
            let c_val = unsafe { c_next_double(&mut c_state) };
            let rs_val = next_double(&mut rs_state);

            assert_eq!(
                c_val.to_bits(),
                rs_val.to_bits(),
                "Mismatch at iteration {i} for seed {seed:?}: C={c_val} Rust={rs_val}"
            );
            assert_eq!(
                c_state.state, rs_state.state,
                "State mismatch at iteration {i} for seed {seed:?}"
            );
        }
    }
}
