use libloading::Library;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type ToBarycentric = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn index(&mut self, len: usize) -> usize {
        (self.next_u64() as usize) % len
    }

    fn normal_f32(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let exponent = 120 + (self.next_u32() % 15);
        let fraction = self.next_u32() & 0x007f_ffff;
        f32::from_bits(sign | (exponent << 23) | fraction)
    }

    fn unit_f32(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-xlkWFI.so"),
        crate_dir.join("target/release/libto_barycentric_lib.so"),
    )
}

fn with_implementations(test: impl FnOnce(ToBarycentric, ToBarycentric)) {
    let (c_path, rust_path) = library_paths();
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

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_function: ToBarycentric = *c_library
            .get::<ToBarycentric>(b"to_barycentric\0")
            .expect("load C to_barycentric");
        let rust_function: ToBarycentric = *rust_library
            .get::<ToBarycentric>(b"to_barycentric\0")
            .expect("load Rust to_barycentric");

        test(c_function, rust_function);
    }
}

fn assert_same(
    c_function: ToBarycentric,
    rust_function: ToBarycentric,
    case: usize,
    inputs: [LmVec2; 4],
) {
    let c_result = unsafe { c_function(inputs[0], inputs[1], inputs[2], inputs[3]) };
    let rust_result = unsafe { rust_function(inputs[0], inputs[1], inputs[2], inputs[3]) };
    let c_bits = [c_result.x.to_bits(), c_result.y.to_bits()];
    let rust_bits = [rust_result.x.to_bits(), rust_result.y.to_bits()];
    let input_bits = inputs.map(|value| [value.x.to_bits(), value.y.to_bits()]);

    assert_eq!(
        c_bits, rust_bits,
        "case {case}: inputs={inputs:?}, input_bits={input_bits:08x?}, \
         C={c_result:?}, Rust={rust_result:?}"
    );
}

fn point(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

#[test]
fn finite_nondegenerate_inputs_match() {
    with_implementations(|c_function, rust_function| {
        let mut rng = Rng::new(0x14e8_50b1_a4d3_3f72);

        for case in 0..10_000 {
            let p1 = point(rng.normal_f32(), rng.normal_f32());
            let dx = rng.normal_f32();
            let dy = rng.normal_f32();
            let p2 = point(p1.x + dx, p1.y + dy);
            let p3 = point(p1.x - dy, p1.y + dx);
            let (u, v) = match case % 4 {
                0 => {
                    let u = rng.unit_f32() * 0.5;
                    let v = rng.unit_f32() * (1.0 - u);
                    (u, v)
                }
                1 => (rng.unit_f32(), 0.0),
                2 => match (case / 4) % 3 {
                    0 => (0.0, 0.0),
                    1 => (1.0, 0.0),
                    _ => (0.0, 1.0),
                },
                _ => (rng.unit_f32() * 4.0 - 1.0, rng.unit_f32() * 4.0 - 1.0),
            };
            let p = point(
                p1.x + (p3.x - p1.x) * u + (p2.x - p1.x) * v,
                p1.y + (p3.y - p1.y) * u + (p2.y - p1.y) * v,
            );

            assert_same(c_function, rust_function, case, [p1, p2, p3, p]);
        }
    });
}

#[test]
fn degenerate_and_near_degenerate_inputs_match() {
    with_implementations(|c_function, rust_function| {
        let mut rng = Rng::new(0x2a3f_c19d_5e70_8b64);

        for case in 0..10_000 {
            let base = point(rng.normal_f32(), rng.normal_f32());
            let p = point(rng.normal_f32(), rng.normal_f32());
            let inputs = match case % 5 {
                0 => [base, base, base, p],
                1 => {
                    let p3 = point(rng.normal_f32(), rng.normal_f32());
                    [base, base, p3, p]
                }
                2 => {
                    let direction = point(
                        (rng.next_u32() as i16) as f32,
                        (rng.next_u32() as i16) as f32,
                    );
                    let p2 = point(base.x + direction.x, base.y + direction.y);
                    let p3 = point(base.x + direction.x * 2.0, base.y + direction.y * 2.0);
                    [base, p2, p3, p]
                }
                3 => [
                    point(0.0, 0.0),
                    point(1.0, 0.0),
                    point(1.0, f32::from_bits(1)),
                    p,
                ],
                _ => [
                    point(0.0, 0.0),
                    point(1.0, 1.0),
                    point(2.0, 2.0 + f32::EPSILON),
                    p,
                ],
            };

            assert_same(c_function, rust_function, case, inputs);
        }
    });
}

#[test]
fn finite_ieee_boundaries_match() {
    with_implementations(|c_function, rust_function| {
        const VALUES: [u32; 14] = [
            0x0000_0000,
            0x8000_0000,
            0x0000_0001,
            0x8000_0001,
            0x007f_ffff,
            0x807f_ffff,
            0x0080_0000,
            0x8080_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x7f00_0000,
            0xff00_0000,
        ];
        let mut rng = Rng::new(0xa19c_874d_e320_56bf);

        for case in 0..20_000 {
            let mut coordinates = [0.0_f32; 8];
            for coordinate in &mut coordinates {
                *coordinate = f32::from_bits(VALUES[rng.index(VALUES.len())]);
            }
            let inputs = [
                point(coordinates[0], coordinates[1]),
                point(coordinates[2], coordinates[3]),
                point(coordinates[4], coordinates[5]),
                point(coordinates[6], coordinates[7]),
            ];

            assert_same(c_function, rust_function, case, inputs);
        }
    });
}

#[test]
fn nonfinite_and_nan_payload_inputs_match() {
    with_implementations(|c_function, rust_function| {
        const NONFINITE: [u32; 12] = [
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc0_0000,
            0x7fc0_0001,
            0xffc0_0001,
            0x7fff_ffff,
            0xffff_ffff,
            0x7f80_0001,
            0xff80_0001,
            0x7fa1_2345,
            0xffa1_2345,
        ];
        let mut rng = Rng::new(0x673b_0ea2_c958_f41d);

        for case in 0..20_000 {
            let mut coordinates = [0.0_f32; 8];
            for coordinate in &mut coordinates {
                *coordinate = if rng.next_u32() % 3 == 0 {
                    rng.normal_f32()
                } else {
                    f32::from_bits(NONFINITE[rng.index(NONFINITE.len())])
                };
            }
            coordinates[case % coordinates.len()] =
                f32::from_bits(NONFINITE[rng.index(NONFINITE.len())]);
            let inputs = [
                point(coordinates[0], coordinates[1]),
                point(coordinates[2], coordinates[3]),
                point(coordinates[4], coordinates[5]),
                point(coordinates[6], coordinates[7]),
            ];

            assert_same(c_function, rust_function, case, inputs);
        }
    });
}
