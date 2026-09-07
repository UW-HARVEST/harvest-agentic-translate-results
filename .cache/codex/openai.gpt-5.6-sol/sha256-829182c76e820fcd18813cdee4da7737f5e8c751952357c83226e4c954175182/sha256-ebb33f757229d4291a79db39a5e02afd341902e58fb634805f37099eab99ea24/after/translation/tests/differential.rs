use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::ptr;

type Wcscat = unsafe extern "C" fn(*mut c_int, usize, *const c_int) -> c_int;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_wcscat: Wcscat,
    rust_wcscat: Wcscat,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C shared library missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library missing: {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_wcscat = *c
                .get::<Wcscat>(b"wcscat\0")
                .unwrap_or_else(|error| panic!("C wcscat symbol missing: {error}"));
            let rust_wcscat = *rust
                .get::<Wcscat>(b"wcscat\0")
                .unwrap_or_else(|error| panic!("Rust wcscat symbol missing: {error}"));
            Self {
                _c: c,
                _rust: rust,
                c_wcscat,
                rust_wcscat,
            }
        }
    }

    fn compare(
        &self,
        initial_destination: &[c_int],
        num_elem: usize,
        source: &[c_int],
    ) -> (c_int, Vec<c_int>) {
        let mut c_destination = initial_destination.to_vec();
        let mut rust_destination = initial_destination.to_vec();

        let c_result =
            unsafe { (self.c_wcscat)(c_destination.as_mut_ptr(), num_elem, source.as_ptr()) };
        let rust_result =
            unsafe { (self.rust_wcscat)(rust_destination.as_mut_ptr(), num_elem, source.as_ptr()) };

        assert_eq!(rust_result, c_result, "return-code mismatch");
        assert_eq!(
            rust_destination, c_destination,
            "destination bytes differ for num_elem={num_elem}"
        );
        (c_result, c_destination)
    }

    fn compare_null_destination(&self, num_elem: usize, source: &[c_int]) -> c_int {
        let c_result = unsafe { (self.c_wcscat)(ptr::null_mut(), num_elem, source.as_ptr()) };
        let rust_result = unsafe { (self.rust_wcscat)(ptr::null_mut(), num_elem, source.as_ptr()) };
        assert_eq!(rust_result, c_result, "return-code mismatch");
        c_result
    }

    fn compare_null_source(
        &self,
        initial_destination: &[c_int],
        num_elem: usize,
    ) -> (c_int, Vec<c_int>) {
        let mut c_destination = initial_destination.to_vec();
        let mut rust_destination = initial_destination.to_vec();

        let c_result =
            unsafe { (self.c_wcscat)(c_destination.as_mut_ptr(), num_elem, ptr::null()) };
        let rust_result =
            unsafe { (self.rust_wcscat)(rust_destination.as_mut_ptr(), num_elem, ptr::null()) };

        assert_eq!(rust_result, c_result, "return-code mismatch");
        assert_eq!(rust_destination, c_destination, "destination bytes differ");
        (c_result, c_destination)
    }
}

fn c_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("C_WCSCAT_SO") {
        return path.into();
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.starts_with("libharvest-work-"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_WCSCAT_SO") {
        return path.into();
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libwcscat_lib.so")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must have a parent")
        .to_path_buf()
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

    fn usize_in(&mut self, start: usize, end_exclusive: usize) -> usize {
        assert!(start < end_exclusive);
        start + self.next_u64() as usize % (end_exclusive - start)
    }

    fn nonzero_wchar(&mut self) -> c_int {
        let value = self.next_u64() as u32 as c_int;
        if value == 0 { c_int::MIN } else { value }
    }

    fn nonzero_values(&mut self, length: usize) -> Vec<c_int> {
        (0..length).map(|_| self.nonzero_wchar()).collect()
    }
}

fn terminated(mut values: Vec<c_int>) -> Vec<c_int> {
    values.push(0);
    values
}

fn destination_with_tail(
    rng: &mut Rng,
    prefix: &[c_int],
    num_elem: usize,
    allocation_len: usize,
) -> Vec<c_int> {
    assert!(prefix.len() <= num_elem);
    assert!(num_elem <= allocation_len);
    let mut destination = rng.nonzero_values(allocation_len);
    destination[..prefix.len()].copy_from_slice(prefix);
    destination
}

// CONFIGS.md row 1.
#[test]
fn config_01_empty_destination_empty_source_minimum_buffer() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0101_5eed_cafe_f00d);
    for _ in 0..256 {
        let allocation_len = rng.usize_in(1, 17);
        let destination = destination_with_tail(&mut rng, &[0], 1, allocation_len);
        let source = [0];
        let (result, output) = libraries.compare(&destination, 1, &source);
        assert_eq!(result, 0);
        assert_eq!(output[0], 0);
        assert_eq!(&output[1..], &destination[1..]);
    }
}

// CONFIGS.md row 2.
#[test]
fn config_02_empty_destination_nonempty_source_with_spare_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0202_5eed_cafe_f00d);
    for _ in 0..256 {
        let source_len = rng.usize_in(1, 33);
        let spare = rng.usize_in(1, 17);
        let source = terminated(rng.nonzero_values(source_len));
        let num_elem = source.len() + spare;
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination = destination_with_tail(&mut rng, &[0], num_elem, allocation_len);
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 0);
        assert_eq!(&output[..source.len()], source.as_slice());
        assert_eq!(&output[source.len()..], &destination[source.len()..]);
    }
}

// CONFIGS.md row 3.
#[test]
fn config_03_empty_destination_nonempty_source_exact_fit() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0303_5eed_cafe_f00d);
    for _ in 0..256 {
        let source_len = rng.usize_in(1, 49);
        let source = terminated(rng.nonzero_values(source_len));
        let num_elem = source.len();
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination = destination_with_tail(&mut rng, &[0], num_elem, allocation_len);
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 0);
        assert_eq!(&output[..num_elem], source.as_slice());
        assert_eq!(&output[num_elem..], &destination[num_elem..]);
    }
}

// CONFIGS.md row 4.
#[test]
fn config_04_nonempty_destination_empty_source() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0404_5eed_cafe_f00d);
    for iteration in 0..256 {
        let destination_len = rng.usize_in(1, 49);
        let spare_after_nul = if iteration % 2 == 0 {
            0
        } else {
            rng.usize_in(1, 17)
        };
        let num_elem = destination_len + 1 + spare_after_nul;
        let prefix = terminated(rng.nonzero_values(destination_len));
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination = destination_with_tail(&mut rng, &prefix, num_elem, allocation_len);
        let source = [0];
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 0);
        assert_eq!(output, destination);
    }
}

// CONFIGS.md row 5.
#[test]
fn config_05_nonempty_destination_nonempty_source_with_spare_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0505_5eed_cafe_f00d);
    for _ in 0..256 {
        let destination_len = rng.usize_in(1, 33);
        let source_len = rng.usize_in(1, 33);
        let spare = rng.usize_in(1, 17);
        let destination_prefix = terminated(rng.nonzero_values(destination_len));
        let source = terminated(rng.nonzero_values(source_len));
        let num_elem = destination_len + source.len() + spare;
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination =
            destination_with_tail(&mut rng, &destination_prefix, num_elem, allocation_len);
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 0);
        assert_eq!(
            &output[destination_len..destination_len + source.len()],
            source.as_slice()
        );
        assert_eq!(
            &output[destination_len + source.len()..],
            &destination[destination_len + source.len()..]
        );
    }
}

// CONFIGS.md row 6.
#[test]
fn config_06_nonempty_destination_nonempty_source_exact_fit() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0606_5eed_cafe_f00d);
    for _ in 0..256 {
        let destination_len = rng.usize_in(1, 49);
        let source_len = rng.usize_in(1, 49);
        let destination_prefix = terminated(rng.nonzero_values(destination_len));
        let source = terminated(rng.nonzero_values(source_len));
        let num_elem = destination_len + source.len();
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination =
            destination_with_tail(&mut rng, &destination_prefix, num_elem, allocation_len);
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 0);
        assert_eq!(
            &output[destination_len..destination_len + source.len()],
            source.as_slice()
        );
        assert_eq!(&output[num_elem..], &destination[num_elem..]);
    }
}

// ERRORS.md row 1.
#[test]
fn error_01_null_destination() {
    let libraries = Libraries::load();
    let source = [0];
    assert_eq!(libraries.compare_null_destination(8, &source), 22);
}

// ERRORS.md row 2.
#[test]
fn error_02_zero_length() {
    let libraries = Libraries::load();
    let destination = [11, 22, 33, 44];
    let source = [0];
    let (result, output) = libraries.compare(&destination, 0, &source);
    assert_eq!(result, 22);
    assert_eq!(output, destination);
}

// ERRORS.md row 3.
#[test]
fn error_03_null_source() {
    let libraries = Libraries::load();
    let destination = [11, 22, 0, 44];
    let (result, output) = libraries.compare_null_source(&destination, destination.len());
    assert_eq!(result, 22);
    assert_eq!(output, [0, 22, 0, 44]);
}

// ERRORS.md row 4.
#[test]
fn error_04_destination_without_nul() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0e04_5eed_cafe_f00d);
    for _ in 0..256 {
        let num_elem = rng.usize_in(1, 65);
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination = rng.nonzero_values(allocation_len);
        let source = [0];
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 34);
        assert_eq!(output[0], 0);
        assert_eq!(&output[1..], &destination[1..]);
    }
}

// ERRORS.md row 5.
#[test]
fn error_05_source_does_not_fit() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x0e05_5eed_cafe_f00d);
    for _ in 0..256 {
        let destination_len = rng.usize_in(0, 33);
        let remaining = rng.usize_in(1, 33);
        let destination_prefix = terminated(rng.nonzero_values(destination_len));
        let num_elem = destination_len + remaining;
        let allocation_len = num_elem + rng.usize_in(0, 9);
        let destination =
            destination_with_tail(&mut rng, &destination_prefix, num_elem, allocation_len);
        let extra_source_elements = rng.usize_in(0, 17);
        let source = terminated(rng.nonzero_values(remaining + extra_source_elements));
        let (result, output) = libraries.compare(&destination, num_elem, &source);
        assert_eq!(result, 34);
        assert_eq!(output[0], 0);
        if destination_len > 0 {
            assert_eq!(
                &output[1..destination_len],
                &destination[1..destination_len]
            );
        }
        for offset in 0..remaining {
            let index = destination_len + offset;
            let expected = if index == 0 { 0 } else { source[offset] };
            assert_eq!(output[index], expected);
        }
        assert_eq!(&output[num_elem..], &destination[num_elem..]);
    }
}

// ERRORS.md row 6.
#[test]
fn error_06_null_destination_with_size_max() {
    let libraries = Libraries::load();
    let source = [0];
    assert_eq!(libraries.compare_null_destination(usize::MAX, &source), 22);
}
