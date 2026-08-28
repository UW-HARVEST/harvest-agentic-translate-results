use libloading::{Library, Symbol};
use std::env;
use std::ffi::c_int;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sprite {
    texture_id: u64,
    sort_bits: i32,
    padding: [u8; 4],
}

type MergeSort = unsafe extern "C" fn(*mut Sprite, *mut Sprite, c_int);

struct APIs {
    _c_library: Library,
    _rust_library: Library,
    c_merge_sort: MergeSort,
    rust_merge_sort: MergeSort,
}

impl APIs {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_library_path = find_c_library(&manifest_dir.join("../c_src/build"));
        let rust_library_path = manifest_dir.join("target/release/libmerge_sort_lib.so");

        assert!(
            rust_library_path.is_file(),
            "Rust cdylib does not exist at {}",
            rust_library_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load C library {}: {error}",
                    c_library_path.display()
                )
            });
            let rust_library = Library::new(&rust_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load Rust library {}: {error}",
                    rust_library_path.display()
                )
            });
            let c_merge_sort = load_merge_sort(&c_library, &c_library_path);
            let rust_merge_sort = load_merge_sort(&rust_library, &rust_library_path);

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_merge_sort,
                rust_merge_sort,
            }
        }
    }

    fn compare(&self, input: &[Sprite], scratch: &[Sprite], size: c_int) {
        let mut c_input = input.to_vec();
        let mut rust_input = input.to_vec();
        let mut c_scratch = scratch.to_vec();
        let mut rust_scratch = scratch.to_vec();

        unsafe {
            (self.c_merge_sort)(c_input.as_mut_ptr(), c_scratch.as_mut_ptr(), size);
            (self.rust_merge_sort)(rust_input.as_mut_ptr(), rust_scratch.as_mut_ptr(), size);
        }

        assert_eq!(c_input, rust_input, "output buffer differs for size {size}");
        assert_eq!(
            c_scratch, rust_scratch,
            "scratch buffer differs for size {size}"
        );
    }
}

unsafe fn load_merge_sort(library: &Library, path: &Path) -> MergeSort {
    let symbol: Symbol<'_, MergeSort> =
        unsafe { library.get(b"merge_sort\0") }.unwrap_or_else(|error| {
            panic!("failed to load merge_sort from {}: {error}", path.display())
        });
    *symbol
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut libraries: Vec<PathBuf> = fs::read_dir(build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .map(|entry| entry.expect("failed to read C build entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.pop().unwrap()
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn sprite(&mut self, sort_bits: i32) -> Sprite {
        Sprite {
            texture_id: self.next_u64(),
            sort_bits,
            padding: (self.next_u64() as u32).to_ne_bytes(),
        }
    }

    fn sprites(&mut self, length: usize) -> Vec<Sprite> {
        (0..length)
            .map(|_| {
                let sort_bits = self.next_i32();
                self.sprite(sort_bits)
            })
            .collect()
    }
}

fn random_scratch(rng: &mut Rng, length: usize) -> Vec<Sprite> {
    rng.sprites(length)
}

#[test]
fn config_01_empty_input() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0101_0101_0101_0101);

    for _ in 0..256 {
        let input = rng.sprites(1);
        let scratch = rng.sprites(1);
        apis.compare(&input, &scratch, 0);
    }
}

#[test]
fn config_02_single_element() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0202_0202_0202_0202);

    for _ in 0..256 {
        let input = rng.sprites(1);
        let scratch = rng.sprites(1);
        apis.compare(&input, &scratch, 1);
    }
}

#[test]
fn config_03_two_elements_ascending() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0303_0303_0303_0303);

    for _ in 0..256 {
        let left = rng.next_i32() & 0x3fff_ffff;
        let right = left + 1 + (rng.next_i32() & 0x3fff_ffff);
        let input = [rng.sprite(left), rng.sprite(right)];
        let scratch = random_scratch(&mut rng, 2);
        apis.compare(&input, &scratch, 2);
    }
}

#[test]
fn config_04_two_elements_equal_keys() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0404_0404_0404_0404);

    for iteration in 0..256 {
        let key = rng.next_i32();
        let (left_texture, right_texture) = if iteration % 2 == 0 {
            (0, u64::MAX)
        } else {
            (u64::MAX, 0)
        };
        let input = [
            Sprite {
                texture_id: left_texture,
                sort_bits: key,
                padding: (rng.next_u64() as u32).to_ne_bytes(),
            },
            Sprite {
                texture_id: right_texture,
                sort_bits: key,
                padding: (rng.next_u64() as u32).to_ne_bytes(),
            },
        ];
        let scratch = random_scratch(&mut rng, 2);
        apis.compare(&input, &scratch, 2);
    }
}

#[test]
fn config_05_two_elements_descending() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0505_0505_0505_0505);

    for _ in 0..256 {
        let right = rng.next_i32() & 0x3fff_ffff;
        let left = right + 1 + (rng.next_i32() & 0x3fff_ffff);
        let input = [rng.sprite(left), rng.sprite(right)];
        let scratch = random_scratch(&mut rng, 2);
        apis.compare(&input, &scratch, 2);
    }
}

#[test]
fn config_06_odd_many_elements() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0606_0606_0606_0606);

    for _ in 0..256 {
        let length = 3 + 2 * (rng.next_u64() as usize % 63);
        let mut input = rng.sprites(length);
        for sprite in &mut input {
            sprite.sort_bits %= 17;
        }
        let scratch = random_scratch(&mut rng, length);
        apis.compare(&input, &scratch, length as c_int);
    }
}

#[test]
fn config_07_even_many_elements() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0707_0707_0707_0707);

    for _ in 0..256 {
        let length = 4 + 2 * (rng.next_u64() as usize % 63);
        let mut input = rng.sprites(length);
        for sprite in &mut input {
            sprite.sort_bits %= 17;
        }
        let scratch = random_scratch(&mut rng, length);
        apis.compare(&input, &scratch, length as c_int);
    }
}

#[test]
fn config_08_all_equal_keys() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0808_0808_0808_0808);

    for _ in 0..256 {
        let length = 2 + (rng.next_u64() as usize % 127);
        let key = rng.next_i32();
        let input: Vec<Sprite> = (0..length).map(|_| rng.sprite(key)).collect();
        let scratch = random_scratch(&mut rng, length);
        apis.compare(&input, &scratch, length as c_int);
    }
}

#[test]
fn config_09_integer_boundaries() {
    let apis = APIs::load();
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    let boundary_keys = [i32::MIN, i32::MAX, 0, -1, 1, i32::MIN, i32::MAX];

    for _ in 0..256 {
        let repetitions = 1 + (rng.next_u64() as usize % 16);
        let mut input = Vec::with_capacity(boundary_keys.len() * repetitions);
        for _ in 0..repetitions {
            for key in boundary_keys {
                input.push(rng.sprite(key));
            }
        }
        for index in (1..input.len()).rev() {
            let other = rng.next_u64() as usize % (index + 1);
            input.swap(index, other);
        }
        let scratch = random_scratch(&mut rng, input.len());
        apis.compare(&input, &scratch, input.len() as c_int);
    }
}

const PROBE_API: &str = "SPRITEBATCH_PROBE_API";
const PROBE_CASE: &str = "SPRITEBATCH_PROBE_CASE";

#[test]
fn invalid_boundary_probe_child() {
    let Ok(api_name) = env::var(PROBE_API) else {
        return;
    };
    let case = env::var(PROBE_CASE).expect("probe case is required when probe API is set");
    let apis = APIs::load();
    let function = match api_name.as_str() {
        "c" => apis.c_merge_sort,
        "rust" => apis.rust_merge_sort,
        _ => panic!("unknown probe API {api_name}"),
    };
    let mut input = [Sprite {
        texture_id: 0x0123_4567_89ab_cdef,
        sort_bits: -123,
        padding: [0x12, 0x34, 0x56, 0x78],
    }];
    let mut scratch = [Sprite {
        texture_id: 0xfedc_ba98_7654_3210,
        sort_bits: 456,
        padding: [0x87, 0x65, 0x43, 0x21],
    }];

    unsafe {
        match case.as_str() {
            "null-null-zero" => function(std::ptr::null_mut(), std::ptr::null_mut(), 0),
            "null-input-zero" => function(std::ptr::null_mut(), scratch.as_mut_ptr(), 0),
            "null-scratch-zero" => function(input.as_mut_ptr(), std::ptr::null_mut(), 0),
            "null-input-one" => function(std::ptr::null_mut(), scratch.as_mut_ptr(), 1),
            "null-scratch-one" => function(input.as_mut_ptr(), std::ptr::null_mut(), 1),
            "oversized" => function(input.as_mut_ptr(), scratch.as_mut_ptr(), c_int::MAX),
            "negative" => function(input.as_mut_ptr(), scratch.as_mut_ptr(), -1),
            _ => panic!("unknown probe case {case}"),
        }
    }
}

fn run_probe(api: &str, case: &str) -> Output {
    Command::new(env::current_exe().expect("failed to locate integration test executable"))
        .args(["--exact", "invalid_boundary_probe_child", "--nocapture"])
        .env(PROBE_API, api)
        .env(PROBE_CASE, case)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {api} probe {case}: {error}"))
}

fn assert_same_probe_result(case: &str, expect_success: bool) {
    let c = run_probe("c", case);
    let rust = run_probe("rust", case);
    let c_result = (c.status.code(), c.status.signal());
    let rust_result = (rust.status.code(), rust.status.signal());

    assert_eq!(
        c_result,
        rust_result,
        "termination differs for {case}\nC stderr:\n{}\nRust stderr:\n{}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        c.status.success(),
        expect_success,
        "unexpected C termination for {case}: {c_result:?}\n{}",
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(
        rust.status.success(),
        expect_success,
        "unexpected Rust termination for {case}: {rust_result:?}\n{}",
        String::from_utf8_lossy(&rust.stderr)
    );
}

#[test]
fn boundary_null_pointers_with_zero_size() {
    for case in ["null-null-zero", "null-input-zero", "null-scratch-zero"] {
        assert_same_probe_result(case, true);
    }
}

#[test]
fn boundary_null_pointers_with_nonzero_size() {
    for case in ["null-input-one", "null-scratch-one"] {
        assert_same_probe_result(case, false);
    }
}

#[test]
fn boundary_oversized_length() {
    assert_same_probe_result("oversized", false);
}

#[test]
fn boundary_negative_length() {
    assert_same_probe_result("negative", false);
}
