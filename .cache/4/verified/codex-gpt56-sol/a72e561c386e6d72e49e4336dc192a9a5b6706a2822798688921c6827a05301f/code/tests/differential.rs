use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const TRIALS: usize = 128;
const SPRITE_SIZE: usize = 16;

#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SpriteBytes {
    bytes: [u8; SPRITE_SIZE],
}

impl SpriteBytes {
    fn new(texture_id: u64, sort_bits: i32, padding: [u8; 4]) -> Self {
        let mut bytes = [0; SPRITE_SIZE];
        bytes[..8].copy_from_slice(&texture_id.to_ne_bytes());
        bytes[8..12].copy_from_slice(&sort_bits.to_ne_bytes());
        bytes[12..].copy_from_slice(&padding);
        Self { bytes }
    }

    fn sort_bits(self) -> i32 {
        i32::from_ne_bytes(self.bytes[8..12].try_into().unwrap())
    }
}

type MergeSort = unsafe extern "C" fn(*mut SpriteBytes, *mut SpriteBytes, c_int);

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn padding(&mut self) -> [u8; 4] {
        (self.next_u64() as u32).to_ne_bytes()
    }

    fn sprite(&mut self) -> SpriteBytes {
        SpriteBytes::new(self.next_u64(), self.next_i32(), self.padding())
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_REFERENCE_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("c_src/build/libtranslated_rust.so"))
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_REFERENCE_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target/release/libmerge_sort_lib.so"))
}

fn load_merge_sort(library: &Library) -> MergeSort {
    let symbol: Symbol<'_, MergeSort> =
        unsafe { library.get(b"merge_sort\0") }.expect("load merge_sort");
    *symbol
}

fn with_implementations(test: impl FnOnce(MergeSort, MergeSort)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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

    let c_library = unsafe { Library::new(c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(rust_path) }.expect("load Rust shared library");
    test(load_merge_sort(&c_library), load_merge_sort(&rust_library));
}

fn compare_call(
    c_merge_sort: MergeSort,
    rust_merge_sort: MergeSort,
    input: Vec<SpriteBytes>,
    scratch: Vec<SpriteBytes>,
    size: c_int,
) {
    let mut c_input = input.clone();
    let mut rust_input = input;
    let mut c_scratch = scratch.clone();
    let mut rust_scratch = scratch;

    unsafe {
        c_merge_sort(c_input.as_mut_ptr(), c_scratch.as_mut_ptr(), size);
        rust_merge_sort(rust_input.as_mut_ptr(), rust_scratch.as_mut_ptr(), size);
    }

    assert_eq!(
        rust_input, c_input,
        "primary buffer differs for size {size}"
    );
    assert_eq!(
        rust_scratch, c_scratch,
        "scratch buffer differs for size {size}"
    );
}

fn random_sprites(rng: &mut Rng, len: usize) -> Vec<SpriteBytes> {
    (0..len).map(|_| rng.sprite()).collect()
}

#[test]
fn config_01_zero_length() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0101_0101_0101_0101);
        for _ in 0..TRIALS {
            compare_call(c, rust, vec![rng.sprite()], vec![rng.sprite()], 0);
        }
    });
}

#[test]
fn config_02_single_record() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0202_0202_0202_0202);
        for _ in 0..TRIALS {
            compare_call(c, rust, vec![rng.sprite()], vec![rng.sprite()], 1);
        }
    });
}

#[test]
fn config_03_pair_takes_left() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0303_0303_0303_0303);
        for trial in 0..TRIALS {
            let left = rng.next_i32() >> 1;
            let right = if trial % 2 == 0 {
                left
            } else {
                left.saturating_add((rng.next_u64() % 1024 + 1) as i32)
            };
            let input = vec![
                SpriteBytes::new(rng.next_u64(), left, rng.padding()),
                SpriteBytes::new(rng.next_u64(), right, rng.padding()),
            ];
            compare_call(c, rust, input, random_sprites(&mut rng, 2), 2);
        }
    });
}

#[test]
fn config_04_pair_takes_right() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0404_0404_0404_0404);
        for _ in 0..TRIALS {
            let right = rng.next_i32() >> 1;
            let left = right.saturating_add((rng.next_u64() % 1024 + 1) as i32);
            let input = vec![
                SpriteBytes::new(rng.next_u64(), left, rng.padding()),
                SpriteBytes::new(rng.next_u64(), right, rng.padding()),
            ];
            compare_call(c, rust, input, random_sprites(&mut rng, 2), 2);
        }
    });
}

#[test]
fn config_05_odd_recursive_shapes() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0505_0505_0505_0505);
        for _ in 0..TRIALS {
            let len = 3 + 2 * (rng.next_u64() as usize % 31);
            compare_call(
                c,
                rust,
                random_sprites(&mut rng, len),
                random_sprites(&mut rng, len),
                len as c_int,
            );
        }
    });
}

#[test]
fn config_06_even_recursive_shapes() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0606_0606_0606_0606);
        for _ in 0..TRIALS {
            let len = 4 + 2 * (rng.next_u64() as usize % 31);
            compare_call(
                c,
                rust,
                random_sprites(&mut rng, len),
                random_sprites(&mut rng, len),
                len as c_int,
            );
        }
    });
}

#[test]
fn config_07_equal_sort_bits_ignore_texture_id() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0707_0707_0707_0707);
        for _ in 0..TRIALS {
            let len = 2 + rng.next_u64() as usize % 63;
            let sort_bits = rng.next_i32();
            let input = (0..len)
                .map(|_| SpriteBytes::new(rng.next_u64(), sort_bits, rng.padding()))
                .collect();
            compare_call(c, rust, input, random_sprites(&mut rng, len), len as c_int);
        }
    });
}

#[test]
fn config_08_mixed_repeated_sort_bits() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0808_0808_0808_0808);
        let keys = [-2, -1, 0, 1, 2];
        for _ in 0..TRIALS {
            let len = 8 + rng.next_u64() as usize % 57;
            let input = (0..len)
                .map(|_| {
                    let key = keys[rng.next_u64() as usize % keys.len()];
                    SpriteBytes::new(rng.next_u64(), key, rng.padding())
                })
                .collect();
            compare_call(c, rust, input, random_sprites(&mut rng, len), len as c_int);
        }
    });
}

#[test]
fn config_09_already_sorted() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x0909_0909_0909_0909);
        for _ in 0..TRIALS {
            let len = 2 + rng.next_u64() as usize % 63;
            let mut input = random_sprites(&mut rng, len);
            input.sort_by_key(|sprite| sprite.sort_bits());
            compare_call(c, rust, input, random_sprites(&mut rng, len), len as c_int);
        }
    });
}

#[test]
fn config_10_reverse_sorted() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x1010_1010_1010_1010);
        for _ in 0..TRIALS {
            let len = 2 + rng.next_u64() as usize % 63;
            let mut input = random_sprites(&mut rng, len);
            input.sort_by_key(|sprite| std::cmp::Reverse(sprite.sort_bits()));
            compare_call(c, rust, input, random_sprites(&mut rng, len), len as c_int);
        }
    });
}

#[test]
fn config_11_integer_extrema() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x1111_1111_1111_1111);
        let keys = [i32::MIN, i32::MAX, 0, -1, 1];
        let textures = [0, u64::MAX, 1, u64::MAX - 1];
        for _ in 0..TRIALS {
            let len = 5 + rng.next_u64() as usize % 60;
            let input = (0..len)
                .map(|index| {
                    let key = keys[index % keys.len()];
                    let texture = textures[rng.next_u64() as usize % textures.len()];
                    SpriteBytes::new(texture, key, rng.padding())
                })
                .collect();
            compare_call(c, rust, input, random_sprites(&mut rng, len), len as c_int);
        }
    });
}

#[test]
fn config_12_large_valid_length() {
    with_implementations(|c, rust| {
        let mut rng = Rng::new(0x1212_1212_1212_1212);
        for _ in 0..16 {
            let len = 8_192;
            compare_call(
                c,
                rust,
                random_sprites(&mut rng, len),
                random_sprites(&mut rng, len),
                len as c_int,
            );
        }
    });
}

fn dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn dynamic_symbol_surfaces_match() {
    assert_eq!(
        dynamic_symbols(&rust_library_path()),
        dynamic_symbols(&c_library_path())
    );
}

#[cfg(unix)]
fn exact_outcome(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

fn run_boundary_child(library: &Path, boundary: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_BOUNDARY", boundary)
        .status()
        .expect("run isolated boundary child")
}

#[test]
fn generic_undefined_boundaries_have_matching_observed_outcomes() {
    for boundary in ["null_zero", "null_source", "null_destination", "huge"] {
        let c_status = run_boundary_child(&c_library_path(), boundary);
        let rust_status = run_boundary_child(&rust_library_path(), boundary);
        assert_eq!(
            exact_outcome(rust_status),
            exact_outcome(c_status),
            "process outcome differs for undefined boundary {boundary}"
        );
    }
}

#[test]
fn ffi_boundary_child() {
    let Some(library_path) = std::env::var_os("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let boundary = std::env::var("DIFF_CHILD_BOUNDARY").expect("child boundary name");
    let library = unsafe { Library::new(library_path) }.expect("load child shared library");
    let merge_sort = load_merge_sort(&library);
    let mut one = SpriteBytes::new(1, 1, [1, 2, 3, 4]);

    unsafe {
        match boundary.as_str() {
            "null_zero" => merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), 0),
            "null_source" => merge_sort(std::ptr::null_mut(), &mut one, 1),
            "null_destination" => merge_sort(&mut one, std::ptr::null_mut(), 1),
            "huge" => merge_sort(std::ptr::null_mut(), std::ptr::null_mut(), c_int::MAX),
            _ => panic!("unknown boundary {boundary}"),
        }
    }
}
