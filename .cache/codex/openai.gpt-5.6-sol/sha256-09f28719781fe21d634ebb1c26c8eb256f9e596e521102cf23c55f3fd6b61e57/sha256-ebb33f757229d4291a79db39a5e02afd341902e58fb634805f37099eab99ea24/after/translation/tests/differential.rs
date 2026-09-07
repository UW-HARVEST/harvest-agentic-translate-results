use libloading::{Library, Symbol};
use std::fs;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type ToBarycentric = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

fn find_c_library(manifest_dir: &Path) -> PathBuf {
    let build_dir = manifest_dir.join("../c_src/build");
    let mut matches = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect::<Vec<_>>();
    matches.sort();
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one C shared library in {}, found {matches:?}",
        build_dir.display()
    );
    matches.remove(0)
}

fn rust_library(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .join("target")
        .join("release")
        .join("libto_barycentric_lib.so")
}

fn assert_same(
    c_fn: &Symbol<'_, ToBarycentric>,
    rust_fn: &Symbol<'_, ToBarycentric>,
    inputs: [LmVec2; 4],
    case: usize,
) {
    let c = unsafe { c_fn(inputs[0], inputs[1], inputs[2], inputs[3]) };
    let rust = unsafe { rust_fn(inputs[0], inputs[1], inputs[2], inputs[3]) };
    assert_eq!(
        [c.x.to_bits(), c.y.to_bits()],
        [rust.x.to_bits(), rust.y.to_bits()],
        "case {case}: inputs={inputs:?}, C={c:?}, Rust={rust:?}"
    );
}

fn next_u32(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

#[test]
fn configuration_1_matches_c_byte_for_byte() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = find_c_library(manifest_dir);
    let rust_path = rust_library(manifest_dir);
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }
        .unwrap_or_else(|error| panic!("cannot load {}: {error}", c_path.display()));
    let rust_library = unsafe { Library::new(&rust_path) }
        .unwrap_or_else(|error| panic!("cannot load {}: {error}", rust_path.display()));
    let c_fn: Symbol<'_, ToBarycentric> =
        unsafe { c_library.get(b"to_barycentric\0") }.expect("C symbol missing");
    let rust_fn: Symbol<'_, ToBarycentric> =
        unsafe { rust_library.get(b"to_barycentric\0") }.expect("Rust symbol missing");

    let edges = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0x7fc1_2345,
        0xffc5_4321,
    ];

    let mut case = 0;
    for &bits in &edges {
        let value = f32::from_bits(bits);
        assert_same(
            &c_fn,
            &rust_fn,
            [
                LmVec2 { x: value, y: 0.0 },
                LmVec2 { x: 0.0, y: value },
                LmVec2 { x: value, y: value },
                LmVec2 {
                    x: -value,
                    y: value,
                },
            ],
            case,
        );
        case += 1;
    }

    let geometry_edges = [
        [
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 0.0 },
        ],
        [
            LmVec2 { x: 1.0, y: 1.0 },
            LmVec2 { x: 1.0, y: 1.0 },
            LmVec2 { x: 1.0, y: 1.0 },
            LmVec2 { x: 2.0, y: 3.0 },
        ],
        [
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 1.0, y: 1.0 },
            LmVec2 { x: 2.0, y: 2.0 },
            LmVec2 { x: 3.0, y: 3.0 },
        ],
        [
            LmVec2 { x: -1.0, y: -1.0 },
            LmVec2 { x: 1.0, y: -1.0 },
            LmVec2 { x: -1.0, y: 1.0 },
            LmVec2 { x: 0.0, y: 0.0 },
        ],
    ];
    for inputs in geometry_edges {
        assert_same(&c_fn, &rust_fn, inputs, case);
        case += 1;
    }

    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..100_000 {
        let inputs = std::array::from_fn(|_| LmVec2 {
            x: f32::from_bits(next_u32(&mut state)),
            y: f32::from_bits(next_u32(&mut state)),
        });
        assert_same(&c_fn, &rust_fn, inputs, case);
        case += 1;
    }

    for _ in 0..100_000 {
        let inputs = std::array::from_fn(|_| {
            let x = (next_u32(&mut state) as i32) as f32 / 65_536.0;
            let y = (next_u32(&mut state) as i32) as f32 / 65_536.0;
            LmVec2 { x, y }
        });
        assert_same(&c_fn, &rust_fn, inputs, case);
        case += 1;
    }
}
