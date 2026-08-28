#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fmt::Debug;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct R {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Circle {
    p: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Capsule {
    a: V,
    b: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Cache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Proxy {
    radius: f32,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Sv {
    sA: V,
    sB: V,
    p: V,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: f32,
    count: c_int,
}

const ZERO_V: V = V { x: 0.0, y: 0.0 };
const ZERO_SV: Sv = Sv {
    sA: ZERO_V,
    sB: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    iA: 0,
    iB: 0,
};

fn simplex(count: c_int, a: V, b: V, c: V) -> Simplex {
    Simplex {
        a: Sv { p: a, ..ZERO_SV },
        b: Sv { p: b, ..ZERO_SV },
        c: Sv { p: c, ..ZERO_SV },
        d: ZERO_SV,
        div: 1.0,
        count,
    }
}

type FnV = unsafe extern "C" fn(f32, f32) -> V;
type FnVs = unsafe extern "C" fn(V, f32) -> V;
type FnVv = unsafe extern "C" fn(V, V) -> V;
type FnVvv = unsafe extern "C" fn(V, V, V) -> V;
type FnVf = unsafe extern "C" fn(V) -> f32;
type FnVvf = unsafe extern "C" fn(V, V) -> f32;
type FnVtoV = unsafe extern "C" fn(V) -> V;
type FnR = unsafe extern "C" fn() -> R;
type FnX = unsafe extern "C" fn() -> X;
type FnRv = unsafe extern "C" fn(R, V) -> V;
type FnXv = unsafe extern "C" fn(X, V) -> V;
type FnBb = unsafe extern "C" fn(*mut V, *mut Aabb);
type FnProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> f32;
type FnSimplex = unsafe extern "C" fn(*mut Simplex);
type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
type FnSupport = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
type FnGjk = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const X,
    *const c_void,
    c_int,
    *const X,
    *mut V,
    *mut V,
    c_int,
    *mut c_int,
    *mut Cache,
) -> f32;
type FnWrapper =
    unsafe extern "C" fn(i8, *mut V, *mut V, f32, f32, f32, f32, f32, f32, f32, f32, f32);

struct Api {
    _library: Library,
    c2V: FnV,
    c2Mulvs: FnVs,
    c2Maxv: FnVv,
    c2Minv: FnVv,
    c2Clampv: FnVvv,
    c2Sub: FnVv,
    c2Dot: FnVvf,
    c2RotIdentity: FnR,
    c2xIdentity: FnX,
    c2BBVerts: FnBb,
    c2MakeProxy: FnProxy,
    c2Len: FnVf,
    c2Det2: FnVvf,
    c2GJKSimplexMetric: FnSimplexF,
    c2Mulrv: FnRv,
    c2Add: FnVv,
    c2Mulxv: FnXv,
    c22: FnSimplex,
    c23: FnSimplex,
    c2Neg: FnVtoV,
    c2Skew: FnVtoV,
    c2CCW90: FnVtoV,
    c2D: FnSimplexV,
    c2Support: FnSupport,
    c2Witness: FnWitness,
    c2Div: FnVs,
    c2Norm: FnVtoV,
    c2L: FnSimplexV,
    c2MulrvT: FnRv,
    c2GJK: FnGjk,
    gjk_cache: FnWrapper,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }.unwrap()
            };
        }
        Self {
            c2V: symbol!("c2V", FnV),
            c2Mulvs: symbol!("c2Mulvs", FnVs),
            c2Maxv: symbol!("c2Maxv", FnVv),
            c2Minv: symbol!("c2Minv", FnVv),
            c2Clampv: symbol!("c2Clampv", FnVvv),
            c2Sub: symbol!("c2Sub", FnVv),
            c2Dot: symbol!("c2Dot", FnVvf),
            c2RotIdentity: symbol!("c2RotIdentity", FnR),
            c2xIdentity: symbol!("c2xIdentity", FnX),
            c2BBVerts: symbol!("c2BBVerts", FnBb),
            c2MakeProxy: symbol!("c2MakeProxy", FnProxy),
            c2Len: symbol!("c2Len", FnVf),
            c2Det2: symbol!("c2Det2", FnVvf),
            c2GJKSimplexMetric: symbol!("c2GJKSimplexMetric", FnSimplexF),
            c2Mulrv: symbol!("c2Mulrv", FnRv),
            c2Add: symbol!("c2Add", FnVv),
            c2Mulxv: symbol!("c2Mulxv", FnXv),
            c22: symbol!("c22", FnSimplex),
            c23: symbol!("c23", FnSimplex),
            c2Neg: symbol!("c2Neg", FnVtoV),
            c2Skew: symbol!("c2Skew", FnVtoV),
            c2CCW90: symbol!("c2CCW90", FnVtoV),
            c2D: symbol!("c2D", FnSimplexV),
            c2Support: symbol!("c2Support", FnSupport),
            c2Witness: symbol!("c2Witness", FnWitness),
            c2Div: symbol!("c2Div", FnVs),
            c2Norm: symbol!("c2Norm", FnVtoV),
            c2L: symbol!("c2L", FnSimplexV),
            c2MulrvT: symbol!("c2MulrvT", FnRv),
            c2GJK: symbol!("c2GJK", FnGjk),
            gjk_cache: symbol!("gjk_cache", FnWrapper),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_apis() -> (Api, Api) {
    let root = manifest_dir();
    let c_path = root.join("../c_src/build/libharvest-work-GriyDD.so");
    let rust_path = root.join("target/release/libgjk_cache_lib.so");
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust cdylib: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn assert_bits_eq(label: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{label}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_bytes_eq<T: Copy + Debug>(label: &str, c: &T, rust: &T) {
    let c_bytes =
        unsafe { std::slice::from_raw_parts((c as *const T).cast::<u8>(), size_of::<T>()) };
    let rust_bytes =
        unsafe { std::slice::from_raw_parts((rust as *const T).cast::<u8>(), size_of::<T>()) };
    assert_eq!(c_bytes, rust_bytes, "{label}: C={c:?}, Rust={rust:?}");
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn finite(&mut self) -> f32 {
        let magnitude = (self.next_u32() % 200_001) as f32 / 97.0;
        if self.next_u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }

    fn moderate(&mut self) -> f32 {
        let magnitude = (self.next_u32() % 20_001) as f32 / 211.0;
        if self.next_u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 10_000 + 1) as f32 / 317.0
    }

    fn vector(&mut self) -> V {
        V {
            x: self.moderate(),
            y: self.moderate(),
        }
    }
}

#[test]
fn dynamic_symbol_surface_matches() {
    let root = manifest_dir();
    let c_path = root.join("../c_src/build/libharvest-work-GriyDD.so");
    let rust_path = root.join("target/release/libgjk_cache_lib.so");
    let symbols = |path: &Path| {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut names: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let _address = fields.next()?;
                let kind = fields.next()?;
                let name = fields.next()?;
                matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_owned())
            })
            .collect();
        names.sort();
        names
    };
    let c_symbols = symbols(&c_path);
    let rust_symbols = symbols(&rust_path);
    let missing: Vec<_> = c_symbols
        .iter()
        .filter(|name| !rust_symbols.contains(name))
        .collect();
    assert!(missing.is_empty(), "missing Rust symbols: {missing:?}");

    // Loading the APIs also validates every expected function ABI lookup.
    let _ = load_apis();
}

#[test]
fn randomized_value_proxy_and_support_functions_match() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x5eed_1234_cafe_f00d);

    unsafe {
        assert_bytes_eq(
            "c2RotIdentity",
            &(c.c2RotIdentity)(),
            &(rust.c2RotIdentity)(),
        );
        assert_bytes_eq("c2xIdentity", &(c.c2xIdentity)(), &(rust.c2xIdentity)());

        for case in 0..256 {
            let a = rng.vector();
            let b = rng.vector();
            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            let scalar = rng.moderate();
            let nonzero = if scalar == 0.0 { 1.0 } else { scalar };
            let r = R {
                c: rng.moderate(),
                s: rng.moderate(),
            };
            let x = X { p: rng.vector(), r };

            assert_bytes_eq(
                &format!("c2V case {case}"),
                &(c.c2V)(a.x, a.y),
                &(rust.c2V)(a.x, a.y),
            );
            assert_bytes_eq(
                &format!("c2Mulvs case {case}"),
                &(c.c2Mulvs)(a, scalar),
                &(rust.c2Mulvs)(a, scalar),
            );
            assert_bytes_eq(
                &format!("c2Maxv case {case}"),
                &(c.c2Maxv)(a, b),
                &(rust.c2Maxv)(a, b),
            );
            assert_bytes_eq(
                &format!("c2Minv case {case}"),
                &(c.c2Minv)(a, b),
                &(rust.c2Minv)(a, b),
            );
            assert_bytes_eq(
                &format!("c2Clampv case {case}"),
                &(c.c2Clampv)(a, lo, hi),
                &(rust.c2Clampv)(a, lo, hi),
            );
            assert_bytes_eq(
                &format!("c2Clampv reversed case {case}"),
                &(c.c2Clampv)(a, hi, lo),
                &(rust.c2Clampv)(a, hi, lo),
            );
            assert_bytes_eq(
                &format!("c2Sub case {case}"),
                &(c.c2Sub)(a, b),
                &(rust.c2Sub)(a, b),
            );
            assert_bits_eq(
                &format!("c2Dot case {case}"),
                (c.c2Dot)(a, b),
                (rust.c2Dot)(a, b),
            );
            assert_bits_eq(&format!("c2Len case {case}"), (c.c2Len)(a), (rust.c2Len)(a));
            assert_bits_eq(
                &format!("c2Det2 case {case}"),
                (c.c2Det2)(a, b),
                (rust.c2Det2)(a, b),
            );
            assert_bytes_eq(
                &format!("c2Mulrv case {case}"),
                &(c.c2Mulrv)(r, a),
                &(rust.c2Mulrv)(r, a),
            );
            assert_bytes_eq(
                &format!("c2Add case {case}"),
                &(c.c2Add)(a, b),
                &(rust.c2Add)(a, b),
            );
            assert_bytes_eq(
                &format!("c2Mulxv case {case}"),
                &(c.c2Mulxv)(x, a),
                &(rust.c2Mulxv)(x, a),
            );
            assert_bytes_eq(
                &format!("c2Neg case {case}"),
                &(c.c2Neg)(a),
                &(rust.c2Neg)(a),
            );
            assert_bytes_eq(
                &format!("c2Skew case {case}"),
                &(c.c2Skew)(a),
                &(rust.c2Skew)(a),
            );
            assert_bytes_eq(
                &format!("c2CCW90 case {case}"),
                &(c.c2CCW90)(a),
                &(rust.c2CCW90)(a),
            );
            assert_bytes_eq(
                &format!("c2Div case {case}"),
                &(c.c2Div)(a, nonzero),
                &(rust.c2Div)(a, nonzero),
            );
            if a.x != 0.0 || a.y != 0.0 {
                assert_bytes_eq(
                    &format!("c2Norm case {case}"),
                    &(c.c2Norm)(a),
                    &(rust.c2Norm)(a),
                );
            }
            assert_bytes_eq(
                &format!("c2MulrvT case {case}"),
                &(c.c2MulrvT)(r, a),
                &(rust.c2MulrvT)(r, a),
            );

            let mut c_bb_out = [V { x: 99.0, y: -99.0 }; 4];
            let mut rust_bb_out = c_bb_out;
            let mut c_bb = Aabb { min: a, max: b };
            let mut rust_bb = c_bb;
            (c.c2BBVerts)(c_bb_out.as_mut_ptr(), &mut c_bb);
            (rust.c2BBVerts)(rust_bb_out.as_mut_ptr(), &mut rust_bb);
            assert_bytes_eq(&format!("c2BBVerts case {case}"), &c_bb_out, &rust_bb_out);

            let circle = Circle {
                p: a,
                r: rng.moderate(),
            };
            let bb = Aabb { min: lo, max: hi };
            let capsule = Capsule {
                a,
                b,
                r: rng.moderate(),
            };
            for (shape_type, shape_ptr) in [
                (0, (&circle as *const Circle).cast::<c_void>()),
                (1, (&bb as *const Aabb).cast::<c_void>()),
                (2, (&capsule as *const Capsule).cast::<c_void>()),
            ] {
                let canary = Proxy {
                    radius: f32::from_bits(0x7fc0_1234),
                    count: 0x1234_5678,
                    verts: [V {
                        x: f32::from_bits(0x7fc0_5678),
                        y: -123.25,
                    }; 8],
                };
                let mut c_proxy = canary;
                let mut rust_proxy = canary;
                (c.c2MakeProxy)(shape_ptr, shape_type, &mut c_proxy);
                (rust.c2MakeProxy)(shape_ptr, shape_type, &mut rust_proxy);
                assert_bytes_eq(
                    &format!("c2MakeProxy type {shape_type} case {case}"),
                    &c_proxy,
                    &rust_proxy,
                );
            }
        }

        let specials = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_1234),
        ];
        for &value in &specials {
            let v = V { x: value, y: 1.0 };
            assert_bytes_eq(
                "c2V special",
                &(c.c2V)(value, -value),
                &(rust.c2V)(value, -value),
            );
            assert_bytes_eq(
                "c2Maxv special",
                &(c.c2Maxv)(v, ZERO_V),
                &(rust.c2Maxv)(v, ZERO_V),
            );
            assert_bytes_eq(
                "c2Minv special",
                &(c.c2Minv)(v, ZERO_V),
                &(rust.c2Minv)(v, ZERO_V),
            );
            assert_bits_eq("c2Len special", (c.c2Len)(v), (rust.c2Len)(v));
        }
        for (a, b) in [
            (V { x: 2.0, y: 2.0 }, V { x: 1.0, y: 1.0 }),
            (V { x: 2.0, y: 1.0 }, V { x: 1.0, y: 2.0 }),
            (V { x: 1.0, y: 2.0 }, V { x: 2.0, y: 1.0 }),
            (V { x: 1.0, y: 1.0 }, V { x: 2.0, y: 2.0 }),
            (V { x: 0.0, y: -0.0 }, V { x: -0.0, y: 0.0 }),
        ] {
            assert_bytes_eq(
                "c2Maxv selection matrix",
                &(c.c2Maxv)(a, b),
                &(rust.c2Maxv)(a, b),
            );
            assert_bytes_eq(
                "c2Minv selection matrix",
                &(c.c2Minv)(a, b),
                &(rust.c2Minv)(a, b),
            );
        }
        for value in [
            ZERO_V,
            V {
                x: f32::MAX,
                y: 0.0,
            },
        ] {
            assert_bits_eq("c2Len boundary", (c.c2Len)(value), (rust.c2Len)(value));
        }
        for (a, b) in [
            (V { x: 1.0, y: 0.0 }, V { x: 0.0, y: 1.0 }),
            (V { x: 0.0, y: 1.0 }, V { x: 1.0, y: 0.0 }),
            (V { x: 1.0, y: 1.0 }, V { x: 2.0, y: 2.0 }),
        ] {
            assert_bits_eq("c2Det2 sign matrix", (c.c2Det2)(a, b), (rust.c2Det2)(a, b));
        }
        for case in 0..128 {
            let lo = rng.moderate();
            let hi = lo + rng.positive();
            let values = [
                lo - rng.positive(),
                lo + (hi - lo) * 0.5,
                hi + rng.positive(),
            ];
            let value = V {
                x: values[case % values.len()],
                y: values[(case / values.len()) % values.len()],
            };
            let lo_v = V { x: lo, y: lo };
            let hi_v = V { x: hi, y: hi };
            assert_bytes_eq(
                "c2Clampv position matrix",
                &(c.c2Clampv)(value, lo_v, hi_v),
                &(rust.c2Clampv)(value, lo_v, hi_v),
            );
        }
        for &zero in &[0.0, -0.0] {
            assert_bytes_eq(
                "c2Div zero",
                &(c.c2Div)(V { x: 1.0, y: -1.0 }, zero),
                &(rust.c2Div)(V { x: 1.0, y: -1.0 }, zero),
            );
        }
        assert_bytes_eq("c2Norm zero", &(c.c2Norm)(ZERO_V), &(rust.c2Norm)(ZERO_V));

        let canary = Proxy {
            radius: f32::from_bits(0x7fc0_1234),
            count: -991,
            verts: [V { x: 17.0, y: -23.0 }; 8],
        };
        for invalid_type in [-1, 3, c_int::MAX] {
            let mut c_proxy = canary;
            let mut rust_proxy = canary;
            (c.c2MakeProxy)(std::ptr::null(), invalid_type, &mut c_proxy);
            (rust.c2MakeProxy)(std::ptr::null(), invalid_type, &mut rust_proxy);
            assert_bytes_eq("invalid c2MakeProxy enum", &c_proxy, &rust_proxy);
            assert_bytes_eq("C invalid enum preserves proxy", &c_proxy, &canary);
        }
        for case in 0..128 {
            let invalid_type = if case & 1 == 0 {
                3 + (rng.next_u32() % 10_000) as c_int
            } else {
                -1 - (rng.next_u32() % 10_000) as c_int
            };
            let randomized_canary = Proxy {
                radius: rng.finite(),
                count: rng.next_u32() as c_int,
                verts: std::array::from_fn(|_| rng.vector()),
            };
            let mut c_proxy = randomized_canary;
            let mut rust_proxy = randomized_canary;
            (c.c2MakeProxy)(std::ptr::null(), invalid_type, &mut c_proxy);
            (rust.c2MakeProxy)(std::ptr::null(), invalid_type, &mut rust_proxy);
            assert_bytes_eq("random invalid c2MakeProxy enum", &c_proxy, &rust_proxy);
        }

        let one = [V { x: 3.0, y: -7.0 }];
        for count in [1, 0, -1, c_int::MIN] {
            assert_eq!(
                (c.c2Support)(one.as_ptr(), count, V { x: 2.0, y: 5.0 }),
                (rust.c2Support)(one.as_ptr(), count, V { x: 2.0, y: 5.0 })
            );
        }
        for case in 0..128 {
            let one = [rng.vector()];
            let direction = rng.vector();
            for count in [1, 0, -1 - case] {
                assert_eq!(
                    (c.c2Support)(one.as_ptr(), count, direction),
                    (rust.c2Support)(one.as_ptr(), count, direction),
                    "c2Support boundary count {count} case {case}"
                );
            }
        }
        for case in 0..128 {
            let count = if case == 0 { 17 } else { 2 + (case % 31) };
            let verts: Vec<_> = (0..count).map(|_| rng.vector()).collect();
            let d = rng.vector();
            assert_eq!(
                (c.c2Support)(verts.as_ptr(), count, d),
                (rust.c2Support)(verts.as_ptr(), count, d),
                "c2Support case {case}"
            );
        }
        let tied = [
            V { x: 1.0, y: 0.0 },
            V { x: 1.0, y: 9.0 },
            V { x: 0.0, y: 100.0 },
        ];
        assert_eq!((c.c2Support)(tied.as_ptr(), 3, V { x: 1.0, y: 0.0 }), 0);
        assert_eq!((rust.c2Support)(tied.as_ptr(), 3, V { x: 1.0, y: 0.0 }), 0);
    }
}

fn scaled(v: V, scale: f32) -> V {
    V {
        x: v.x * scale,
        y: v.y * scale,
    }
}

#[test]
fn randomized_simplex_branch_functions_match() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x9876_5432_dead_beef);

    unsafe {
        for count in [-7, 0, 1, 2, 3, 4, c_int::MAX] {
            for case in 0..96 {
                let mut c_simplex = simplex(count, rng.vector(), rng.vector(), rng.vector());
                let mut rust_simplex = c_simplex;
                assert_bits_eq(
                    &format!("c2GJKSimplexMetric count {count} case {case}"),
                    (c.c2GJKSimplexMetric)(&mut c_simplex),
                    (rust.c2GJKSimplexMetric)(&mut rust_simplex),
                );
            }
        }

        let c22_regions = [
            (
                "vertex A",
                V { x: 1.0, y: 0.0 },
                V { x: 2.0, y: 0.0 },
                1,
                V { x: 1.0, y: 0.0 },
            ),
            (
                "vertex B",
                V { x: -2.0, y: 0.0 },
                V { x: -1.0, y: 0.0 },
                1,
                V { x: -1.0, y: 0.0 },
            ),
            (
                "edge AB",
                V { x: -1.0, y: 1.0 },
                V { x: 1.0, y: 1.0 },
                2,
                V { x: -1.0, y: 1.0 },
            ),
        ];
        for (label, a, b, expected_count, expected_a) in c22_regions {
            for case in 0..96 {
                let scale = rng.positive();
                let mut c_simplex = simplex(2, scaled(a, scale), scaled(b, scale), ZERO_V);
                c_simplex.a.sA = rng.vector();
                c_simplex.a.sB = rng.vector();
                c_simplex.b.sA = rng.vector();
                c_simplex.b.sB = rng.vector();
                c_simplex.a.iA = 11;
                c_simplex.b.iA = 22;
                let mut rust_simplex = c_simplex;
                (c.c22)(&mut c_simplex);
                (rust.c22)(&mut rust_simplex);
                assert_bytes_eq(
                    &format!("c22 {label} case {case}"),
                    &c_simplex,
                    &rust_simplex,
                );
                assert_eq!(c_simplex.count, expected_count, "wrong c22 region: {label}");
                assert_eq!(
                    c_simplex.a.p.x.to_bits(),
                    scaled(expected_a, scale).x.to_bits()
                );
                assert_eq!(
                    c_simplex.a.p.y.to_bits(),
                    scaled(expected_a, scale).y.to_bits()
                );
            }
        }

        let c23_regions = [
            (
                "vertex A",
                [
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: 1.0 },
                    V { x: 2.0, y: -1.0 },
                ],
                1,
                11,
                0,
            ),
            (
                "vertex B",
                [
                    V { x: 2.0, y: 1.0 },
                    V { x: 1.0, y: 0.0 },
                    V { x: 2.0, y: -1.0 },
                ],
                1,
                22,
                0,
            ),
            (
                "vertex C",
                [
                    V { x: 2.0, y: -1.0 },
                    V { x: 2.0, y: 1.0 },
                    V { x: 1.0, y: 0.0 },
                ],
                1,
                33,
                0,
            ),
            (
                "edge AB",
                [
                    V { x: -1.0, y: 1.0 },
                    V { x: 1.0, y: 1.0 },
                    V { x: 0.0, y: 2.0 },
                ],
                2,
                11,
                22,
            ),
            (
                "edge BC",
                [
                    V { x: 0.0, y: 2.0 },
                    V { x: -1.0, y: 1.0 },
                    V { x: 1.0, y: 1.0 },
                ],
                2,
                22,
                33,
            ),
            (
                "edge CA",
                [
                    V { x: 1.0, y: 1.0 },
                    V { x: 0.0, y: 2.0 },
                    V { x: -1.0, y: 1.0 },
                ],
                2,
                33,
                11,
            ),
            (
                "triangle ABC",
                [
                    V { x: -1.0, y: -1.0 },
                    V { x: 1.0, y: -1.0 },
                    V { x: 0.0, y: 1.0 },
                ],
                3,
                11,
                22,
            ),
        ];
        for (label, points, expected_count, expected_a, expected_b) in c23_regions {
            for case in 0..96 {
                let scale = rng.positive();
                let mut c_simplex = simplex(
                    3,
                    scaled(points[0], scale),
                    scaled(points[1], scale),
                    scaled(points[2], scale),
                );
                c_simplex.a.iA = 11;
                c_simplex.b.iA = 22;
                c_simplex.c.iA = 33;
                c_simplex.a.sA = rng.vector();
                c_simplex.b.sA = rng.vector();
                c_simplex.c.sA = rng.vector();
                let mut rust_simplex = c_simplex;
                (c.c23)(&mut c_simplex);
                (rust.c23)(&mut rust_simplex);
                assert_bytes_eq(
                    &format!("c23 {label} case {case}"),
                    &c_simplex,
                    &rust_simplex,
                );
                assert_eq!(c_simplex.count, expected_count, "wrong c23 region: {label}");
                assert_eq!(c_simplex.a.iA, expected_a, "wrong c23 A: {label}");
                if expected_count >= 2 {
                    assert_eq!(c_simplex.b.iA, expected_b, "wrong c23 B: {label}");
                }
            }
        }

        for case in 0..256 {
            let a = rng.vector();
            let b = rng.vector();
            let mut one = simplex(1, a, b, ZERO_V);
            let mut one_rust = one;
            assert_bytes_eq(
                "c2D count 1",
                &(c.c2D)(&mut one),
                &(rust.c2D)(&mut one_rust),
            );

            let mut two = simplex(2, a, b, ZERO_V);
            let mut two_rust = two;
            assert_bytes_eq(
                "c2D count 2",
                &(c.c2D)(&mut two),
                &(rust.c2D)(&mut two_rust),
            );
            let mut other = simplex(if case & 1 == 0 { 3 } else { -1 }, a, b, ZERO_V);
            let mut other_rust = other;
            assert_bytes_eq(
                "c2D default",
                &(c.c2D)(&mut other),
                &(rust.c2D)(&mut other_rust),
            );

            let mut weighted = simplex((case % 4) as c_int, a, b, rng.vector());
            weighted.a.sA = rng.vector();
            weighted.a.sB = rng.vector();
            weighted.b.sA = rng.vector();
            weighted.b.sB = rng.vector();
            weighted.c.sA = rng.vector();
            weighted.c.sB = rng.vector();
            weighted.a.u = rng.positive();
            weighted.b.u = rng.positive();
            weighted.c.u = rng.positive();
            weighted.div = weighted.a.u + weighted.b.u + weighted.c.u;
            let mut weighted_rust = weighted;
            let mut c_a = V {
                x: 777.0,
                y: -888.0,
            };
            let mut c_b = V {
                x: 999.0,
                y: -111.0,
            };
            let mut rust_a = c_a;
            let mut rust_b = c_b;
            (c.c2Witness)(&mut weighted, &mut c_a, &mut c_b);
            (rust.c2Witness)(&mut weighted_rust, &mut rust_a, &mut rust_b);
            assert_bytes_eq(&format!("c2Witness A case {case}"), &c_a, &rust_a);
            assert_bytes_eq(&format!("c2Witness B case {case}"), &c_b, &rust_b);

            for count in [1, 2, 0, 3, -9] {
                let mut c_l = weighted;
                c_l.count = count;
                let mut rust_l = c_l;
                assert_bytes_eq(
                    &format!("c2L count {count} case {case}"),
                    &(c.c2L)(&mut c_l),
                    &(rust.c2L)(&mut rust_l),
                );
            }
        }

        // Force both determinant directions in c2D's count-2 branch.
        for b in [V { x: 1.0, y: 1.0 }, V { x: 1.0, y: -1.0 }] {
            let mut c_simplex = simplex(2, V { x: 1.0, y: 0.0 }, b, ZERO_V);
            let mut rust_simplex = c_simplex;
            assert_bytes_eq(
                "c2D determinant side",
                &(c.c2D)(&mut c_simplex),
                &(rust.c2D)(&mut rust_simplex),
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn shape_type(&self) -> c_int {
        match self {
            Self::Circle(_) => 0,
            Self::Aabb(_) => 1,
            Self::Capsule(_) => 2,
        }
    }

    fn as_ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const Circle).cast(),
            Self::Aabb(value) => (value as *const Aabb).cast(),
            Self::Capsule(value) => (value as *const Capsule).cast(),
        }
    }

    fn vertices(&self) -> Vec<V> {
        match self {
            Self::Circle(value) => vec![value.p],
            Self::Aabb(value) => vec![
                value.min,
                V {
                    x: value.max.x,
                    y: value.min.y,
                },
                value.max,
                V {
                    x: value.min.x,
                    y: value.max.y,
                },
            ],
            Self::Capsule(value) => vec![value.a, value.b],
        }
    }
}

fn random_shape(shape_type: c_int, rng: &mut Rng) -> Shape {
    match shape_type {
        0 => Shape::Circle(Circle {
            p: rng.vector(),
            r: rng.positive(),
        }),
        1 => {
            let center = rng.vector();
            let half = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Aabb(Aabb {
                min: V {
                    x: center.x - half.x,
                    y: center.y - half.y,
                },
                max: V {
                    x: center.x + half.x,
                    y: center.y + half.y,
                },
            })
        }
        2 => Shape::Capsule(Capsule {
            a: rng.vector(),
            b: rng.vector(),
            r: rng.positive(),
        }),
        _ => unreachable!(),
    }
}

fn transform_ptr(value: Option<&X>) -> *const X {
    value.map_or(std::ptr::null(), |x| x)
}

fn output_ptr<T>(enabled: bool, value: &mut T) -> *mut T {
    if enabled { value } else { std::ptr::null_mut() }
}

unsafe fn compare_gjk_call(
    label: &str,
    c: &Api,
    rust: &Api,
    shape_a: &Shape,
    ax: Option<&X>,
    shape_b: &Shape,
    bx: Option<&X>,
    use_radius: c_int,
    cache_seed: Option<Cache>,
    output_mask: u8,
) -> Option<Cache> {
    let mut c_a = V {
        x: f32::from_bits(0x7fc0_1234),
        y: 12345.0,
    };
    let mut rust_a = c_a;
    let mut c_b = V {
        x: -54321.0,
        y: f32::from_bits(0x7fc0_5678),
    };
    let mut rust_b = c_b;
    let mut c_iterations = -771;
    let mut rust_iterations = c_iterations;
    let mut c_cache = cache_seed.unwrap_or(Cache {
        metric: f32::from_bits(0x7fc0_abcd),
        count: -99,
        iA: [101, 102, 103],
        iB: [201, 202, 203],
        div: -707.0,
    });
    let mut rust_cache = c_cache;
    let cache_enabled = cache_seed.is_some();

    let c_distance = unsafe {
        (c.c2GJK)(
            shape_a.as_ptr(),
            shape_a.shape_type(),
            transform_ptr(ax),
            shape_b.as_ptr(),
            shape_b.shape_type(),
            transform_ptr(bx),
            output_ptr(output_mask & 1 != 0, &mut c_a),
            output_ptr(output_mask & 2 != 0, &mut c_b),
            use_radius,
            output_ptr(output_mask & 4 != 0, &mut c_iterations),
            output_ptr(cache_enabled, &mut c_cache),
        )
    };
    let rust_distance = unsafe {
        (rust.c2GJK)(
            shape_a.as_ptr(),
            shape_a.shape_type(),
            transform_ptr(ax),
            shape_b.as_ptr(),
            shape_b.shape_type(),
            transform_ptr(bx),
            output_ptr(output_mask & 1 != 0, &mut rust_a),
            output_ptr(output_mask & 2 != 0, &mut rust_b),
            use_radius,
            output_ptr(output_mask & 4 != 0, &mut rust_iterations),
            output_ptr(cache_enabled, &mut rust_cache),
        )
    };
    assert_bits_eq(&format!("{label} distance"), c_distance, rust_distance);
    assert_bytes_eq(&format!("{label} outA"), &c_a, &rust_a);
    assert_bytes_eq(&format!("{label} outB"), &c_b, &rust_b);
    assert_eq!(
        c_iterations, rust_iterations,
        "{label} iteration count differs"
    );
    if cache_enabled {
        assert_bytes_eq(&format!("{label} cache"), &c_cache, &rust_cache);
        Some(c_cache)
    } else {
        None
    }
}

fn random_transform(rng: &mut Rng) -> X {
    let rotations = [
        R { c: 1.0, s: 0.0 },
        R { c: 0.0, s: 1.0 },
        R { c: 0.0, s: -1.0 },
        R { c: 0.6, s: 0.8 },
        R { c: -0.8, s: 0.6 },
    ];
    X {
        p: rng.vector(),
        r: rotations[rng.next_u32() as usize % rotations.len()],
    }
}

#[test]
fn randomized_gjk_shape_configuration_matrix_matches() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x0ddc_0ffe_e15e_5eed);
    let mut cache_counts = [false; 4];

    unsafe {
        for type_a in 0..=2 {
            for type_b in 0..=2 {
                for case in 0..128 {
                    let shape_a = random_shape(type_a, &mut rng);
                    let shape_b = random_shape(type_b, &mut rng);
                    let ax = random_transform(&mut rng);
                    let bx = random_transform(&mut rng);
                    let (ax, bx) = match case % 4 {
                        0 => (None, None),
                        1 => (Some(&ax), None),
                        2 => (None, Some(&bx)),
                        _ => (Some(&ax), Some(&bx)),
                    };
                    let fresh = Cache {
                        metric: 0.0,
                        count: 0,
                        iA: [0; 3],
                        iB: [0; 3],
                        div: 0.0,
                    };
                    let label = format!("types {type_a}/{type_b} case {case} fresh");
                    let warm = compare_gjk_call(
                        &label,
                        &c,
                        &rust,
                        &shape_a,
                        ax,
                        &shape_b,
                        bx,
                        if case & 1 == 0 { 0 } else { 7 },
                        Some(fresh),
                        7,
                    )
                    .unwrap();
                    if (0..=3).contains(&warm.count) {
                        cache_counts[warm.count as usize] = true;
                    }
                    compare_gjk_call(
                        &format!("types {type_a}/{type_b} case {case} warm"),
                        &c,
                        &rust,
                        &shape_a,
                        ax,
                        &shape_b,
                        bx,
                        if case & 1 == 0 { 0 } else { -1 },
                        Some(warm),
                        7,
                    );
                }
            }
        }
    }

    assert!(
        cache_counts[1],
        "random matrix never produced a count-1 cache"
    );
    assert!(
        cache_counts[2],
        "random matrix never produced a count-2 cache"
    );
    assert!(
        cache_counts[3],
        "random matrix never produced a count-3 cache"
    );
}

#[test]
fn gjk_boundaries_cache_modes_and_wrapper_match() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xa11c_e55e_1234_5678);
    let identity = X {
        p: ZERO_V,
        r: R { c: 1.0, s: 0.0 },
    };

    unsafe {
        let separated_a = Shape::Circle(Circle {
            p: V { x: -100.0, y: 0.0 },
            r: 3.0,
        });
        let separated_b = Shape::Capsule(Capsule {
            a: V { x: 50.0, y: -20.0 },
            b: V { x: 50.0, y: 20.0 },
            r: 7.0,
        });
        let overlapping_a = Shape::Aabb(Aabb {
            min: V { x: -10.0, y: -10.0 },
            max: V { x: 10.0, y: 10.0 },
        });
        let overlapping_b = Shape::Circle(Circle { p: ZERO_V, r: 5.0 });

        for use_radius in [0, 1, -1, c_int::MAX] {
            compare_gjk_call(
                "forced separated radius mode",
                &c,
                &rust,
                &separated_a,
                None,
                &separated_b,
                None,
                use_radius,
                None,
                7,
            );
            compare_gjk_call(
                "forced overlap radius mode",
                &c,
                &rust,
                &overlapping_a,
                None,
                &overlapping_b,
                None,
                use_radius,
                None,
                7,
            );
        }

        for transform_mode in 0..4 {
            let shifted = X {
                p: V { x: 13.0, y: -29.0 },
                r: R { c: 0.0, s: 1.0 },
            };
            let (ax, bx) = match transform_mode {
                0 => (None, None),
                1 => (Some(&shifted), None),
                2 => (None, Some(&shifted)),
                _ => (Some(&identity), Some(&shifted)),
            };
            compare_gjk_call(
                "transform null matrix",
                &c,
                &rust,
                &separated_a,
                ax,
                &separated_b,
                bx,
                1,
                None,
                7,
            );
        }

        for case in 0..128 {
            let shape_a = random_shape((case % 3) as c_int, &mut rng);
            let shape_b = random_shape(((case / 3) % 3) as c_int, &mut rng);
            let output_mask = (case % 8) as u8;
            compare_gjk_call(
                &format!("optional outputs mask {output_mask} case {case}"),
                &c,
                &rust,
                &shape_a,
                None,
                &shape_b,
                None,
                if case & 1 == 0 { 0 } else { 1 },
                None,
                output_mask,
            );
        }

        let geometry_cases = [
            (
                "touching",
                Shape::Circle(Circle { p: ZERO_V, r: 1.0 }),
                Shape::Circle(Circle {
                    p: V { x: 2.0, y: 0.0 },
                    r: 1.0,
                }),
                1,
            ),
            (
                "coincident",
                Shape::Circle(Circle { p: ZERO_V, r: 0.0 }),
                Shape::Circle(Circle { p: ZERO_V, r: 0.0 }),
                0,
            ),
            (
                "tiny direction",
                Shape::Circle(Circle { p: ZERO_V, r: 0.0 }),
                Shape::Circle(Circle {
                    p: V {
                        x: f32::EPSILON * 0.25,
                        y: 0.0,
                    },
                    r: 0.0,
                }),
                0,
            ),
            (
                "overlapping boxes",
                Shape::Aabb(Aabb {
                    min: V { x: -2.0, y: -2.0 },
                    max: V { x: 2.0, y: 2.0 },
                }),
                Shape::Aabb(Aabb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 3.0, y: 3.0 },
                }),
                1,
            ),
        ];
        for (label, shape_a, shape_b, use_radius) in geometry_cases {
            compare_gjk_call(
                label,
                &c,
                &rust,
                &shape_a,
                None,
                &shape_b,
                None,
                use_radius,
                Some(Cache {
                    metric: 0.0,
                    count: 0,
                    iA: [0; 3],
                    iB: [0; 3],
                    div: 0.0,
                }),
                7,
            );
        }

        let large_a = Shape::Aabb(Aabb {
            min: V {
                x: -20_000.0,
                y: -20_000.0,
            },
            max: V {
                x: 20_000.0,
                y: 20_000.0,
            },
        });
        let large_b = large_a;
        let verts_a = large_a.vertices();
        let verts_b = large_b.vertices();
        let mut stale = None;
        'search: for i_a0 in 0..4 {
            for i_b0 in 0..4 {
                for i_a1 in 0..4 {
                    for i_b1 in 0..4 {
                        for i_a2 in 0..4 {
                            for i_b2 in 0..4 {
                                let p0 = V {
                                    x: verts_b[i_b0].x - verts_a[i_a0].x,
                                    y: verts_b[i_b0].y - verts_a[i_a0].y,
                                };
                                let p1 = V {
                                    x: verts_b[i_b1].x - verts_a[i_a1].x,
                                    y: verts_b[i_b1].y - verts_a[i_a1].y,
                                };
                                let p2 = V {
                                    x: verts_b[i_b2].x - verts_a[i_a2].x,
                                    y: verts_b[i_b2].y - verts_a[i_a2].y,
                                };
                                let metric =
                                    (p1.x - p0.x) * (p2.y - p0.y) - (p1.y - p0.y) * (p2.x - p0.x);
                                if metric < -1.0e8 {
                                    stale = Some(Cache {
                                        metric: 1.0,
                                        count: 3,
                                        iA: [i_a0 as c_int, i_a1 as c_int, i_a2 as c_int],
                                        iB: [i_b0 as c_int, i_b1 as c_int, i_b2 as c_int],
                                        div: 1.0,
                                    });
                                    break 'search;
                                }
                            }
                        }
                    }
                }
            }
        }
        let stale = stale.expect("failed to construct a negative stale triangle metric");
        for case in 0..64 {
            let extent = 20_000.0 + rng.positive() * 100.0;
            let scaled_a = Shape::Aabb(Aabb {
                min: V {
                    x: -extent,
                    y: -extent,
                },
                max: V {
                    x: extent,
                    y: extent,
                },
            });
            let scaled_b = scaled_a;
            compare_gjk_call(
                &format!("stale negative cache rejected case {case}"),
                &c,
                &rust,
                &scaled_a,
                None,
                &scaled_b,
                None,
                0,
                Some(stale),
                7,
            );
        }

        for case in 0..256 {
            let values = [
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.finite(),
                rng.positive(),
            ];
            for reverse in [0_i8, 1, -1, i8::MAX] {
                let mut c_a = V {
                    x: 123.0,
                    y: -456.0,
                };
                let mut c_b = V {
                    x: 789.0,
                    y: -1011.0,
                };
                let mut rust_a = c_a;
                let mut rust_b = c_b;
                let a_enabled = case & 1 != 0;
                let b_enabled = case & 2 != 0;
                (c.gjk_cache)(
                    reverse,
                    output_ptr(a_enabled, &mut c_a),
                    output_ptr(b_enabled, &mut c_b),
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                    values[4],
                    values[5],
                    values[6],
                    values[7],
                    values[8],
                );
                (rust.gjk_cache)(
                    reverse,
                    output_ptr(a_enabled, &mut rust_a),
                    output_ptr(b_enabled, &mut rust_b),
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                    values[4],
                    values[5],
                    values[6],
                    values[7],
                    values[8],
                );
                assert_bytes_eq("gjk_cache ignored a9", &c_a, &rust_a);
                assert_bytes_eq("gjk_cache ignored b9", &c_b, &rust_b);
                assert_bytes_eq(
                    "C gjk_cache preserves a9",
                    &c_a,
                    &V {
                        x: 123.0,
                        y: -456.0,
                    },
                );
                assert_bytes_eq(
                    "C gjk_cache preserves b9",
                    &c_b,
                    &V {
                        x: 789.0,
                        y: -1011.0,
                    },
                );
            }
        }
    }
}

unsafe fn run_null_probe(api: &Api, case: &str) {
    let mut out = [ZERO_V; 4];
    let mut bb = Aabb {
        min: V { x: -1.0, y: -1.0 },
        max: V { x: 1.0, y: 1.0 },
    };
    let circle = Circle { p: ZERO_V, r: 1.0 };
    let mut proxy = Proxy {
        radius: 0.0,
        count: 0,
        verts: [ZERO_V; 8],
    };
    let mut value = simplex(1, V { x: 1.0, y: 0.0 }, ZERO_V, ZERO_V);
    value.a.sA = V { x: 1.0, y: 2.0 };
    value.a.sB = V { x: 3.0, y: 4.0 };
    let mut witness_a = ZERO_V;
    let mut witness_b = ZERO_V;

    match case {
        "bb_out" => unsafe { (api.c2BBVerts)(std::ptr::null_mut(), &mut bb) },
        "bb_input" => unsafe { (api.c2BBVerts)(out.as_mut_ptr(), std::ptr::null_mut()) },
        "proxy_shape" => unsafe { (api.c2MakeProxy)(std::ptr::null(), 0, &mut proxy) },
        "proxy_output" => unsafe {
            (api.c2MakeProxy)((&circle as *const Circle).cast(), 0, std::ptr::null_mut())
        },
        "metric" => unsafe {
            (api.c2GJKSimplexMetric)(std::ptr::null_mut());
        },
        "c22" => unsafe { (api.c22)(std::ptr::null_mut()) },
        "c23" => unsafe { (api.c23)(std::ptr::null_mut()) },
        "direction" => unsafe {
            (api.c2D)(std::ptr::null_mut());
        },
        "support" => unsafe {
            (api.c2Support)(std::ptr::null(), 1, V { x: 1.0, y: 1.0 });
        },
        "witness_simplex" => unsafe {
            (api.c2Witness)(std::ptr::null_mut(), &mut witness_a, &mut witness_b)
        },
        "witness_a" => unsafe { (api.c2Witness)(&mut value, std::ptr::null_mut(), &mut witness_b) },
        "witness_b" => unsafe { (api.c2Witness)(&mut value, &mut witness_a, std::ptr::null_mut()) },
        "interpolation" => unsafe {
            (api.c2L)(std::ptr::null_mut());
        },
        "gjk_a" => unsafe {
            (api.c2GJK)(
                std::ptr::null(),
                0,
                std::ptr::null(),
                (&circle as *const Circle).cast(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        },
        "gjk_b" => unsafe {
            (api.c2GJK)(
                (&circle as *const Circle).cast(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        },
        _ => panic!("unknown null probe: {case}"),
    }
}

#[test]
fn null_pointer_termination_matches() {
    if let (Ok(library), Ok(case)) = (
        std::env::var("DIFF_NULL_LIBRARY"),
        std::env::var("DIFF_NULL_CASE"),
    ) {
        let root = manifest_dir();
        let path = match library.as_str() {
            "c" => root.join("../c_src/build/libharvest-work-GriyDD.so"),
            "rust" => root.join("target/release/libgjk_cache_lib.so"),
            _ => panic!("unknown library selector: {library}"),
        };
        let api = unsafe { Api::load(&path) };
        unsafe { run_null_probe(&api, &case) };
        panic!("null probe unexpectedly returned: {library}/{case}");
    }

    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    let cases = [
        "bb_out",
        "bb_input",
        "proxy_shape",
        "proxy_output",
        "metric",
        "c22",
        "c23",
        "direction",
        "support",
        "witness_simplex",
        "witness_a",
        "witness_b",
        "interpolation",
        "gjk_a",
        "gjk_b",
    ];
    for case in cases {
        let run = |library: &str| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_termination_matches", "--nocapture"])
                .env("DIFF_NULL_LIBRARY", library)
                .env("DIFF_NULL_CASE", case)
                .env("RUST_TEST_THREADS", "1")
                .output()
                .unwrap()
                .status
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(
            !c_status.success() && !rust_status.success(),
            "{case}: a null-pointer probe unexpectedly succeeded"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{case}: C status {c_status:?}, Rust status {rust_status:?}"
        );
    }
}
