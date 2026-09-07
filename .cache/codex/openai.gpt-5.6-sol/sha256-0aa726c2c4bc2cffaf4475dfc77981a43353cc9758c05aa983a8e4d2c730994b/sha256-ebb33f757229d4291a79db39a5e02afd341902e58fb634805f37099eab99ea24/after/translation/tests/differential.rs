use libloading::Library;
use std::env;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct TflacMd5 {
    pos: u32,
    total: u64,
    buffer: [u8; 72],
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tflac {
    md5_ctx: TflacMd5,
    cur_blocksize: u32,
    channels: u32,
}

#[repr(C, align(8))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Md5Storage([u8; size_of::<TflacMd5>()]);

impl Md5Storage {
    fn as_mut_ptr(&mut self) -> *mut TflacMd5 {
        self.0.as_mut_ptr().cast()
    }

    fn set_total(&mut self, total: u64) {
        unsafe {
            std::ptr::addr_of_mut!((*self.as_mut_ptr()).total).write(total);
        }
    }
}

#[repr(C, align(8))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct TflacStorage([u8; size_of::<Tflac>()]);

impl TflacStorage {
    fn as_mut_ptr(&mut self) -> *mut Tflac {
        self.0.as_mut_ptr().cast()
    }
}

type PackU64Le = unsafe extern "C" fn(*mut u8, u64);
type Md5AddSample = unsafe extern "C" fn(*mut TflacMd5, u32, u64);
type UpdateMd5 = unsafe extern "C" fn(*mut Tflac, *const i32) -> u32;

struct Api {
    _library: Library,
    pack_u64le: PackU64Le,
    md5_addsample: Md5AddSample,
    update_md5: UpdateMd5,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let pack_u64le = unsafe {
            *library
                .get::<PackU64Le>(b"tflac_pack_u64le\0")
                .expect("missing tflac_pack_u64le")
        };
        let md5_addsample = unsafe {
            *library
                .get::<Md5AddSample>(b"tflac_md5_addsample\0")
                .expect("missing tflac_md5_addsample")
        };
        let update_md5 = unsafe {
            *library
                .get::<UpdateMd5>(b"update_md5\0")
                .expect("missing update_md5")
        };
        Self {
            _library: library,
            pack_u64le,
            md5_addsample,
            update_md5,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C library not found at {}; build it first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust release library not found at {}; build it first",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root()
        .join("../c_src/build")
        .join("libharvest-work-wBzseN.so")
}

fn rust_library_path() -> PathBuf {
    crate_root()
        .join("target/release")
        .join("libupdate_md5_lib.so")
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.u64() as usize) % upper_exclusive
    }
}

fn random_md5(rng: &mut Rng, pos: u32) -> Md5Storage {
    let mut storage = Md5Storage([0; size_of::<TflacMd5>()]);
    for byte in &mut storage.0 {
        *byte = rng.u64() as u8;
    }
    unsafe {
        let state = storage.as_mut_ptr();
        std::ptr::addr_of_mut!((*state).pos).write(pos);
        std::ptr::addr_of_mut!((*state).total).write(rng.u64());
    }
    storage
}

fn random_tflac(
    rng: &mut Rng,
    pos: u32,
    total: u64,
    cur_blocksize: u32,
    channels: u32,
) -> TflacStorage {
    let mut storage = TflacStorage([0; size_of::<Tflac>()]);
    for byte in &mut storage.0 {
        *byte = rng.u64() as u8;
    }
    unsafe {
        let state = storage.as_mut_ptr();
        std::ptr::addr_of_mut!((*state).md5_ctx.pos).write(pos);
        std::ptr::addr_of_mut!((*state).md5_ctx.total).write(total);
        std::ptr::addr_of_mut!((*state).cur_blocksize).write(cur_blocksize);
        std::ptr::addr_of_mut!((*state).channels).write(channels);
    }
    storage
}

fn assert_add_matches(
    pair: &Pair,
    row: usize,
    iteration: usize,
    initial: Md5Storage,
    bits: u32,
    value: u64,
) {
    let mut c_state = initial.clone();
    let mut rust_state = initial;
    unsafe {
        (pair.c.md5_addsample)(c_state.as_mut_ptr(), bits, value);
        (pair.rust.md5_addsample)(rust_state.as_mut_ptr(), bits, value);
    }
    assert_eq!(
        c_state, rust_state,
        "CONFIGS.md row {row}, randomized iteration {iteration}, bits={bits}, value={value:#x}"
    );
}

fn product_fields(mode: ProductMode, rng: &mut Rng) -> (u32, u32) {
    match mode {
        ProductMode::GreaterThan40 => (41 + rng.u32() % 10_000, 1),
        ProductMode::Equal40 => {
            const PAIRS: [(u32, u32); 8] = [
                (40, 1),
                (20, 2),
                (10, 4),
                (8, 5),
                (5, 8),
                (4, 10),
                (2, 20),
                (1, 40),
            ];
            PAIRS[rng.usize(PAIRS.len())]
        }
        ProductMode::Below40 => {
            const PAIRS: [(u32, u32); 8] = [
                (0, 0),
                (0, 19),
                (23, 0),
                (1, 1),
                (3, 7),
                (5, 7),
                (13, 3),
                (39, 1),
            ];
            PAIRS[rng.usize(PAIRS.len())]
        }
        ProductMode::MultiplicationWraps => {
            let channels = 2 + rng.u32() % 7;
            let threshold = u32::MAX / channels;
            let room = u32::MAX - threshold;
            (threshold + 1 + rng.u32() % room, channels)
        }
    }
}

#[derive(Clone, Copy)]
enum ProductMode {
    GreaterThan40,
    Equal40,
    Below40,
    MultiplicationWraps,
}

#[derive(Clone, Copy)]
enum PositionMode {
    NoWrap,
    ExactFinal,
    PartialFinal,
    ExactBeforeFinal,
    PartialBeforeFinal,
    NonNormalized,
    NearU32Max,
}

fn initial_position(mode: PositionMode, rng: &mut Rng) -> u32 {
    match mode {
        PositionMode::NoWrap => rng.u32() % 24,
        PositionMode::ExactFinal => 24,
        PositionMode::PartialFinal => 25 + rng.u32() % 7,
        PositionMode::ExactBeforeFinal => {
            const POSITIONS: [u32; 4] = [32, 40, 48, 56];
            POSITIONS[rng.usize(POSITIONS.len())]
        }
        PositionMode::PartialBeforeFinal => {
            const POSITIONS: [u32; 24] = [
                33, 34, 35, 36, 37, 38, 39, 41, 42, 43, 44, 45, 46, 47, 49, 50, 51, 52, 53, 54, 55,
                57, 58, 59,
            ];
            // Include the remaining positions 60..63 without making the table unwieldy.
            if rng.u32() & 3 == 0 {
                60 + rng.u32() % 4
            } else {
                POSITIONS[rng.usize(POSITIONS.len())]
            }
        }
        PositionMode::NonNormalized => {
            let multiple = 1 + rng.u32() % 1_000_000;
            multiple * 64 + 56 + rng.u32() % 8
        }
        PositionMode::NearU32Max => u32::MAX - rng.u32() % 8,
    }
}

fn random_samples(rng: &mut Rng) -> [i32; 136] {
    let mut samples = [0_i32; 136];
    for sample in &mut samples {
        *sample = rng.u32() as i32;
    }
    samples
}

fn assert_update_matches(
    pair: &Pair,
    row: usize,
    iteration: usize,
    position_mode: PositionMode,
    product_mode: ProductMode,
    rng: &mut Rng,
) {
    let pos = initial_position(position_mode, rng);
    let (cur_blocksize, channels) = product_fields(product_mode, rng);
    let total = if iteration % 16 == 0 {
        u64::MAX - (rng.u64() % 320)
    } else {
        rng.u64()
    };
    let initial = random_tflac(rng, pos, total, cur_blocksize, channels);
    let samples = random_samples(rng);
    let mut c_state = initial.clone();
    let mut rust_state = initial;
    let c_result;
    let rust_result;
    unsafe {
        c_result = (pair.c.update_md5)(c_state.as_mut_ptr(), samples.as_ptr());
        rust_result = (pair.rust.update_md5)(rust_state.as_mut_ptr(), samples.as_ptr());
    }
    assert_eq!(
        c_result, rust_result,
        "CONFIGS.md row {row}, iteration {iteration}: return mismatch"
    );
    assert_eq!(
        c_state, rust_state,
        "CONFIGS.md row {row}, iteration {iteration}: state mismatch"
    );
}

#[test]
fn config_01_pack_u64le() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
    let mut values = vec![0, 1, u64::MAX, u64::MAX - 1, 0x0123_4567_89ab_cdef];
    values.extend((0..1_024).map(|_| rng.u64()));
    for (iteration, value) in values.into_iter().enumerate() {
        let mut c_bytes = [0xa5; 8];
        let mut rust_bytes = [0xa5; 8];
        unsafe {
            (pair.c.pack_u64le)(c_bytes.as_mut_ptr(), value);
            (pair.rust.pack_u64le)(rust_bytes.as_mut_ptr(), value);
        }
        assert_eq!(
            c_bytes, rust_bytes,
            "CONFIGS.md row 1, iteration {iteration}, value={value:#x}"
        );
    }
}

#[test]
fn configs_02_through_11_md5_addsample() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for iteration in 0..512 {
        let value = rng.u64();

        let pos = rng.u32() % 64;
        assert_add_matches(&pair, 2, iteration, random_md5(&mut rng, pos), 0, value);

        let pos = rng.u32() % 64;
        assert_add_matches(
            &pair,
            3,
            iteration,
            random_md5(&mut rng, pos),
            1 + rng.u32() % 7,
            value,
        );

        let bytes = 1 + rng.u32() % 8;
        let pos = rng.u32() % (64 - bytes);
        assert_add_matches(
            &pair,
            4,
            iteration,
            random_md5(&mut rng, pos),
            bytes * 8,
            value,
        );

        let bytes = 1 + rng.u32() % 8;
        let pos = rng.u32() % (64 - bytes);
        assert_add_matches(
            &pair,
            5,
            iteration,
            random_md5(&mut rng, pos),
            bytes * 8 + 1 + rng.u32() % 7,
            value,
        );

        let bytes = 1 + rng.u32() % 8;
        assert_add_matches(
            &pair,
            6,
            iteration,
            random_md5(&mut rng, 64 - bytes),
            bytes * 8,
            value,
        );

        let remainder = 1 + rng.u32() % 7;
        assert_add_matches(
            &pair,
            7,
            iteration,
            random_md5(&mut rng, 56 + remainder),
            64,
            value,
        );

        let bytes = 9 + rng.u32() % 64;
        assert_add_matches(
            &pair,
            8,
            iteration,
            random_md5(&mut rng, 72 - bytes),
            bytes * 8,
            value,
        );

        let multiple = 1 + rng.u32() % 1_000_000;
        let low = rng.u32() % 64;
        let target = rng.u32() % 9;
        let bytes = (target + 64 - low) % 64;
        assert_add_matches(
            &pair,
            9,
            iteration,
            random_md5(&mut rng, multiple * 64 + low),
            bytes * 8,
            value,
        );

        let distance_from_max = rng.u32() % 8;
        let final_pos = rng.u32() % 9;
        let bytes = distance_from_max + 1 + final_pos;
        assert_add_matches(
            &pair,
            10,
            iteration,
            random_md5(&mut rng, u32::MAX - distance_from_max),
            bytes * 8,
            value,
        );

        let total_wrap_pos = rng.u32() % 56;
        let mut total_wrap = random_md5(&mut rng, total_wrap_pos);
        total_wrap.set_total(u64::MAX - rng.u64() % 64);
        assert_add_matches(&pair, 11, iteration, total_wrap, 64, value);
    }
}

#[test]
fn configs_12_through_39_update_md5() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    let configurations = [
        (12, PositionMode::NoWrap, ProductMode::GreaterThan40),
        (13, PositionMode::NoWrap, ProductMode::Equal40),
        (14, PositionMode::NoWrap, ProductMode::Below40),
        (15, PositionMode::NoWrap, ProductMode::MultiplicationWraps),
        (16, PositionMode::ExactFinal, ProductMode::GreaterThan40),
        (17, PositionMode::ExactFinal, ProductMode::Equal40),
        (18, PositionMode::ExactFinal, ProductMode::Below40),
        (
            19,
            PositionMode::ExactFinal,
            ProductMode::MultiplicationWraps,
        ),
        (20, PositionMode::PartialFinal, ProductMode::GreaterThan40),
        (21, PositionMode::PartialFinal, ProductMode::Equal40),
        (22, PositionMode::PartialFinal, ProductMode::Below40),
        (
            23,
            PositionMode::PartialFinal,
            ProductMode::MultiplicationWraps,
        ),
        (
            24,
            PositionMode::ExactBeforeFinal,
            ProductMode::GreaterThan40,
        ),
        (25, PositionMode::ExactBeforeFinal, ProductMode::Equal40),
        (26, PositionMode::ExactBeforeFinal, ProductMode::Below40),
        (
            27,
            PositionMode::ExactBeforeFinal,
            ProductMode::MultiplicationWraps,
        ),
        (
            28,
            PositionMode::PartialBeforeFinal,
            ProductMode::GreaterThan40,
        ),
        (29, PositionMode::PartialBeforeFinal, ProductMode::Equal40),
        (30, PositionMode::PartialBeforeFinal, ProductMode::Below40),
        (
            31,
            PositionMode::PartialBeforeFinal,
            ProductMode::MultiplicationWraps,
        ),
        (32, PositionMode::NonNormalized, ProductMode::GreaterThan40),
        (33, PositionMode::NonNormalized, ProductMode::Equal40),
        (34, PositionMode::NonNormalized, ProductMode::Below40),
        (
            35,
            PositionMode::NonNormalized,
            ProductMode::MultiplicationWraps,
        ),
        (36, PositionMode::NearU32Max, ProductMode::GreaterThan40),
        (37, PositionMode::NearU32Max, ProductMode::Equal40),
        (38, PositionMode::NearU32Max, ProductMode::Below40),
        (
            39,
            PositionMode::NearU32Max,
            ProductMode::MultiplicationWraps,
        ),
    ];

    for (row, position_mode, product_mode) in configurations {
        for iteration in 0..256 {
            assert_update_matches(&pair, row, iteration, position_mode, product_mode, &mut rng);
        }
    }
}

fn child_status(library: &str, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_crash_worker")
        .arg("--nocapture")
        .env("TFLAC_CRASH_LIBRARY", library)
        .env("TFLAC_CRASH_CASE", case)
        .status()
        .unwrap_or_else(|error| panic!("failed to run crash worker: {error}"))
}

#[test]
fn generic_null_boundaries_match() {
    for (row, case) in [
        ("G1", "pack_null"),
        ("G2", "add_null"),
        ("G3", "update_null_t"),
        ("G4", "update_null_samples"),
    ] {
        let c_status = child_status("c", case);
        let rust_status = child_status("rust", case);
        assert!(
            !c_status.success(),
            "ERRORS.md {row}: C unexpectedly succeeded"
        );
        assert!(
            !rust_status.success(),
            "ERRORS.md {row}: Rust unexpectedly succeeded"
        );
        #[cfg(unix)]
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "ERRORS.md {row}: termination signal mismatch: C={c_status:?}, Rust={rust_status:?}"
        );
        #[cfg(not(unix))]
        assert_eq!(
            c_status.code(),
            rust_status.code(),
            "ERRORS.md {row}: termination status mismatch: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn ffi_crash_worker() {
    let Ok(library_choice) = env::var("TFLAC_CRASH_LIBRARY") else {
        return;
    };
    let case = env::var("TFLAC_CRASH_CASE").expect("TFLAC_CRASH_CASE");
    let path = match library_choice.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown TFLAC_CRASH_LIBRARY={other}"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        match case.as_str() {
            "pack_null" => (api.pack_u64le)(std::ptr::null_mut(), 0),
            "add_null" => (api.md5_addsample)(std::ptr::null_mut(), 64, 0),
            "update_null_t" => {
                let samples = [0_i32; 136];
                (api.update_md5)(std::ptr::null_mut(), samples.as_ptr());
            }
            "update_null_samples" => {
                let mut state = Tflac {
                    md5_ctx: TflacMd5 {
                        pos: 0,
                        total: 0,
                        buffer: [0; 72],
                    },
                    cur_blocksize: 40,
                    channels: 1,
                };
                (api.update_md5)(&mut state, std::ptr::null());
            }
            other => panic!("unknown TFLAC_CRASH_CASE={other}"),
        }
    }
}

#[test]
fn all_c_symbols_resolve_from_both_libraries() {
    let _pair = Pair::load();
}
