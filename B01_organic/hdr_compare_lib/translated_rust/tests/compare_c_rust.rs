use hdr_compare_lib::hdr_compare;
use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn call_c_hdr_compare(lib: &Library, h1: &[u8; 4], h2: &[u8; 4]) -> c_int {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const u8, *const u8) -> c_int> =
            lib.get(b"hdr_compare").unwrap();
        func(h1.as_ptr(), h2.as_ptr())
    }
}

fn call_rust_hdr_compare(h1: &[u8; 4], h2: &[u8; 4]) -> c_int {
    hdr_compare(h1.as_ptr(), h2.as_ptr())
}

#[test]
fn test_hdr_compare_exhaustive() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };

    let h0_vals: &[u8] = &[0x00, 0x7f, 0xfe, 0xff];
    let h1_vals: &[u8] = &[
        0x00, 0x02, 0x04, 0x06, 0xe2, 0xe3, 0xf0, 0xf2, 0xf4, 0xf6, 0xf8, 0xfa, 0xfb, 0xfc,
        0xfe, 0xff,
    ];
    let h2_vals: &[u8] = &[
        0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x30, 0x40, 0x80, 0x90, 0xb0, 0xf0, 0xfc, 0xff,
    ];

    let mut tested = 0u64;
    let mut mismatches = Vec::new();

    for &a0 in h0_vals {
        for &a1 in h1_vals {
            for &a2 in h2_vals {
                for &b0 in h0_vals {
                    for &b1 in h1_vals {
                        for &b2 in h2_vals {
                            let h1 = [a0, a1, a2, 0];
                            let h2 = [b0, b1, b2, 0];
                            let c_result = call_c_hdr_compare(&lib, &h1, &h2);
                            let r_result = call_rust_hdr_compare(&h1, &h2);
                            if c_result != r_result {
                                mismatches.push((h1, h2, c_result, r_result));
                                if mismatches.len() >= 20 {
                                    break;
                                }
                            }
                            tested += 1;
                        }
                        if mismatches.len() >= 20 { break; }
                    }
                    if mismatches.len() >= 20 { break; }
                }
                if mismatches.len() >= 20 { break; }
            }
            if mismatches.len() >= 20 { break; }
        }
        if mismatches.len() >= 20 { break; }
    }

    if !mismatches.is_empty() {
        for (h1, h2, c, r) in &mismatches {
            eprintln!(
                "MISMATCH: h1=[{:#04x},{:#04x},{:#04x}] h2=[{:#04x},{:#04x},{:#04x}] C={} Rust={}",
                h1[0], h1[1], h1[2], h2[0], h2[1], h2[2], c, r
            );
        }
        panic!(
            "{} mismatches found out of {} tests",
            mismatches.len(), tested
        );
    }

    eprintln!("All {} test cases passed", tested);
}
