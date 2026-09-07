use libloading::{Library, Symbol};
use std::env;
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::ptr;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sprite {
    texture_id: u64,
    sort_bits: i32,
    padding: [u8; 4],
}

type MergeSort = unsafe extern "C" fn(*mut Sprite, *mut Sprite, c_int);

const C_LIBRARY_NAME: &str = "libharvest-work-68uRX7.so";
const RUST_LIBRARY_NAME: &str = "libmerge_sort_lib.so";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation crate must have a parent")
        .join("c_src")
        .join("build")
        .join(C_LIBRARY_NAME)
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("release")
        .join(RUST_LIBRARY_NAME)
}

fn assert_libraries_exist() {
    assert!(
        c_library_path().is_file(),
        "missing C library at {}",
        c_library_path().display()
    );
    assert!(
        rust_library_path().is_file(),
        "missing Rust library at {}",
        rust_library_path().display()
    );
}

fn sprite_bytes(sprites: &[Sprite]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            sprites.as_ptr().cast::<u8>(),
            std::mem::size_of_val(sprites),
        )
    }
}

fn compare_case(label: &str, input: &[Sprite], scratch: &[Sprite]) {
    assert_eq!(
        input.len(),
        scratch.len(),
        "{label}: input and scratch lengths differ"
    );
    let size = c_int::try_from(input.len()).expect("test input length fits in c_int");
    let mut c_output = input.to_vec();
    let mut c_scratch = scratch.to_vec();
    let mut rust_output = input.to_vec();
    let mut rust_scratch = scratch.to_vec();

    assert_libraries_exist();
    unsafe {
        let c_library = Library::new(c_library_path()).expect("load C shared library");
        let rust_library = Library::new(rust_library_path()).expect("load Rust shared library");
        let c_merge_sort: Symbol<MergeSort> =
            c_library.get(b"merge_sort\0").expect("load C merge_sort");
        let rust_merge_sort: Symbol<MergeSort> = rust_library
            .get(b"merge_sort\0")
            .expect("load Rust merge_sort");

        c_merge_sort(c_output.as_mut_ptr(), c_scratch.as_mut_ptr(), size);
        rust_merge_sort(rust_output.as_mut_ptr(), rust_scratch.as_mut_ptr(), size);
    }

    assert_eq!(
        sprite_bytes(&c_output),
        sprite_bytes(&rust_output),
        "{label}: result buffer differs"
    );
    assert_eq!(
        sprite_bytes(&c_scratch),
        sprite_bytes(&rust_scratch),
        "{label}: scratch buffer differs"
    );
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range_usize(&mut self, low: usize, high_exclusive: usize) -> usize {
        low + (self.next_u64() as usize % (high_exclusive - low))
    }

    fn sprite(&mut self, sort_bits: i32) -> Sprite {
        let padding_bits = self.next_u64().to_ne_bytes();
        Sprite {
            texture_id: self.next_u64(),
            sort_bits,
            padding: [
                padding_bits[0],
                padding_bits[1],
                padding_bits[2],
                padding_bits[3],
            ],
        }
    }

    fn arbitrary_sprite(&mut self) -> Sprite {
        let sort_bits = self.next_i32();
        self.sprite(sort_bits)
    }
}

fn random_scratch(rng: &mut Rng, len: usize) -> Vec<Sprite> {
    (0..len).map(|_| rng.arbitrary_sprite()).collect()
}

#[test]
fn config_01_empty_input() {
    let mut rng = Rng::new(0x0101_0101_0101_0101);
    for iteration in 0..128 {
        compare_case(
            &format!("empty iteration {iteration}"),
            &[],
            &random_scratch(&mut rng, 0),
        );
    }
}

#[test]
fn config_02_singleton_input() {
    let mut rng = Rng::new(0x0202_0202_0202_0202);
    for iteration in 0..256 {
        let input = [rng.arbitrary_sprite()];
        let scratch = random_scratch(&mut rng, 1);
        compare_case(
            &format!("singleton iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_03_pair_left_less_than_or_equal() {
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for iteration in 0..512 {
        let first = (rng.next_u64() % 65_536) as i32 - 32_768;
        let second = (rng.next_u64() % 65_536) as i32 - 32_768;
        let low = first.min(second);
        let high = first.max(second);
        let input = [rng.sprite(low), rng.sprite(high)];
        let scratch = random_scratch(&mut rng, 2);
        compare_case(
            &format!("left <= right iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_04_pair_left_greater() {
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    for iteration in 0..512 {
        let low = (rng.next_u64() % 65_535) as i32 - 32_768;
        let gap = (rng.next_u64() % 32_768 + 1) as i32;
        let high = low + gap;
        let input = [rng.sprite(high), rng.sprite(low)];
        let scratch = random_scratch(&mut rng, 2);
        compare_case(
            &format!("left > right iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_05_even_mixed_inputs() {
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for iteration in 0..512 {
        let len = rng.range_usize(2, 33) * 2;
        let input: Vec<_> = (0..len)
            .map(|_| {
                let sort_bits = (rng.next_u64() % 33) as i32 - 16;
                rng.sprite(sort_bits)
            })
            .collect();
        let scratch = random_scratch(&mut rng, len);
        compare_case(
            &format!("even mixed iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_06_odd_mixed_inputs() {
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for iteration in 0..512 {
        let len = rng.range_usize(1, 32) * 2 + 1;
        let input: Vec<_> = (0..len)
            .map(|_| {
                let sort_bits = (rng.next_u64() % 33) as i32 - 16;
                rng.sprite(sort_bits)
            })
            .collect();
        let scratch = random_scratch(&mut rng, len);
        compare_case(
            &format!("odd mixed iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_07_equal_sort_bits_ignore_texture_id() {
    let mut rng = Rng::new(0x0707_0707_0707_0707);
    for iteration in 0..512 {
        let len = rng.range_usize(2, 65);
        let sort_bits = rng.next_i32();
        let mut input: Vec<_> = (0..len).map(|_| rng.sprite(sort_bits)).collect();
        for (index, sprite) in input.iter_mut().enumerate() {
            sprite.texture_id = if index % 2 == 0 {
                u64::MAX - index as u64
            } else {
                index as u64
            };
        }
        let scratch = random_scratch(&mut rng, len);
        compare_case(
            &format!("equal sort_bits iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

#[test]
fn config_08_integer_boundaries() {
    let mut rng = Rng::new(0x0808_0808_0808_0808);
    for iteration in 0..512 {
        let len = rng.range_usize(3, 65);
        let mut input: Vec<_> = (0..len)
            .map(|_| {
                let sort_bits = rng.next_i32();
                rng.sprite(sort_bits)
            })
            .collect();
        input[0].sort_bits = i32::MIN;
        input[len / 2].sort_bits = i32::MAX;
        input[len - 1].sort_bits = if iteration % 2 == 0 {
            i32::MIN
        } else {
            i32::MAX
        };
        let scratch = random_scratch(&mut rng, len);
        compare_case(
            &format!("integer boundaries iteration {iteration}"),
            &input,
            &scratch,
        );
    }
}

fn run_boundary_child(library: &str, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("resolve current integration-test executable"))
        .arg("--exact")
        .arg("ffi_boundary_child")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .status()
        .expect("run boundary child")
}

fn status_description(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        format!(
            "success={}, code={:?}, signal={:?}, core_dumped={}",
            status.success(),
            status.code(),
            status.signal(),
            status.core_dumped()
        )
    }
    #[cfg(not(unix))]
    {
        format!("success={}, code={:?}", status.success(), status.code())
    }
}

#[test]
fn error_surface_generic_boundaries() {
    assert_libraries_exist();
    for case in [
        "zero_null",
        "null_input",
        "null_scratch",
        "negative",
        "oversized",
    ] {
        let c_status = run_boundary_child("c", case);
        let rust_status = run_boundary_child("rust", case);
        assert_eq!(
            status_description(c_status),
            status_description(rust_status),
            "boundary case {case} differs between C and Rust"
        );
    }
}

#[test]
fn ffi_boundary_child() {
    let Ok(library_kind) = env::var("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let case = env::var("DIFF_CHILD_CASE").expect("DIFF_CHILD_CASE must be set");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library kind: {other}"),
    };

    let mut valid_input = [Sprite {
        texture_id: 1,
        sort_bits: 2,
        padding: [3, 4, 5, 6],
    }];
    let mut valid_scratch = [Sprite {
        texture_id: 7,
        sort_bits: 8,
        padding: [9, 10, 11, 12],
    }];

    unsafe {
        let library = Library::new(path).expect("load child shared library");
        let merge_sort: Symbol<MergeSort> =
            library.get(b"merge_sort\0").expect("load child merge_sort");
        match case.as_str() {
            "zero_null" => merge_sort(ptr::null_mut(), ptr::null_mut(), 0),
            "null_input" => merge_sort(ptr::null_mut(), valid_scratch.as_mut_ptr(), 1),
            "null_scratch" => merge_sort(valid_input.as_mut_ptr(), ptr::null_mut(), 1),
            "negative" => merge_sort(ptr::null_mut(), ptr::null_mut(), -1),
            "oversized" => merge_sort(ptr::null_mut(), ptr::null_mut(), c_int::MAX),
            other => panic!("unknown child boundary case: {other}"),
        }
    }
}
