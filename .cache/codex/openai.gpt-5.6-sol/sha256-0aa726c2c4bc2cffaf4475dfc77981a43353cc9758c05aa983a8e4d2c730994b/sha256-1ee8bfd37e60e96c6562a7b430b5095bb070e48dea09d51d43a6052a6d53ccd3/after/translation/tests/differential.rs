use libloading::Library;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const RANDOM_CASES: usize = 256;

type PackFn = unsafe extern "C" fn(*mut u8, u64);
type AddSampleFn = unsafe extern "C" fn(*mut TflacMd5, u32, u64);
type UpdateFn = unsafe extern "C" fn(*mut Tflac, *const i32) -> u32;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
struct TflacMd5 {
    pos: u32,
    total: u64,
    buffer: [u8; 72],
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
struct Tflac {
    md5_ctx: TflacMd5,
    cur_blocksize: u32,
    channels: u32,
}

struct Api {
    _library: Library,
    pack: PackFn,
    add_sample: AddSampleFn,
    update: UpdateFn,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let pack = unsafe {
            *library
                .get::<PackFn>(b"tflac_pack_u64le\0")
                .expect("missing tflac_pack_u64le")
        };
        let add_sample = unsafe {
            *library
                .get::<AddSampleFn>(b"tflac_md5_addsample\0")
                .expect("missing tflac_md5_addsample")
        };
        let update = unsafe {
            *library
                .get::<UpdateFn>(b"update_md5\0")
                .expect("missing update_md5")
        };
        Self {
            _library: library,
            pack,
            add_sample,
            update,
        }
    }
}

struct ApiPair {
    c: Api,
    rust: Api,
}

impl ApiPair {
    fn load() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn seeded(config: &str) -> Self {
        let mut seed = 0xcbf2_9ce4_8422_2325_u64;
        for byte in config.bytes() {
            seed ^= u64::from(byte);
            seed = seed.wrapping_mul(0x1000_0000_01b3);
        }
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libupdate_md5_lib.so")
}

fn random_md5(rng: &mut Rng, pos: u32, total: u64) -> TflacMd5 {
    let mut buffer = [0_u8; 72];
    for byte in &mut buffer {
        *byte = rng.u32() as u8;
    }
    TflacMd5 { pos, total, buffer }
}

fn random_samples(rng: &mut Rng) -> [i32; 136] {
    let mut samples = [0_i32; 136];
    for sample in &mut samples {
        *sample = rng.i32();
    }
    samples
}

fn compare_add_sample(
    apis: &ApiPair,
    config: &str,
    iteration: usize,
    initial: TflacMd5,
    bits: u32,
    value: u64,
) {
    let mut c_state = initial.clone();
    let mut rust_state = initial;
    unsafe {
        (apis.c.add_sample)(&mut c_state, bits, value);
        (apis.rust.add_sample)(&mut rust_state, bits, value);
    }
    assert_eq!(
        c_state, rust_state,
        "{config} diverged at iteration {iteration}, bits={bits}, value={value:#018x}"
    );
}

fn run_add_cases<F>(config: &str, mut generate: F)
where
    F: FnMut(&mut Rng, usize) -> (TflacMd5, u32, u64),
{
    let apis = ApiPair::load();
    let mut rng = Rng::seeded(config);
    for iteration in 0..RANDOM_CASES {
        let (state, bits, value) = generate(&mut rng, iteration);
        compare_add_sample(&apis, config, iteration, state, bits, value);
    }
}

#[derive(Clone, Copy)]
enum PositionPath {
    NoWrap,
    WrapZero,
    WrapOne,
    WrapMany,
}

#[derive(Clone, Copy)]
enum ReturnPath {
    Normal,
    Underflow,
    MultiplyOverflow,
}

fn position_for(rng: &mut Rng, path: PositionPath) -> u32 {
    match path {
        PositionPath::NoWrap => rng.u32() % 24,
        PositionPath::WrapZero => [24, 32, 40, 48, 56][rng.u32() as usize % 5],
        PositionPath::WrapOne => [25, 33, 41, 49, 57][rng.u32() as usize % 5],
        PositionPath::WrapMany => 26 + rng.u32() % 7,
    }
}

fn return_inputs(rng: &mut Rng, path: ReturnPath) -> (u32, u32) {
    match path {
        ReturnPath::Normal => (5 + rng.u32() % 10_000, 8),
        ReturnPath::Underflow => {
            if rng.u32() & 3 == 0 {
                (rng.u32(), 0)
            } else {
                (rng.u32() % 40, 1)
            }
        }
        ReturnPath::MultiplyOverflow => (0x8000_0000 + rng.u32() % 0x4000_0000, 2),
    }
}

fn run_update_cases(
    config: &str,
    position_path: PositionPath,
    return_path: ReturnPath,
    overflow_total: bool,
) {
    let apis = ApiPair::load();
    let mut rng = Rng::seeded(config);
    for iteration in 0..RANDOM_CASES {
        let pos = position_for(&mut rng, position_path);
        let total = if overflow_total {
            u64::MAX - u64::from(rng.u32() % 320)
        } else {
            rng.u64() & 0x3fff_ffff_ffff_ffff
        };
        let (cur_blocksize, channels) = return_inputs(&mut rng, return_path);
        let md5_ctx = random_md5(&mut rng, pos, total);
        let initial = Tflac {
            md5_ctx,
            cur_blocksize,
            channels,
        };
        let samples = random_samples(&mut rng);
        let mut c_state = initial.clone();
        let mut rust_state = initial;
        let c_result = unsafe { (apis.c.update)(&mut c_state, samples.as_ptr()) };
        let rust_result = unsafe { (apis.rust.update)(&mut rust_state, samples.as_ptr()) };
        assert_eq!(
            c_result, rust_result,
            "{config} return diverged at iteration {iteration}"
        );
        assert_eq!(
            c_state, rust_state,
            "{config} state diverged at iteration {iteration}"
        );
    }
}

#[test]
fn c01_pack_u64le_full_range() {
    let apis = ApiPair::load();
    let mut rng = Rng::seeded("C1");
    for iteration in 0..RANDOM_CASES {
        let value = match iteration {
            0 => 0,
            1 => u64::MAX,
            _ => rng.u64(),
        };
        let mut c_bytes = [0xa5_u8; 16];
        let mut rust_bytes = c_bytes;
        unsafe {
            (apis.c.pack)(c_bytes.as_mut_ptr().add(4), value);
            (apis.rust.pack)(rust_bytes.as_mut_ptr().add(4), value);
        }
        assert_eq!(c_bytes, rust_bytes, "C1 diverged for {value:#018x}");
    }
}

#[test]
fn c02_add_no_wrap_byte_aligned() {
    run_add_cases("C2", |rng, iteration| {
        let bytes = if iteration == 0 { 0 } else { rng.u32() % 8 };
        let pos = rng.u32() % (64 - bytes);
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), bytes * 8, value)
    });
}

#[test]
fn c03_add_no_wrap_non_byte_aligned() {
    run_add_cases("C3", |rng, _| {
        let bytes = rng.u32() % 8;
        let pos = rng.u32() % (64 - bytes);
        let bits = bytes * 8 + 1 + rng.u32() % 7;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), bits, value)
    });
}

#[test]
fn c04_add_wrap_zero() {
    run_add_cases("C4", |rng, _| {
        let pos = rng.u32() % 64;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), (64 - pos) * 8, value)
    });
}

#[test]
fn c05_add_wrap_one() {
    run_add_cases("C5", |rng, _| {
        let pos = rng.u32() % 64;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), (65 - pos) * 8, value)
    });
}

#[test]
fn c06_add_wrap_many() {
    run_add_cases("C6", |rng, _| {
        let pos = rng.u32() % 64;
        let wrapped = 2 + rng.u32() % 7;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), (64 + wrapped - pos) * 8, value)
    });
}

#[test]
fn c07_add_position_overflow_below_64() {
    run_add_cases("C7", |rng, _| {
        let wrapped = rng.u32() % 16;
        let bytes = wrapped + 1 + rng.u32() % 16;
        let pos = u32::MAX - bytes + 1 + wrapped;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), bytes * 8, value)
    });
}

#[test]
fn c08_add_total_overflow() {
    run_add_cases("C8", |rng, _| {
        let bits = 1 + rng.u32() % 63;
        let total = u64::MAX - u64::from(rng.u32() % bits);
        let pos = rng.u32() % 8;
        let value = rng.u64();
        (random_md5(rng, pos, total), bits, value)
    });
}

#[test]
fn c09_add_tail_spill_boundary() {
    run_add_cases("C9", |rng, _| {
        let pos = 56 + rng.u32() % 8;
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, pos, total), 64, value)
    });
}

macro_rules! update_test {
    ($name:ident, $id:literal, $position:ident, $return_path:ident) => {
        #[test]
        fn $name() {
            run_update_cases(
                $id,
                PositionPath::$position,
                ReturnPath::$return_path,
                false,
            );
        }
    };
}

update_test!(c10_update_no_wrap_normal, "C10", NoWrap, Normal);
update_test!(c11_update_wrap_zero_normal, "C11", WrapZero, Normal);
update_test!(c12_update_wrap_one_normal, "C12", WrapOne, Normal);
update_test!(c13_update_wrap_many_normal, "C13", WrapMany, Normal);
update_test!(c14_update_no_wrap_underflow, "C14", NoWrap, Underflow);
update_test!(c15_update_wrap_zero_underflow, "C15", WrapZero, Underflow);
update_test!(c16_update_wrap_one_underflow, "C16", WrapOne, Underflow);
update_test!(c17_update_wrap_many_underflow, "C17", WrapMany, Underflow);
update_test!(
    c18_update_no_wrap_multiply_overflow,
    "C18",
    NoWrap,
    MultiplyOverflow
);
update_test!(
    c19_update_wrap_zero_multiply_overflow,
    "C19",
    WrapZero,
    MultiplyOverflow
);
update_test!(
    c20_update_wrap_one_multiply_overflow,
    "C20",
    WrapOne,
    MultiplyOverflow
);
update_test!(
    c21_update_wrap_many_multiply_overflow,
    "C21",
    WrapMany,
    MultiplyOverflow
);

#[test]
fn c22_update_total_overflow() {
    run_update_cases("C22", PositionPath::NoWrap, ReturnPath::Normal, true);
}

#[test]
fn c23_add_maximum_bits() {
    run_add_cases("C23", |rng, _| {
        let total = rng.u64() & 0x3fff_ffff_ffff_ffff;
        let value = rng.u64();
        (random_md5(rng, 1, total), u32::MAX, value)
    });
}

fn run_crash_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("ffi_crash_child")
        .arg("--nocapture")
        .env("FFI_CRASH_LIBRARY", library)
        .env("FFI_CRASH_CASE", case)
        .status()
        .unwrap_or_else(|error| panic!("failed to run crash child: {error}"))
}

#[cfg(unix)]
fn assert_same_crash(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_crash_child(&c_library_path(), case);
    let rust_status = run_crash_child(&rust_library_path(), case);
    assert!(!c_status.success(), "{case}: C unexpectedly succeeded");
    assert!(
        !rust_status.success(),
        "{case}: Rust unexpectedly succeeded"
    );
    assert!(
        c_status.signal().is_some(),
        "{case}: C did not terminate from a Unix signal"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "{case}: process termination signal differs"
    );
}

#[test]
fn g01_pack_null_destination() {
    assert_same_crash("pack_null");
}

#[test]
fn g02_add_null_context() {
    assert_same_crash("add_null");
}

#[test]
fn g03_update_null_context() {
    assert_same_crash("update_context_null");
}

#[test]
fn g04_update_null_samples() {
    assert_same_crash("update_samples_null");
}

#[test]
fn ffi_crash_child() {
    let Ok(case) = env::var("FFI_CRASH_CASE") else {
        return;
    };
    let library =
        PathBuf::from(env::var_os("FFI_CRASH_LIBRARY").expect("FFI_CRASH_LIBRARY must be set"));
    let api = Api::load(&library);
    unsafe {
        match case.as_str() {
            "pack_null" => (api.pack)(std::ptr::null_mut(), 0),
            "add_null" => (api.add_sample)(std::ptr::null_mut(), 64, 0),
            "update_context_null" => {
                (api.update)(std::ptr::null_mut(), std::ptr::null());
            }
            "update_samples_null" => {
                let mut state = Tflac {
                    md5_ctx: TflacMd5 {
                        pos: 0,
                        total: 0,
                        buffer: [0; 72],
                    },
                    cur_blocksize: 40,
                    channels: 1,
                };
                (api.update)(&mut state, std::ptr::null());
            }
            _ => panic!("unknown crash case {case}"),
        };
    }
}
