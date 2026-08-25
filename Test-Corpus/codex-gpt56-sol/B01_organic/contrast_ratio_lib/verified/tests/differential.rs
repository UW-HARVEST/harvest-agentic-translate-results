use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type ContrastRatio = unsafe extern "C" fn(CbRgb255, CbRgb255) -> f32;

struct Implementations {
    c: Library,
    rust: Library,
}

impl Implementations {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build c_src first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        // Each handle resolves its own symbol, exactly as an external caller does.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    fn compare(&self, a: CbRgb255, b: CbRgb255) {
        unsafe {
            let c_fn: Symbol<'_, ContrastRatio> =
                self.c.get(b"contrast_ratio\0").expect("load C symbol");
            let rust_fn: Symbol<'_, ContrastRatio> = self
                .rust
                .get(b"contrast_ratio\0")
                .expect("load Rust symbol");
            let expected = c_fn(a, b);
            let actual = rust_fn(a, b);

            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "mismatch for A={a:?}, B={b:?}: C={expected:?} ({:#010x}), Rust={actual:?} ({:#010x})",
                expected.to_bits(),
                actual.to_bits()
            );
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DIFFERENTIAL_LIB") {
        return PathBuf::from(path);
    }

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    root.join("target")
        .join(profile)
        .join("libcontrast_ratio_lib.so")
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u8(&mut self) -> u8 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u8
    }

    fn channel_for_branch(&mut self, power_branch: bool) -> u8 {
        if power_branch {
            11 + self.next_u8() % 245
        } else {
            self.next_u8() % 11
        }
    }
}

fn pair_for_mask(rng: &mut FixedRng, mask: u8) -> (CbRgb255, CbRgb255) {
    let mut channels = [0_u8; 6];
    for (index, channel) in channels.iter_mut().enumerate() {
        let power_branch = mask & (1 << (5 - index)) != 0;
        *channel = rng.channel_for_branch(power_branch);
    }
    (
        CbRgb255 {
            r: channels[0],
            g: channels[1],
            b: channels[2],
        },
        CbRgb255 {
            r: channels[3],
            g: channels[4],
            b: channels[5],
        },
    )
}

#[test]
fn all_channel_threshold_branch_combinations_match() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x8d26_4f7b_c1a9_350e);

    for mask in 0_u8..64 {
        for _ in 0..256 {
            let (a, b) = pair_for_mask(&mut rng, mask);
            implementations.compare(a, b);
            implementations.compare(b, a);
        }
    }
}

#[test]
fn zero_luminance_and_equal_color_cases_match() {
    let implementations = Implementations::load();
    let black = CbRgb255 { r: 0, g: 0, b: 0 };
    implementations.compare(black, black);

    let mut rng = FixedRng::new(0x2b19_a430_ee76_15cd);
    for _ in 0..512 {
        let mut color = CbRgb255 {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
        };
        if color.r == 0 && color.g == 0 && color.b == 0 {
            color.r = 1;
        }
        implementations.compare(black, color);
        implementations.compare(color, black);
        implementations.compare(color, color);
    }
}

#[test]
fn threshold_adjacent_and_domain_endpoint_cases_match() {
    let implementations = Implementations::load();

    for position in 0..6 {
        for bits in 0_u8..64 {
            let mut channels = [10_u8; 6];
            for (index, channel) in channels.iter_mut().enumerate() {
                *channel = if bits & (1 << index) == 0 { 10 } else { 11 };
            }
            channels[position] = 11;
            implementations.compare(
                CbRgb255 {
                    r: channels[0],
                    g: channels[1],
                    b: channels[2],
                },
                CbRgb255 {
                    r: channels[3],
                    g: channels[4],
                    b: channels[5],
                },
            );
        }
    }

    for bits in 0_u8..64 {
        let mut channels = [0_u8; 6];
        for (index, channel) in channels.iter_mut().enumerate() {
            *channel = if bits & (1 << index) == 0 { 0 } else { 255 };
        }
        implementations.compare(
            CbRgb255 {
                r: channels[0],
                g: channels[1],
                b: channels[2],
            },
            CbRgb255 {
                r: channels[3],
                g: channels[4],
                b: channels[5],
            },
        );
    }
}

#[test]
fn unrestricted_random_inputs_match() {
    let implementations = Implementations::load();
    let mut rng = FixedRng::new(0x72c4_19d8_a605_3efb);

    for _ in 0..100_000 {
        let a = CbRgb255 {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
        };
        let b = CbRgb255 {
            r: rng.next_u8(),
            g: rng.next_u8(),
            b: rng.next_u8(),
        };
        implementations.compare(a, b);
    }
}
