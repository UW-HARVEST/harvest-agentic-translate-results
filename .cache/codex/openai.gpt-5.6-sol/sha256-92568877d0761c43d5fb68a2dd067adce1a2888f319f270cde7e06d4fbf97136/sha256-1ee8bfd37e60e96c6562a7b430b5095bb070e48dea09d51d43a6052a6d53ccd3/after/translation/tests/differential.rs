use libloading::{Library, Symbol};
use std::{
    env,
    ffi::c_int,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[repr(C)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

type Premultiply = unsafe extern "C" fn(*mut CpImage);

struct Differential {
    c: Library,
    rust: Library,
}

impl Differential {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust release shared library: {}",
            rust_path.display()
        );

        // SAFETY: Both paths are build products controlled by this test.
        let c = unsafe { Library::new(c_path) }.expect("load C shared library");
        // SAFETY: Both paths are build products controlled by this test.
        let rust = unsafe { Library::new(rust_path) }.expect("load Rust shared library");
        Self { c, rust }
    }

    fn compare(&self, w: c_int, h: c_int, input: &[[u8; 4]]) {
        let mut c_pixels = input.to_vec();
        let mut rust_pixels = input.to_vec();
        let mut c_image = image(w, h, &mut c_pixels);
        let mut rust_image = image(w, h, &mut rust_pixels);

        // SAFETY: The symbol signature matches lib.h, and each buffer contains
        // enough pixels for the representable positive extent used by callers.
        unsafe {
            let c_fn: Symbol<Premultiply> =
                self.c.get(b"premultiply\0").expect("resolve C premultiply");
            let rust_fn: Symbol<Premultiply> = self
                .rust
                .get(b"premultiply\0")
                .expect("resolve Rust premultiply");
            c_fn(&mut c_image);
            rust_fn(&mut rust_image);
        }

        assert_eq!(
            rust_pixels, c_pixels,
            "byte mismatch for dimensions ({w}, {h})"
        );
    }

    fn compare_null_pixels(&self, w: c_int, h: c_int) {
        let mut c_image = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };
        let mut rust_image = CpImage {
            w,
            h,
            pix: std::ptr::null_mut(),
        };

        // SAFETY: These dimensions make the C loop execute zero times, so the
        // null pixel pointers are never dereferenced.
        unsafe {
            let c_fn: Symbol<Premultiply> =
                self.c.get(b"premultiply\0").expect("resolve C premultiply");
            let rust_fn: Symbol<Premultiply> = self
                .rust
                .get(b"premultiply\0")
                .expect("resolve Rust premultiply");
            c_fn(&mut c_image);
            rust_fn(&mut rust_image);
        }
    }
}

fn image(w: c_int, h: c_int, pixels: &mut [[u8; 4]]) -> CpImage {
    CpImage {
        w,
        h,
        pix: pixels.as_mut_ptr().cast(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("read C build entry").path())
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("lib"))
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
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    target_dir.join("release/libpremultiply_lib.so")
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn range(&mut self, start: u32, end: u32) -> u32 {
        start + self.next_u32() % (end - start)
    }

    fn pixels(&mut self, count: usize) -> Vec<[u8; 4]> {
        (0..count).map(|_| self.next_u32().to_le_bytes()).collect()
    }
}

#[test]
fn c1_zero_extent() {
    let differential = Differential::load();
    let mut rng = Rng::new(0xd1ff_e2e7_1a1c_0001);
    differential.compare_null_pixels(0, 0);
    for _ in 0..512 {
        let magnitude = rng.range(1, 100_001) as c_int;
        let signed = if rng.next_u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        };
        if rng.next_u32() & 1 == 0 {
            differential.compare_null_pixels(0, signed);
        } else {
            differential.compare_null_pixels(signed, 0);
        }
    }
}

#[test]
fn c2_negative_extent() {
    let differential = Differential::load();
    let mut rng = Rng::new(0x8bf5_4d3c_27a1_9001);
    for _ in 0..512 {
        let magnitude_w = rng.range(1, 129) as c_int;
        let magnitude_h = rng.range(1, 129) as c_int;
        let dimensions = if rng.next_u32() & 1 == 0 {
            (-magnitude_w, magnitude_h)
        } else {
            (magnitude_w, -magnitude_h)
        };
        differential.compare(dimensions.0, dimensions.1, &rng.pixels(8));
    }
}

#[test]
fn c3_one_pixel() {
    let differential = Differential::load();
    let mut rng = Rng::new(0x0ddc_0ffe_e15e_ba11);
    for iteration in 0..2048 {
        let mut pixel = rng.next_u32().to_le_bytes();
        pixel[3] = match iteration % 8 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 127,
            4 => 128,
            5 => 253,
            6 => 254,
            _ => 255,
        };
        differential.compare(1, 1, &[pixel]);
    }
}

#[test]
fn c4_many_positive_pixels() {
    let differential = Differential::load();
    let mut rng = Rng::new(0xd1ff_e2e7_1a1c_0004);
    for _ in 0..512 {
        let w = rng.range(1, 33) as c_int;
        let h = rng.range(1, 33) as c_int;
        differential.compare(w, h, &rng.pixels((w * h) as usize));
    }
}

#[test]
fn c5_two_negative_dimensions() {
    let differential = Differential::load();
    let mut rng = Rng::new(0xd1ff_e2e7_1a1c_0005);
    for _ in 0..512 {
        let w = rng.range(1, 33) as c_int;
        let h = rng.range(1, 33) as c_int;
        differential.compare(-w, -h, &rng.pixels((w * h) as usize));
    }
}

#[test]
fn g3_zero_extent_accepts_null_pixels() {
    let differential = Differential::load();
    for &(w, h) in &[(0, 0), (0, 1), (1, 0), (0, -1), (-1, 0)] {
        differential.compare_null_pixels(w, h);
    }
}

#[test]
fn g4_oversized_width_with_zero_height() {
    let differential = Differential::load();
    differential.compare_null_pixels(c_int::MAX, 0);
}

#[test]
fn g1_and_g2_null_dereferences_match_c_signal() {
    use std::os::unix::process::ExitStatusExt;

    for case in ["null_image", "null_pixels"] {
        let c_status = run_crash_probe("c", case);
        let rust_status = run_crash_probe("rust", case);
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different termination signal for {case}"
        );
        assert_eq!(
            c_status.signal(),
            Some(11),
            "C did not terminate with SIGSEGV for {case}: {c_status}"
        );
    }
}

fn run_crash_probe(library: &str, case: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().expect("locate integration test executable"))
        .arg("--exact")
        .arg("crash_probe")
        .arg("--nocapture")
        .env("PREMULTIPLY_CRASH_LIBRARY", library)
        .env("PREMULTIPLY_CRASH_CASE", case)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run crash probe")
}

#[test]
fn crash_probe() {
    let Some(library) = env::var_os("PREMULTIPLY_CRASH_LIBRARY") else {
        return;
    };
    let case = env::var("PREMULTIPLY_CRASH_CASE").expect("crash case");
    let path = match library.to_str().expect("UTF-8 library selector") {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library selector: {other}"),
    };

    // SAFETY: This test intentionally supplies invalid pointers in a child
    // process to compare the libraries' externally observable crash behavior.
    unsafe {
        let loaded = Library::new(Path::new(&path)).expect("load crash-probe library");
        let premultiply: Symbol<Premultiply> =
            loaded.get(b"premultiply\0").expect("resolve premultiply");
        match case.as_str() {
            "null_image" => premultiply(std::ptr::null_mut()),
            "null_pixels" => {
                let mut image = CpImage {
                    w: 1,
                    h: 1,
                    pix: std::ptr::null_mut(),
                };
                premultiply(&mut image);
            }
            other => panic!("unknown crash case: {other}"),
        }
    }

    panic!("invalid call unexpectedly returned");
}
