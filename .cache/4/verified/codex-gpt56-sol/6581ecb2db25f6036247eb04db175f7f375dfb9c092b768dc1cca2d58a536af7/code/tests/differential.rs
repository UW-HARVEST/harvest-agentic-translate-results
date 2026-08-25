use libloading::Library;
use std::path::{Path, PathBuf};
use std::ptr;

type ProcessBuffer = unsafe extern "C" fn(*mut u8, usize, u32, i32, i32) -> usize;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_process: ProcessBuffer,
    rust_process: ProcessBuffer,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = library_path("C_DRIVER_SO", root.join("c_src/build/libdriver_c.so"));
        let rust_path = library_path("RUST_DRIVER_SO", root.join("target/release/libdriver.so"));

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_process = *c
                .get::<ProcessBuffer>(b"process_buffer\0")
                .expect("C process_buffer export is missing");
            let rust_process = *rust
                .get::<ProcessBuffer>(b"process_buffer\0")
                .expect("Rust process_buffer export is missing");

            Self {
                _c: c,
                _rust: rust,
                c_process,
                rust_process,
            }
        }
    }

    fn compare(&self, input: &[u8], flags: u32, param1: i32, param2: i32) {
        let guard_len = 32;
        let capacity = input.len().saturating_mul(2).saturating_add(guard_len);
        let mut c_buffer = vec![0xa5; capacity.max(guard_len)];
        let mut rust_buffer = c_buffer.clone();
        c_buffer[..input.len()].copy_from_slice(input);
        rust_buffer[..input.len()].copy_from_slice(input);

        let c_len =
            unsafe { (self.c_process)(c_buffer.as_mut_ptr(), input.len(), flags, param1, param2) };
        let rust_len = unsafe {
            (self.rust_process)(rust_buffer.as_mut_ptr(), input.len(), flags, param1, param2)
        };

        assert_eq!(
            rust_len,
            c_len,
            "length differs: flags={flags:#010x}, param1={param1}, param2={param2}, input_len={}",
            input.len()
        );
        assert!(
            c_len <= capacity,
            "C returned length {c_len} beyond test allocation {capacity}"
        );
        assert_eq!(
            rust_buffer, c_buffer,
            "bytes differ: flags={flags:#010x}, param1={param1}, param2={param2}, input_len={}, returned_len={c_len}",
            input.len()
        );
    }

    fn run(
        &self,
        input: &[u8],
        flags: u32,
        param1: i32,
        param2: i32,
    ) -> (usize, Vec<u8>, usize, Vec<u8>) {
        let capacity = input.len().saturating_mul(2).saturating_add(32).max(32);
        let mut c_buffer = vec![0xa5; capacity];
        let mut rust_buffer = c_buffer.clone();
        c_buffer[..input.len()].copy_from_slice(input);
        rust_buffer[..input.len()].copy_from_slice(input);

        let c_len =
            unsafe { (self.c_process)(c_buffer.as_mut_ptr(), input.len(), flags, param1, param2) };
        let rust_len = unsafe {
            (self.rust_process)(rust_buffer.as_mut_ptr(), input.len(), flags, param1, param2)
        };
        (c_len, c_buffer, rust_len, rust_buffer)
    }
}

fn library_path(variable: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(default)
}

fn require_file(path: &Path) {
    assert!(
        path.is_file(),
        "required shared library does not exist: {}",
        path.display()
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

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn randomized_input(rng: &mut Rng, len: usize, mode: usize) -> Vec<u8> {
    match mode % 5 {
        0 => (0..len).map(|_| rng.byte()).collect(),
        1 => (0..len).map(|_| rng.byte() & 0x07).collect(),
        2 => {
            let mut output = Vec::with_capacity(len);
            while output.len() < len {
                let value = rng.byte();
                let run = 1 + (rng.next_u64() as usize % 12);
                output.extend(std::iter::repeat(value).take(run.min(len - output.len())));
            }
            output
        }
        3 => vec![rng.byte(); len],
        _ => (0..len)
            .map(|index| match index % 4 {
                0 => 0,
                1 => 255,
                2 => index as u8,
                _ => (index / 2) as u8,
            })
            .collect(),
    }
}

#[test]
fn shared_objects_and_exports_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    require_file(&library_path(
        "C_DRIVER_SO",
        root.join("c_src/build/libdriver_c.so"),
    ));
    require_file(&library_path(
        "RUST_DRIVER_SO",
        root.join("target/release/libdriver.so"),
    ));
    let _libraries = Libraries::load();
}

#[test]
fn rows_1_to_32_all_flag_masks_randomized() {
    let libraries = Libraries::load();
    let params = [
        i32::MIN,
        -257,
        -17,
        -1,
        0,
        1,
        2,
        3,
        4,
        17,
        127,
        254,
        255,
        256,
        257,
        i32::MAX,
    ];
    let boundary_lengths = [1, 2, 3, 4, 5, 7, 8, 15, 16, 127, 128, 255, 256];
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    for flags in 0_u32..=0x1f {
        for case in 0..160 {
            let len = if case < boundary_lengths.len() {
                boundary_lengths[case]
            } else {
                1 + (rng.next_u64() as usize % 256)
            };
            let input = randomized_input(&mut rng, len, case);
            let param1 = params[case % params.len()];
            let param2 = match case % 4 {
                0 => 0,
                1 => 1,
                2 => -1,
                _ => rng.next_u64() as i32,
            };
            libraries.compare(&input, flags, param1, param2);
        }
    }
}

#[test]
fn row_33_unknown_flag_bits_are_ignored() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);

    for case in 0..128 {
        let len = 1 + (rng.next_u64() as usize % 256);
        let input = randomized_input(&mut rng, len, case);
        let param1 = (rng.next_u64() as i32 % 600) - 300;
        let param2 = rng.next_u64() as i32;

        libraries.compare(&input, 0xffff_ffe0, param1, param2);
        libraries.compare(&input, 0xffff_ffff, param1, param2);

        let (c_base_len, c_base, _, _) = libraries.run(&input, 0x1f, param1, param2);
        let (c_high_len, c_high, rust_high_len, rust_high) =
            libraries.run(&input, 0xffff_ffff, param1, param2);
        assert_eq!(c_high_len, c_base_len);
        assert_eq!(c_high, c_base);
        assert_eq!(rust_high_len, c_high_len);
        assert_eq!(rust_high, c_high);
    }
}

#[test]
fn rows_34_to_38_rotate_branches() {
    let libraries = Libraries::load();
    let cases = [
        (vec![7], 19),
        ((0..8).collect(), 16),
        ((0..9).collect(), -2),
        ((0..10).collect(), 2),
        ((0..10).collect(), 5),
    ];

    for (input, param1) in cases {
        libraries.compare(&input, 0x01, param1, 0);
    }
}

#[test]
fn rows_39_to_45_compaction_branches() {
    let libraries = Libraries::load();
    let cases = [
        (vec![1, 1, 1, 2, 3, 3, 3], 0),
        (vec![1, 2, 3, 4, 5], 1),
        (vec![8, 8, 9, 9, 9], 3),
        (vec![8, 8, 8, 9], 3),
        (vec![8, 8, 8, 8, 9, 10], 3),
        (vec![1, 1, 1, 2], 256),
        (vec![42; 256], 2),
    ];

    for (input, param1) in cases {
        libraries.compare(&input, 0x02, param1, 0);
    }
}

#[test]
fn rows_46_to_48_deduplication_branches() {
    let libraries = Libraries::load();
    libraries.compare(&[9], 0x04, 0, 0);
    libraries.compare(&[3, 1, 3, 2, 1, 0, 255, 0], 0x04, 0, 0);
    libraries.compare(&[3, 1, 3, 2, 1, 0, 255, 0], 0x04, 0, -7);
}

#[test]
fn rows_49_to_53_interleave_branches() {
    let libraries = Libraries::load();
    libraries.compare(&[9], 0x08, 0, 0);
    libraries.compare(&(0..10).collect::<Vec<u8>>(), 0x08, 0, 0);
    libraries.compare(&(0..9).collect::<Vec<u8>>(), 0x08, 0, 0);

    let even = (0..514).map(|index| index as u8).collect::<Vec<_>>();
    let odd = (0..515).map(|index| (index * 17) as u8).collect::<Vec<_>>();
    libraries.compare(&even, 0x08, 0, 0);
    libraries.compare(&odd, 0x08, 0, 0);
}

#[test]
fn rows_54_to_60_reverse_segment_branches() {
    let libraries = Libraries::load();
    let cases = [
        ((0..3).collect::<Vec<u8>>(), 2),
        ((0..8).collect::<Vec<u8>>(), 0),
        ((0..8).collect::<Vec<u8>>(), 1),
        ((0..8).collect::<Vec<u8>>(), 9),
        ((0..8).collect::<Vec<u8>>(), 4),
        ((0..9).collect::<Vec<u8>>(), 4),
        ((0..10).collect::<Vec<u8>>(), 4),
    ];

    for (input, param1) in cases {
        libraries.compare(&input, 0x10, param1, 0);
    }
}

#[test]
fn rows_61_to_64_composed_and_large_rotate_branches() {
    let libraries = Libraries::load();
    libraries.compare(&[5, 5, 5], 0x1a, 3, 0);
    libraries.compare(&[0, 0, 0, 1, 2, 2, 2, 3, 4, 4, 4, 5], 0x13, 3, 0);

    let mut rng = Rng::new(0xe703_7ed1_a0b4_28db);
    for case in 0..256 {
        let len = 1 + (rng.next_u64() as usize % 256);
        let input = randomized_input(&mut rng, len, case);
        let params = [-257, -1, 0, 1, 2, 3, 4, 255, 256, 257];
        libraries.compare(
            &input,
            0x1f,
            params[case % params.len()],
            if case % 2 == 0 { 0 } else { -1 },
        );
    }

    let large_rotate = (0..700)
        .map(|index| (index * 29 + 7) as u8)
        .collect::<Vec<_>>();
    libraries.compare(&large_rotate, 0x01, 300, 0);
}

#[test]
fn error_row_1_null_pointer_returns_zero() {
    let libraries = Libraries::load();
    let lengths = [0, 1, 256, usize::MAX];

    for length in lengths {
        for flags in [0, 0x1f, u32::MAX] {
            let c_result =
                unsafe { (libraries.c_process)(ptr::null_mut(), length, flags, i32::MIN, -1) };
            let rust_result =
                unsafe { (libraries.rust_process)(ptr::null_mut(), length, flags, i32::MIN, -1) };
            assert_eq!(c_result, 0);
            assert_eq!(rust_result, c_result);
        }
    }
}

#[test]
fn error_row_2_zero_length_returns_zero_without_writes() {
    let libraries = Libraries::load();

    for flags in [0, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f, u32::MAX] {
        for param1 in [i32::MIN, -1, 0, 1, 255, 256, i32::MAX] {
            let (c_len, c_buffer, rust_len, rust_buffer) = libraries.run(&[], flags, param1, -1);
            assert_eq!(c_len, 0);
            assert_eq!(rust_len, c_len);
            assert_eq!(rust_buffer, c_buffer);
            assert!(c_buffer.iter().all(|&byte| byte == 0xa5));
        }
    }
}
