use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

type Wcscat = unsafe extern "C" fn(*mut c_int, usize, *const c_int) -> c_int;

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libharvest-work-kx3K47.so");
        let profile_dir = std::env::current_exe()
            .expect("current test executable path")
            .parent()
            .and_then(|path| path.parent())
            .expect("Cargo profile directory")
            .to_owned();
        let rust_path = profile_dir.join("libwcscat_lib.so");

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

        // Both paths are controlled build outputs with the expected ABI.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    fn functions(&self) -> (Symbol<'_, Wcscat>, Symbol<'_, Wcscat>) {
        // The symbol type exactly matches the declaration in the public C header.
        unsafe {
            (
                self.c.get(b"wcscat\0").expect("load C wcscat"),
                self.rust.get(b"wcscat\0").expect("load Rust wcscat"),
            )
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum LengthShape {
    Empty,
    One,
    Many,
}

#[derive(Clone, Copy, Debug)]
struct ValidRow {
    number: usize,
    dst: LengthShape,
    src: LengthShape,
    spare_capacity: bool,
}

const VALID_ROWS: [ValidRow; 18] = [
    ValidRow {
        number: 1,
        dst: LengthShape::Empty,
        src: LengthShape::Empty,
        spare_capacity: false,
    },
    ValidRow {
        number: 2,
        dst: LengthShape::Empty,
        src: LengthShape::Empty,
        spare_capacity: true,
    },
    ValidRow {
        number: 3,
        dst: LengthShape::Empty,
        src: LengthShape::One,
        spare_capacity: false,
    },
    ValidRow {
        number: 4,
        dst: LengthShape::Empty,
        src: LengthShape::One,
        spare_capacity: true,
    },
    ValidRow {
        number: 5,
        dst: LengthShape::Empty,
        src: LengthShape::Many,
        spare_capacity: false,
    },
    ValidRow {
        number: 6,
        dst: LengthShape::Empty,
        src: LengthShape::Many,
        spare_capacity: true,
    },
    ValidRow {
        number: 7,
        dst: LengthShape::One,
        src: LengthShape::Empty,
        spare_capacity: false,
    },
    ValidRow {
        number: 8,
        dst: LengthShape::One,
        src: LengthShape::Empty,
        spare_capacity: true,
    },
    ValidRow {
        number: 9,
        dst: LengthShape::One,
        src: LengthShape::One,
        spare_capacity: false,
    },
    ValidRow {
        number: 10,
        dst: LengthShape::One,
        src: LengthShape::One,
        spare_capacity: true,
    },
    ValidRow {
        number: 11,
        dst: LengthShape::One,
        src: LengthShape::Many,
        spare_capacity: false,
    },
    ValidRow {
        number: 12,
        dst: LengthShape::One,
        src: LengthShape::Many,
        spare_capacity: true,
    },
    ValidRow {
        number: 13,
        dst: LengthShape::Many,
        src: LengthShape::Empty,
        spare_capacity: false,
    },
    ValidRow {
        number: 14,
        dst: LengthShape::Many,
        src: LengthShape::Empty,
        spare_capacity: true,
    },
    ValidRow {
        number: 15,
        dst: LengthShape::Many,
        src: LengthShape::One,
        spare_capacity: false,
    },
    ValidRow {
        number: 16,
        dst: LengthShape::Many,
        src: LengthShape::One,
        spare_capacity: true,
    },
    ValidRow {
        number: 17,
        dst: LengthShape::Many,
        src: LengthShape::Many,
        spare_capacity: false,
    },
    ValidRow {
        number: 18,
        dst: LengthShape::Many,
        src: LengthShape::Many,
        spare_capacity: true,
    },
];

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

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next_u64() as usize % (end - start))
    }

    fn nonzero_wchar(&mut self) -> c_int {
        loop {
            let value = self.next_u64() as u32 as c_int;
            if value != 0 {
                return value;
            }
        }
    }
}

fn shaped_len(shape: LengthShape, rng: &mut Rng) -> usize {
    match shape {
        LengthShape::Empty => 0,
        LengthShape::One => 1,
        LengthShape::Many => rng.range(2, 65),
    }
}

fn random_string(len: usize, rng: &mut Rng) -> Vec<c_int> {
    let mut value = Vec::with_capacity(len + 1);
    value.extend((0..len).map(|_| rng.nonzero_wchar()));
    value.push(0);
    value
}

fn assert_call_matches(
    implementations: &Implementations,
    initial_dst: &[c_int],
    num_elem: usize,
    src: Option<&[c_int]>,
    context: &str,
) -> c_int {
    let (c_wcscat, rust_wcscat) = implementations.functions();
    let mut c_dst = initial_dst.to_vec();
    let mut rust_dst = initial_dst.to_vec();
    let src_ptr = src.map_or(std::ptr::null(), |value| value.as_ptr());

    // Each destination is an independent allocation with identical bytes.
    let c_result = unsafe { c_wcscat(c_dst.as_mut_ptr(), num_elem, src_ptr) };
    let rust_result = unsafe { rust_wcscat(rust_dst.as_mut_ptr(), num_elem, src_ptr) };

    assert_eq!(rust_result, c_result, "{context}: return value");
    assert_eq!(rust_dst, c_dst, "{context}: destination bytes");
    c_result
}

#[test]
fn valid_configuration_matrix_matches_for_randomized_inputs() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    for row in VALID_ROWS {
        for iteration in 0..256 {
            let dst_len = shaped_len(row.dst, &mut rng);
            let src_len = shaped_len(row.src, &mut rng);
            let spare = if row.spare_capacity {
                rng.range(1, 33)
            } else {
                0
            };
            let num_elem = dst_len + src_len + 1 + spare;
            let guard_len = rng.range(1, 9);
            let mut initial_dst = random_string(dst_len, &mut rng);
            initial_dst.resize_with(num_elem + guard_len, || rng.nonzero_wchar());
            let src = random_string(src_len, &mut rng);
            let context = format!(
                "CONFIGS.md row {}, iteration {iteration}, dst_len={dst_len}, \
                 src_len={src_len}, num_elem={num_elem}",
                row.number
            );

            let result = assert_call_matches(
                &implementations,
                &initial_dst,
                num_elem,
                Some(&src),
                &context,
            );
            assert_eq!(result, 0, "{context}: expected successful C result");
        }
    }
}

#[test]
fn error_row_1_null_destination_matches() {
    let implementations = Implementations::load();
    let (c_wcscat, rust_wcscat) = implementations.functions();
    let src = [1, 0];

    for num_elem in [1, 17, usize::MAX] {
        let c_result = unsafe { c_wcscat(std::ptr::null_mut(), num_elem, src.as_ptr()) };
        let rust_result = unsafe { rust_wcscat(std::ptr::null_mut(), num_elem, src.as_ptr()) };
        assert_eq!(
            rust_result, c_result,
            "ERRORS.md row 1, num_elem={num_elem}"
        );
        assert_eq!(c_result, 22);
    }
}

#[test]
fn error_row_2_zero_length_matches_and_preserves_destination() {
    let implementations = Implementations::load();
    let src = [9, 0];

    for src_value in [None, Some(src.as_slice())] {
        let result = assert_call_matches(
            &implementations,
            &[123, 456, 789],
            0,
            src_value,
            "ERRORS.md row 2",
        );
        assert_eq!(result, 22);
    }
}

#[test]
fn error_row_3_null_source_matches_and_clears_destination() {
    let implementations = Implementations::load();

    for num_elem in [1, 2, 64, usize::MAX] {
        let initial = [123, 456, 789];
        let result = assert_call_matches(
            &implementations,
            &initial,
            num_elem,
            None,
            &format!("ERRORS.md row 3, num_elem={num_elem}"),
        );
        assert_eq!(result, 22);
    }
}

#[test]
fn error_row_4_unterminated_destination_matches() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xa55a_1eaf_0bad_5eed);

    for iteration in 0..512 {
        let num_elem = rng.range(1, 129);
        let guard_len = rng.range(1, 9);
        let mut initial = Vec::with_capacity(num_elem + guard_len);
        initial.extend((0..num_elem + guard_len).map(|_| rng.nonzero_wchar()));
        let src = random_string(rng.range(0, 65), &mut rng);
        let result = assert_call_matches(
            &implementations,
            &initial,
            num_elem,
            Some(&src),
            &format!("ERRORS.md row 4, iteration {iteration}, num_elem={num_elem}"),
        );
        assert_eq!(result, 34);
    }
}

#[test]
fn error_row_5_source_exhausts_remaining_capacity_matches() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);

    for iteration in 0..512 {
        let dst_len = rng.range(0, 65);
        let remaining = rng.range(1, 65);
        let source_overage = rng.range(0, 65);
        let src_len = remaining + source_overage;
        let num_elem = dst_len + remaining;
        let guard_len = rng.range(1, 9);
        let mut initial = random_string(dst_len, &mut rng);
        initial.resize_with(num_elem + guard_len, || rng.nonzero_wchar());
        let src = random_string(src_len, &mut rng);

        let result = assert_call_matches(
            &implementations,
            &initial,
            num_elem,
            Some(&src),
            &format!(
                "ERRORS.md row 5, iteration {iteration}, dst_len={dst_len}, \
                src_len={src_len}, remaining={remaining}"
            ),
        );
        assert_eq!(result, 34);
    }
}
