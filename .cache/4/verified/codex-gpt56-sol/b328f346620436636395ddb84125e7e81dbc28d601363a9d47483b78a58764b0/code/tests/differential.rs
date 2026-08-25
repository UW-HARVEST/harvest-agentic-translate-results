use libloading::Library;
use std::path::{Path, PathBuf};

const _: usize = std::mem::size_of::<to_barycentric_lib::LmVec2>();

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type ToBarycentric = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

struct Differential {
    _c_library: Library,
    _rust_library: Library,
    c_to_barycentric: ToBarycentric,
    rust_to_barycentric: ToBarycentric,
}

impl Differential {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library not found at {}; build it with CMake first",
            c_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_to_barycentric = *c_library
                .get::<ToBarycentric>(b"to_barycentric\0")
                .expect("C library is missing to_barycentric");
            let rust_to_barycentric = *rust_library
                .get::<ToBarycentric>(b"to_barycentric\0")
                .expect("Rust library is missing to_barycentric");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_to_barycentric,
                rust_to_barycentric,
            }
        }
    }

    fn assert_same(&self, case: usize, points: [LmVec2; 4]) {
        unsafe {
            let c = (self.c_to_barycentric)(points[0], points[1], points[2], points[3]);
            let rust = (self.rust_to_barycentric)(points[0], points[1], points[2], points[3]);
            assert_eq!(
                [c.x.to_bits(), c.y.to_bits()],
                [rust.x.to_bits(), rust.y.to_bits()],
                "case {case}: input bits={:?}, C={:?}, Rust={:?}",
                points.map(|point| [point.x.to_bits(), point.y.to_bits()]),
                [c.x.to_bits(), c.y.to_bits()],
                [rust.x.to_bits(), rust.y.to_bits()]
            );
        }
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("failed to locate test executable");
    let deps_dir = test_executable
        .parent()
        .expect("test executable has no parent");
    let profile_dir = deps_dir
        .parent()
        .expect("Cargo deps directory has no parent");
    let exact_candidates = [
        deps_dir.join("libto_barycentric_lib.so"),
        profile_dir.join("libto_barycentric_lib.so"),
    ];

    if let Some(path) = exact_candidates.into_iter().find(|path| path.is_file()) {
        return path;
    }

    for directory in [profile_dir, deps_dir] {
        if let Some(path) = find_hashed_library(directory) {
            return path;
        }
    }

    panic!(
        "Rust shared library was not found under {}",
        profile_dir.display()
    );
}

fn find_hashed_library(directory: &Path) -> Option<PathBuf> {
    std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("libto_barycentric_lib") && name.ends_with(".so")
                })
        })
}

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

    fn moderate(&mut self) -> f32 {
        (self.next_u32() % 8193) as f32 / 32.0 - 128.0
    }

    fn finite_vec(&mut self) -> LmVec2 {
        LmVec2 {
            x: self.moderate(),
            y: self.moderate(),
        }
    }
}

#[test]
fn finite_nondegenerate_triangles_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0x243f_6a88_85a3_08d3);

    for case in 0..4096 {
        let p1 = rng.finite_vec();
        let width = (rng.next_u32() % 1024 + 1) as f32 / 16.0;
        let height = (rng.next_u32() % 1024 + 1) as f32 / 16.0;
        let p2 = LmVec2 {
            x: p1.x + width,
            y: p1.y,
        };
        let p3 = LmVec2 {
            x: p1.x,
            y: p1.y + height,
        };
        let (u, v) = match case % 3 {
            0 => (
                (rng.next_u32() % 400) as f32 / 1000.0,
                (rng.next_u32() % 400) as f32 / 1000.0,
            ),
            1 => {
                let u = (rng.next_u32() % 1001) as f32 / 1000.0;
                (u, 1.0 - u)
            }
            _ => (
                (rng.next_u32() % 2001) as f32 / 500.0 - 1.0,
                (rng.next_u32() % 2001) as f32 / 500.0 - 1.0,
            ),
        };
        let p = LmVec2 {
            x: p1.x + v * width,
            y: p1.y + u * height,
        };
        api.assert_same(case, [p1, p2, p3, p]);
    }
}

#[test]
fn collinear_distinct_vertices_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0x1319_8a2e_0370_7344);

    for case in 0..2048 {
        let p1 = rng.finite_vec();
        let first = (rng.next_u32() % 1024 + 1) as f32 / 16.0;
        let second = first + (rng.next_u32() % 1024 + 1) as f32 / 16.0;
        let p2 = LmVec2 {
            x: p1.x + first,
            y: p1.y,
        };
        let p3 = LmVec2 {
            x: p1.x + second,
            y: p1.y,
        };
        api.assert_same(case, [p1, p2, p3, rng.finite_vec()]);
    }
}

#[test]
fn coincident_vertices_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0xa409_3822_299f_31d0);

    for case in 0..2048 {
        let p1 = rng.finite_vec();
        let other = rng.finite_vec();
        let points = match case % 3 {
            0 => [p1, p1, other, rng.finite_vec()],
            1 => [p1, other, p1, rng.finite_vec()],
            _ => [p1, p1, p1, rng.finite_vec()],
        };
        api.assert_same(case, points);
    }
}

#[test]
fn signed_zero_and_subnormal_components_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0x082e_fa98_ec4e_6c89);
    let values = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x0000_0100,
        0x8000_0100,
        0x007f_ffff,
        0x807f_ffff,
        0x0080_0000,
        0x8080_0000,
    ];

    for case in 0..4096 {
        let mut scalar = || {
            let index = rng.next_u32() as usize % values.len();
            f32::from_bits(values[index])
        };
        let points = [(); 4].map(|()| LmVec2 {
            x: scalar(),
            y: scalar(),
        });
        api.assert_same(case, points);
    }
}

#[test]
fn overflowing_finite_components_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0x4528_21e6_38d0_1377);

    for case in 0..4096 {
        let mut scalar = || {
            let sign = rng.next_u32() & 0x8000_0000;
            let mantissa = rng.next_u32() & 0x007f_ffff;
            f32::from_bits(sign | 0x7e00_0000 | mantissa)
        };
        let points = [(); 4].map(|()| LmVec2 {
            x: scalar(),
            y: scalar(),
        });
        api.assert_same(case, points);
    }
}

#[test]
fn infinite_components_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0xbe54_66cf_34e9_0c6c);

    for case in 0..4096 {
        let mut points = [(); 4].map(|()| rng.finite_vec());
        let replacements = case % 8 + 1;
        for _ in 0..replacements {
            let index = rng.next_u32() as usize % 8;
            let infinity = f32::from_bits(0x7f80_0000 | (rng.next_u32() & 0x8000_0000));
            if index % 2 == 0 {
                points[index / 2].x = infinity;
            } else {
                points[index / 2].y = infinity;
            }
        }
        api.assert_same(case, points);
    }
}

#[test]
fn nan_payloads_match() {
    let api = Differential::load();
    let mut rng = Rng::new(0xc0ac_29b7_c97c_50dd);

    for case in 0..8192 {
        let mut points = [(); 4].map(|()| rng.finite_vec());
        let replacements = case % 8 + 1;
        for replacement in 0..replacements {
            let index = rng.next_u32() as usize % 8;
            let sign = rng.next_u32() & 0x8000_0000;
            let payload = (rng.next_u32() & 0x003f_ffff).max(1);
            let quiet = if (case + replacement) % 2 == 0 {
                0x0040_0000
            } else {
                0
            };
            let nan = f32::from_bits(sign | 0x7f80_0000 | quiet | payload);
            if index % 2 == 0 {
                points[index / 2].x = nan;
            } else {
                points[index / 2].y = nan;
            }
        }
        api.assert_same(case, points);
    }
}
