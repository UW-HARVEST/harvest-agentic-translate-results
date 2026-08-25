use call_predict_lib as _;
use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type CallPredict = unsafe extern "C" fn(c_int) -> c_int;

struct Api {
    _library: Library,
    call_predict: CallPredict,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe {
            Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
        };
        let call_predict = unsafe {
            *library
                .get::<CallPredict>(b"call_predict\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve call_predict from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            call_predict,
        }
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libtranslated_rust.so"),
        root.join("target/debug/deps/libcall_predict_lib.so"),
    )
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(
        c_path.is_file(),
        "C shared library is missing: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library is missing: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn compare(c: &Api, rust: &Api, pfcn: c_int) {
    let c_result = unsafe { (c.call_predict)(pfcn) };
    let rust_result = unsafe { (rust.call_predict)(pfcn) };
    assert_eq!(
        rust_result, c_result,
        "call_predict differed for pfcn={pfcn}"
    );
}

#[test]
fn configs_01_through_12_dedicated_predictors_match() {
    let (c, rust) = load_apis();
    let mut rng = XorShift64(0x4f1b_7c2a_d935_806e);
    let mut observations = [0_u16; 12];

    for _ in 0..4096 {
        let pfcn = (rng.next_u32() % 12) as c_int;
        compare(&c, &rust, pfcn);
        observations[pfcn as usize] += 1;
    }

    for (pfcn, count) in observations.into_iter().enumerate() {
        assert!(count > 256, "insufficient samples for pfcn={pfcn}: {count}");
    }
}

#[test]
fn config_13_default_branch_matches() {
    let (c, rust) = load_apis();
    let mut rng = XorShift64(0x8d62_ef90_13a7_c54b);

    for pfcn in [c_int::MIN, -1, 12, c_int::MAX] {
        compare(&c, &rust, pfcn);
    }

    let mut observations = 0;
    while observations < 4096 {
        let pfcn = rng.next_u32() as c_int;
        if !(0..=11).contains(&pfcn) {
            compare(&c, &rust, pfcn);
            observations += 1;
        }
    }
}
