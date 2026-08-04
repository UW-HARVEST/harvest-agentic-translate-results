use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libhdr_compare_lib.so");
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

#[test]
fn test_hdr_compare_exhaustive() {
    let c_lib = load_c_lib();
    let rs_lib = load_rust_lib();
    let c_fn: Symbol<HdrCompareFn> = unsafe { c_lib.get(b"hdr_compare").unwrap() };
    let rs_fn: Symbol<HdrCompareFn> = unsafe { rs_lib.get(b"hdr_compare").unwrap() };

    // Targeted byte values that exercise all branches in hdr_valid and hdr_compare.
    // h[0]: 0xff (valid) vs others (invalid)
    // h[1]: bits that toggle MPEG version, layer, etc.
    // h[2]: bits that toggle bitrate, sample rate fields
    let byte0_vals: &[u8] = &[0x00, 0x7f, 0xfe, 0xff];
    let byte1_vals: &[u8] = &[
        0x00, 0x02, 0xe2, 0xe3, 0xf0, 0xf1, 0xf2, 0xf4, 0xf6, 0xfa, 0xfe, 0xff,
    ];
    let byte2_vals: &[u8] = &[
        0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x1c, 0x30, 0x40, 0x80, 0xf0, 0xfc, 0xff,
    ];
    let byte3: u8 = 0x00; // h[3] is never read, but we need a 4th byte for safety

    let mut tested = 0u64;
    let mut mismatches = 0u64;

    for &b0 in byte0_vals {
        for &b1 in byte1_vals {
            for &b2 in byte2_vals {
                let h1 = [b0, b1, b2, byte3];
                for &b0b in byte0_vals {
                    for &b1b in byte1_vals {
                        for &b2b in byte2_vals {
                            let h2 = [b0b, b1b, b2b, byte3];
                            let c_result =
                                unsafe { c_fn(h1.as_ptr(), h2.as_ptr()) };
                            let rs_result =
                                unsafe { rs_fn(h1.as_ptr(), h2.as_ptr()) };
                            if c_result != rs_result {
                                mismatches += 1;
                                if mismatches <= 10 {
                                    eprintln!(
                                        "MISMATCH h1={:02x?} h2={:02x?}: C={} Rust={}",
                                        &h1[..3], &h2[..3], c_result, rs_result
                                    );
                                }
                            }
                            tested += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("Tested {} input pairs, {} mismatches", tested, mismatches);
    assert_eq!(mismatches, 0, "{} mismatches found out of {} tests", mismatches, tested);
}
