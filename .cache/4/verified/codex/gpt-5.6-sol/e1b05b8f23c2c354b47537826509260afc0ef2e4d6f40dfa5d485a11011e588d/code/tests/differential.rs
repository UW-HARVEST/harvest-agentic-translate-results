use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::ptr;

type Wcscat = unsafe extern "C" fn(dst: *mut c_int, num_elem: usize, src: *const c_int) -> c_int;

struct Api {
    _library: Library,
    wcscat: Wcscat,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let wcscat = unsafe {
            *library
                .get::<Wcscat>(b"wcscat\0")
                .unwrap_or_else(|error| panic!("failed to load wcscat: {error}"))
        };
        Self {
            _library: library,
            wcscat,
        }
    }
}

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self {
            c: Api::load(&root.join("c_src/build/libtranslated_rust.so")),
            rust: Api::load(&root.join("target/debug/libwcscat_lib.so")),
        }
    }
}

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

    fn usize_in(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + self.next_u64() as usize % (end_inclusive - start + 1)
    }

    fn nonzero_i32(&mut self) -> i32 {
        let value = self.next_u64() as i32;
        if value == 0 { i32::MIN } else { value }
    }
}

fn bytes(values: &[c_int]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

fn random_nonzero_values(rng: &mut Rng, length: usize) -> Vec<c_int> {
    (0..length).map(|_| rng.nonzero_i32()).collect()
}

fn assert_valid_case(
    apis: &Apis,
    rng: &mut Rng,
    dst_length: usize,
    src_length: usize,
    spare: usize,
) {
    let num_elem = dst_length + src_length + 1 + spare;
    let mut initial_dst = random_nonzero_values(rng, num_elem + 4);
    initial_dst[dst_length] = 0;

    let mut source = random_nonzero_values(rng, src_length + 4);
    source[src_length] = 0;
    let source_before = source.clone();
    let mut c_dst = initial_dst.clone();
    let mut rust_dst = initial_dst;

    let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
    let rust_result =
        unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), num_elem, source.as_ptr()) };

    assert_eq!(c_result, 0, "C rejected a valid test case");
    assert_eq!(rust_result, c_result, "return value differs");
    assert_eq!(bytes(&rust_dst), bytes(&c_dst), "destination bytes differ");
    assert_eq!(
        bytes(&source),
        bytes(&source_before),
        "source was unexpectedly modified"
    );
}

fn run_valid_row(seed: u64, mut dimensions: impl FnMut(&mut Rng) -> (usize, usize, usize)) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let (dst_length, src_length, spare) = dimensions(&mut rng);
        assert_valid_case(&apis, &mut rng, dst_length, src_length, spare);
    }
}

#[test]
fn config_01_empty_empty_exact() {
    run_valid_row(0x0101, |_| (0, 0, 0));
}

#[test]
fn config_02_empty_empty_spare() {
    run_valid_row(0x0202, |rng| (0, 0, rng.usize_in(1, 32)));
}

#[test]
fn config_03_empty_one_exact() {
    run_valid_row(0x0303, |_| (0, 1, 0));
}

#[test]
fn config_04_empty_many_exact() {
    run_valid_row(0x0404, |rng| (0, rng.usize_in(2, 64), 0));
}

#[test]
fn config_05_empty_nonempty_spare() {
    run_valid_row(0x0505, |rng| (0, rng.usize_in(1, 64), rng.usize_in(1, 32)));
}

#[test]
fn config_06_one_empty_exact() {
    run_valid_row(0x0606, |_| (1, 0, 0));
}

#[test]
fn config_07_many_empty_exact() {
    run_valid_row(0x0707, |rng| (rng.usize_in(2, 64), 0, 0));
}

#[test]
fn config_08_nonempty_empty_spare() {
    run_valid_row(0x0808, |rng| (rng.usize_in(1, 64), 0, rng.usize_in(1, 32)));
}

#[test]
fn config_09_one_one_exact() {
    run_valid_row(0x0909, |_| (1, 1, 0));
}

#[test]
fn config_10_one_many_exact() {
    run_valid_row(0x1010, |rng| (1, rng.usize_in(2, 64), 0));
}

#[test]
fn config_11_many_one_exact() {
    run_valid_row(0x1111, |rng| (rng.usize_in(2, 64), 1, 0));
}

#[test]
fn config_12_many_many_exact() {
    run_valid_row(0x1212, |rng| (rng.usize_in(2, 64), rng.usize_in(2, 64), 0));
}

#[test]
fn config_13_nonempty_nonempty_spare() {
    run_valid_row(0x1313, |rng| {
        (
            rng.usize_in(1, 64),
            rng.usize_in(1, 64),
            rng.usize_in(1, 32),
        )
    });
}

#[test]
fn error_01_null_destination() {
    let apis = Apis::load();
    let source = [17, 0];
    for source_ptr in [source.as_ptr(), ptr::null()] {
        for num_elem in [1, 7, usize::MAX] {
            let c_result = unsafe { (apis.c.wcscat)(ptr::null_mut(), num_elem, source_ptr) };
            let rust_result = unsafe { (apis.rust.wcscat)(ptr::null_mut(), num_elem, source_ptr) };
            assert_eq!(rust_result, c_result);
            assert_eq!(c_result, 22);
        }
    }
}

#[test]
fn error_02_zero_length() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe202);
    for _ in 0..128 {
        let length = rng.usize_in(1, 64);
        let initial = random_nonzero_values(&mut rng, length);
        let source = [rng.nonzero_i32(), 0];
        let mut c_dst = initial.clone();
        let mut rust_dst = initial;
        let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), 0, source.as_ptr()) };
        let rust_result = unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), 0, source.as_ptr()) };
        assert_eq!(rust_result, c_result);
        assert_eq!(c_result, 22);
        assert_eq!(bytes(&rust_dst), bytes(&c_dst));
    }
}

#[test]
fn error_03_null_destination_and_zero_length() {
    let apis = Apis::load();
    let source = [31, 0];
    let c_result = unsafe { (apis.c.wcscat)(ptr::null_mut(), 0, source.as_ptr()) };
    let rust_result = unsafe { (apis.rust.wcscat)(ptr::null_mut(), 0, source.as_ptr()) };
    assert_eq!(rust_result, c_result);
    assert_eq!(c_result, 22);
}

#[test]
fn generic_zero_length_precedes_null_source() {
    let apis = Apis::load();
    let initial = [41, 42, 43];
    let mut c_dst = initial;
    let mut rust_dst = initial;
    let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), 0, ptr::null()) };
    let rust_result = unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), 0, ptr::null()) };
    assert_eq!(rust_result, c_result);
    assert_eq!(c_result, 22);
    assert_eq!(bytes(&rust_dst), bytes(&c_dst));
    assert_eq!(c_dst, initial);
}

#[test]
fn error_04_null_source() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe404);
    for _ in 0..128 {
        let length = rng.usize_in(1, 64);
        let initial = random_nonzero_values(&mut rng, length + 4);
        let mut c_dst = initial.clone();
        let mut rust_dst = initial;
        let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), length, ptr::null()) };
        let rust_result = unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), length, ptr::null()) };
        assert_eq!(rust_result, c_result);
        assert_eq!(c_result, 22);
        assert_eq!(c_dst[0], 0);
        assert_eq!(bytes(&rust_dst), bytes(&c_dst));
    }
}

#[test]
fn error_05_unterminated_destination() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe505);
    for _ in 0..128 {
        let num_elem = rng.usize_in(1, 64);
        let initial = random_nonzero_values(&mut rng, num_elem + 4);
        let source = [rng.nonzero_i32(), 0];
        let mut c_dst = initial.clone();
        let mut rust_dst = initial;
        let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
        let rust_result =
            unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
        assert_eq!(rust_result, c_result);
        assert_eq!(c_result, 34);
        assert_eq!(c_dst[0], 0);
        assert_eq!(bytes(&rust_dst), bytes(&c_dst));
    }
}

#[test]
fn error_06_source_does_not_fit() {
    let apis = Apis::load();
    let mut rng = Rng::new(0xe606);
    for _ in 0..128 {
        let dst_length = rng.usize_in(0, 32);
        let available = rng.usize_in(1, 32);
        let num_elem = dst_length + available;
        let source_length = available + rng.usize_in(0, 32);

        let mut initial = random_nonzero_values(&mut rng, num_elem + 4);
        initial[dst_length] = 0;
        let mut source = random_nonzero_values(&mut rng, source_length + 1);
        source[source_length] = 0;
        let mut c_dst = initial.clone();
        let mut rust_dst = initial;

        let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
        let rust_result =
            unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
        assert_eq!(rust_result, c_result);
        assert_eq!(c_result, 34);
        assert_eq!(c_dst[0], 0);
        assert_eq!(bytes(&rust_dst), bytes(&c_dst));
    }
}

#[test]
fn generic_large_allocated_length() {
    let apis = Apis::load();
    let num_elem = 65_536;
    let mut c_dst = vec![0; num_elem + 4];
    let mut rust_dst = c_dst.clone();
    let source = [0];
    let c_result = unsafe { (apis.c.wcscat)(c_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
    let rust_result =
        unsafe { (apis.rust.wcscat)(rust_dst.as_mut_ptr(), num_elem, source.as_ptr()) };
    assert_eq!(rust_result, c_result);
    assert_eq!(bytes(&rust_dst), bytes(&c_dst));
}
