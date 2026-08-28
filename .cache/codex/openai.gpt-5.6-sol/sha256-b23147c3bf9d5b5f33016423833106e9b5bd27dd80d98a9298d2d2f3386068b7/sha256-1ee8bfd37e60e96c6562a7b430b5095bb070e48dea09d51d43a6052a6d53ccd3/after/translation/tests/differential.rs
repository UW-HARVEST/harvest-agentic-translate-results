use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Tfm = unsafe extern "C" fn(*mut f32, *const f32, c_int);

const ITERATIONS: usize = 2_048;
const DEST_GUARD_BITS: [u32; 2] = [0x7fc1_2345, 0xff81_5678];

struct Implementations {
    _c: Library,
    _rust: Library,
    c_tfm: Tfm,
    rust_tfm: Tfm,
}

impl Implementations {
    fn load() -> Self {
        let c_path = find_single_library(&manifest_dir().join("../c_src/build"), "libtfm_lib.so");
        let rust_path = rust_library_path();

        let c = unsafe { Library::new(&c_path) }.unwrap_or_else(|error| {
            panic!("failed to load C library {}: {error}", c_path.display())
        });
        let rust = unsafe { Library::new(&rust_path) }.unwrap_or_else(|error| {
            panic!(
                "failed to load Rust library {}: {error}",
                rust_path.display()
            )
        });

        let c_tfm = load_tfm(&c, &c_path);
        let rust_tfm = load_tfm(&rust, &rust_path);

        Self {
            _c: c,
            _rust: rust,
            c_tfm,
            rust_tfm,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TFM_RUST_SO") {
        return PathBuf::from(path);
    }

    let target = manifest_dir().join("target");
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("libtfm_lib.so");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "Rust cdylib not found under {}; build the crate before running tests",
        target.display()
    );
}

fn find_single_library(directory: &Path, excluded_name: &str) -> PathBuf {
    let mut libraries: Vec<_> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|extension| extension == "so")
                && path.file_name().is_some_and(|name| name != excluded_name)
        })
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected one C shared library in {}, found {libraries:?}",
        directory.display()
    );
    libraries.pop().unwrap()
}

fn load_tfm(library: &Library, path: &Path) -> Tfm {
    let symbol: Symbol<'_, Tfm> = unsafe { library.get(b"tfm\0") }
        .unwrap_or_else(|error| panic!("failed to resolve tfm from {}: {error}", path.display()));
    *symbol
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.0 = state;
        state as u32
    }

    fn finite(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let exponent = (self.next_u32() % 254) << 23;
        let fraction = self.next_u32() & 0x007f_ffff;
        f32::from_bits(sign | exponent | fraction)
    }

    fn any(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x007f_ffff).max(1);
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }
}

fn destination(count: usize) -> Vec<f32> {
    let mut values = Vec::with_capacity(count * 2 + DEST_GUARD_BITS.len());
    values.extend(DEST_GUARD_BITS.map(f32::from_bits));
    values.extend(
        (0..count * 2).map(|index| f32::from_bits(0xa5a5_0000_u32.wrapping_add(index as u32))),
    );
    values
}

fn assert_same(implementations: &Implementations, src: &[f32], count: c_int) {
    assert!(count >= 0);
    assert!(src.len() >= count as usize * 3);

    let mut c_dest = destination(count as usize);
    let mut rust_dest = c_dest.clone();
    unsafe {
        (implementations.c_tfm)(c_dest.as_mut_ptr().add(2), src.as_ptr(), count);
        (implementations.rust_tfm)(rust_dest.as_mut_ptr().add(2), src.as_ptr(), count);
    }

    let c_bits: Vec<_> = c_dest.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<_> = rust_dest.iter().map(|value| value.to_bits()).collect();
    if let Some(index) = c_bits
        .iter()
        .zip(&rust_bits)
        .position(|(c_value, rust_value)| c_value != rust_value)
    {
        let source_index = index.saturating_sub(2) / 2 * 3;
        let source_end = (source_index + 3).min(src.len());
        let source_bits: Vec<_> = src[source_index..source_end]
            .iter()
            .map(|value| format!("{:08x}", value.to_bits()))
            .collect();
        panic!(
            "first mismatch at destination bit index {index}: C={:08x}, Rust={:08x}, \
             source triple={source_bits:?}, count={count}",
            c_bits[index], rust_bits[index]
        );
    }
    assert_eq!(&c_bits[..2], &DEST_GUARD_BITS);
}

fn assert_same_overlapping(
    implementations: &Implementations,
    initial: &[f32],
    dest_offset: usize,
    src_offset: usize,
    count: c_int,
) {
    assert!(count >= 0);
    assert!(initial.len() >= dest_offset + count as usize * 2);
    assert!(initial.len() >= src_offset + count as usize * 3);

    let mut c_buffer = initial.to_vec();
    let mut rust_buffer = initial.to_vec();
    unsafe {
        (implementations.c_tfm)(
            c_buffer.as_mut_ptr().add(dest_offset),
            c_buffer.as_ptr().add(src_offset),
            count,
        );
        (implementations.rust_tfm)(
            rust_buffer.as_mut_ptr().add(dest_offset),
            rust_buffer.as_ptr().add(src_offset),
            count,
        );
    }

    let c_bits: Vec<_> = c_buffer.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<_> = rust_buffer.iter().map(|value| value.to_bits()).collect();
    assert_eq!(
        c_bits, rust_bits,
        "overlap mismatch: dest_offset={dest_offset}, src_offset={src_offset}, count={count}"
    );
}

#[test]
fn config_1_negative_count_does_not_access_null_pointers() {
    let implementations = Implementations::load();
    for count in [-1, -2, -17, c_int::MIN] {
        unsafe {
            (implementations.c_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
            (implementations.rust_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
        }
    }
}

#[test]
fn config_2_zero_count_does_not_access_null_pointers() {
    let implementations = Implementations::load();
    unsafe {
        (implementations.c_tfm)(std::ptr::null_mut(), std::ptr::null(), 0);
        (implementations.rust_tfm)(std::ptr::null_mut(), std::ptr::null(), 0);
    }
}

#[test]
fn config_3_single_first_branch() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x48cd_9f12_359e_a771);
    for src in [
        [f32::NEG_INFINITY, 0.0, f32::INFINITY],
        [0.0, f32::INFINITY, f32::NEG_INFINITY],
        [
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::from_bits(0x7fa1_2345),
        ],
    ] {
        assert_same(&implementations, &src, 1);
    }
    for _ in 0..ITERATIONS {
        let a = rng.finite();
        let b = rng.finite();
        let (first, second) = if a < b {
            (a, b)
        } else if b < a {
            (b, a)
        } else {
            (f32::NEG_INFINITY, f32::INFINITY)
        };
        assert_same(&implementations, &[first, second, rng.any()], 1);
    }
}

#[test]
fn config_4_single_else_branch() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x917b_445c_0a61_e3df);

    for src in [
        [0.0, -0.0, 0.0],
        [-0.0, 0.0, -0.0],
        [1.0, 1.0, f32::INFINITY],
        [f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
    ] {
        assert_same(&implementations, &src, 1);
    }

    for iteration in 0..ITERATIONS {
        let a = rng.finite();
        let b = rng.finite();
        let (first, second) = if a >= b { (a, b) } else { (b, a) };
        let second = if iteration % 16 == 0 { first } else { second };
        assert_same(&implementations, &[first, second, rng.any()], 1);
    }
}

#[test]
fn config_5_single_unordered_nan_branch() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xe2a8_1390_76bc_54fd);
    for iteration in 0..ITERATIONS {
        let first = if iteration & 1 == 0 {
            rng.nan()
        } else {
            rng.any()
        };
        let second = if iteration & 1 == 0 {
            rng.any()
        } else {
            rng.nan()
        };
        assert!(!(first < second));
        assert_same(&implementations, &[first, second, rng.any()], 1);
    }
}

#[test]
fn config_6_many_mixed_triples() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x71af_8362_19de_c405);
    for count in [2, 3, 7, 31, 257] {
        for _ in 0..64 {
            let mut src = Vec::with_capacity(count * 3);
            for index in 0..count {
                let (first, second) = match index % 3 {
                    0 => {
                        let a = rng.finite();
                        let b = rng.finite();
                        if a < b {
                            (a, b)
                        } else {
                            (f32::NEG_INFINITY, f32::INFINITY)
                        }
                    }
                    1 => {
                        let a = rng.finite();
                        let b = rng.finite();
                        if a >= b { (a, b) } else { (b, a) }
                    }
                    _ => (rng.nan(), rng.any()),
                };
                src.extend([first, second, rng.any()]);
            }
            assert_same(&implementations, &src, count as c_int);
        }
    }
}

#[test]
fn config_7_overlapping_source_and_destination() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x3c07_b8e1_a426_95df);
    let count = 31;
    for (dest_offset, src_offset) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (0, 4)] {
        for _ in 0..64 {
            let length =
                (dest_offset + count * 2).max(src_offset + count * 3) + DEST_GUARD_BITS.len();
            let initial: Vec<_> = (0..length).map(|_| rng.any()).collect();
            assert_same_overlapping(
                &implementations,
                &initial,
                dest_offset,
                src_offset,
                count as c_int,
            );
        }
    }
}

#[test]
fn generic_large_valid_count() {
    let implementations = Implementations::load();
    let count = 16_384;
    let mut rng = Rng::new(0x9f68_2d31_40ba_c7e5);
    let src: Vec<_> = (0..count * 3).map(|_| rng.any()).collect();
    assert_same(&implementations, &src, count as c_int);
}
