use libloading::Library;
use std::ffi::c_int;
use std::path::PathBuf;

type CallPredict = unsafe extern "C" fn(c_int) -> c_int;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_call_predict: CallPredict,
    rust_call_predict: CallPredict,
}

impl Apis {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libharvest-work-7iUyaQ.so");
        let rust_path = manifest_dir.join("target/release/libcall_predict_lib.so");

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c_call_predict = *c_library
                .get::<CallPredict>(b"call_predict\0")
                .expect("load C call_predict");
            let rust_call_predict = *rust_library
                .get::<CallPredict>(b"call_predict\0")
                .expect("load Rust call_predict");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_call_predict,
                rust_call_predict,
            }
        }
    }

    fn compare(&self, pfcn: c_int) -> (c_int, c_int) {
        unsafe { ((self.c_call_predict)(pfcn), (self.rust_call_predict)(pfcn)) }
    }
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn all_valid_configurations_match_through_shared_libraries() {
    let apis = Apis::load();
    let mut inputs = Vec::with_capacity(12 * 512);
    for pfcn in 0..=11 {
        inputs.extend(std::iter::repeat_n(pfcn, 512));
    }

    let mut seed = 0x434f_4e46_4947_5331_u64;
    for index in (1..inputs.len()).rev() {
        let swap_with = next_random(&mut seed) as usize % (index + 1);
        inputs.swap(index, swap_with);
    }

    for pfcn in inputs {
        let (c_result, rust_result) = apis.compare(pfcn);
        assert_eq!(c_result, 1, "unexpected C result for pfcn={pfcn}");
        assert_eq!(
            rust_result, c_result,
            "differential mismatch for valid pfcn={pfcn}"
        );
    }
}

#[test]
fn out_of_range_values_match_exact_rejection() {
    let apis = Apis::load();
    let boundaries = [c_int::MIN, -1, 12, c_int::MAX];

    for pfcn in boundaries {
        let (c_result, rust_result) = apis.compare(pfcn);
        assert_eq!(c_result, 0, "unexpected C result for pfcn={pfcn}");
        assert_eq!(
            rust_result, c_result,
            "differential mismatch at boundary pfcn={pfcn}"
        );
    }

    let mut seed = 0x4552_524f_5253_3031_u64;
    let mut tested = 0;
    while tested < 8_192 {
        let pfcn = next_random(&mut seed) as c_int;
        if (0..=11).contains(&pfcn) {
            continue;
        }

        let (c_result, rust_result) = apis.compare(pfcn);
        assert_eq!(c_result, 0, "unexpected C result for pfcn={pfcn}");
        assert_eq!(
            rust_result, c_result,
            "differential mismatch for rejected pfcn={pfcn}"
        );
        tested += 1;
    }
}
