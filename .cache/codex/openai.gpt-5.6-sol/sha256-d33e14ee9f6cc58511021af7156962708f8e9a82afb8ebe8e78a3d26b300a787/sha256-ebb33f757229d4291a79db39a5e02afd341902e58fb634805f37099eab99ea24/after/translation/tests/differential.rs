use libloading::Library;
use std::path::{Path, PathBuf};

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug)]
struct Rgb {
    R: u8,
    G: u8,
    B: u8,
}

type ContrastRatio = unsafe extern "C" fn(Rgb, Rgb) -> f32;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_contrast_ratio: ContrastRatio,
    rust_contrast_ratio: ContrastRatio,
}

impl Libraries {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = find_c_library(&manifest_dir);
        let rust_path = manifest_dir.join("target/release/libcontrast_ratio_lib.so");

        assert!(
            rust_path.is_file(),
            "Rust cdylib is missing at {}; run cargo build --release first",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

            let c_contrast_ratio = *c
                .get::<ContrastRatio>(b"contrast_ratio\0")
                .expect("C shared object does not export contrast_ratio");
            let rust_contrast_ratio = *rust
                .get::<ContrastRatio>(b"contrast_ratio\0")
                .expect("Rust shared object does not export contrast_ratio");

            Self {
                _c: c,
                _rust: rust,
                c_contrast_ratio,
                rust_contrast_ratio,
            }
        }
    }

    fn assert_same(&self, a: Rgb, b: Rgb, context: &str) {
        let c_result = unsafe { (self.c_contrast_ratio)(a, b) };
        let rust_result = unsafe { (self.rust_contrast_ratio)(a, b) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "{context}: A={a:?}, B={b:?}, C={c_result:?} ({:#010x}), Rust={rust_result:?} ({:#010x})",
            c_result.to_bits(),
            rust_result.to_bits(),
        );
    }
}

fn find_c_library(manifest_dir: &Path) -> PathBuf {
    let build_dir = manifest_dir.join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();

    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared object in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.pop().unwrap()
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

    fn channel(&mut self, nonlinear: bool) -> u8 {
        if nonlinear {
            11 + (self.next_u32() % 245) as u8
        } else {
            (self.next_u32() % 11) as u8
        }
    }
}

fn colors_for_mask(rng: &mut Rng, mask: u8) -> (Rgb, Rgb) {
    let mut channels = [0_u8; 6];
    for (index, channel) in channels.iter_mut().enumerate() {
        let nonlinear = mask & (1 << (5 - index)) != 0;
        *channel = rng.channel(nonlinear);
    }

    (
        Rgb {
            R: channels[0],
            G: channels[1],
            B: channels[2],
        },
        Rgb {
            R: channels[3],
            G: channels[4],
            B: channels[5],
        },
    )
}

#[test]
fn all_channel_branch_configurations_match_byte_for_byte() {
    let libraries = Libraries::load();

    for mask in 0_u8..64 {
        let mut rng = Rng::new(0x6a09_e667_f3bc_c909 ^ u64::from(mask));
        for sample in 0..512 {
            let (a, b) = colors_for_mask(&mut rng, mask);
            libraries.assert_same(a, b, &format!("mask={mask:06b}, sample={sample}"));
        }
    }
}

#[test]
fn ordering_singular_and_threshold_configurations_match_byte_for_byte() {
    let libraries = Libraries::load();
    let black = Rgb { R: 0, G: 0, B: 0 };
    let ten = Rgb {
        R: 10,
        G: 10,
        B: 10,
    };
    let eleven = Rgb {
        R: 11,
        G: 11,
        B: 11,
    };

    libraries.assert_same(black, black, "both-black NaN");
    libraries.assert_same(ten, eleven, "byte threshold 10 versus 11");
    libraries.assert_same(eleven, ten, "byte threshold 11 versus 10");

    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    for sample in 0..512 {
        let mut color = Rgb {
            R: rng.next_u32() as u8,
            G: rng.next_u32() as u8,
            B: rng.next_u32() as u8,
        };
        if color.R == 0 && color.G == 0 && color.B == 0 {
            color.R = 1;
        }

        libraries.assert_same(
            black,
            color,
            &format!("LumA < LumB and one-black infinity sample={sample}"),
        );
        libraries.assert_same(
            color,
            black,
            &format!("LumA >= LumB and one-black infinity sample={sample}"),
        );
        libraries.assert_same(
            color,
            color,
            &format!("equal nonzero luminance sample={sample}"),
        );
        libraries.assert_same(ten, color, &format!("threshold byte 10 sample={sample}"));
        libraries.assert_same(eleven, color, &format!("threshold byte 11 sample={sample}"));
    }

    for extrema_mask in 0_u8..64 {
        let channels = std::array::from_fn::<_, 6, _>(|index| {
            if extrema_mask & (1 << (5 - index)) == 0 {
                0
            } else {
                255
            }
        });
        let a = Rgb {
            R: channels[0],
            G: channels[1],
            B: channels[2],
        };
        let b = Rgb {
            R: channels[3],
            G: channels[4],
            B: channels[5],
        };
        libraries.assert_same(a, b, &format!("0/255 extrema mask={extrema_mask:06b}"));
    }
}

#[test]
fn every_byte_value_in_each_channel_position_matches() {
    let libraries = Libraries::load();
    let baseline_a = Rgb {
        R: 17,
        G: 73,
        B: 149,
    };
    let baseline_b = Rgb {
        R: 231,
        G: 41,
        B: 109,
    };

    for position in 0..6 {
        for value in u8::MIN..=u8::MAX {
            let mut a = baseline_a;
            let mut b = baseline_b;
            match position {
                0 => a.R = value,
                1 => a.G = value,
                2 => a.B = value,
                3 => b.R = value,
                4 => b.G = value,
                5 => b.B = value,
                _ => unreachable!(),
            }
            libraries.assert_same(a, b, &format!("position={position}, value={value}"));
        }
    }
}

#[test]
fn broad_fixed_seed_random_surface_matches_byte_for_byte() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for sample in 0..100_000 {
        let a = Rgb {
            R: rng.next_u32() as u8,
            G: rng.next_u32() as u8,
            B: rng.next_u32() as u8,
        };
        let b = Rgb {
            R: rng.next_u32() as u8,
            G: rng.next_u32() as u8,
            B: rng.next_u32() as u8,
        };
        libraries.assert_same(a, b, &format!("broad random sample={sample}"));
    }
}
