use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
#[derive(Clone, Copy)]
struct Bs {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ScaleInfo {
    scf: [f32; 3 * 64],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

type Dequantize = unsafe extern "C" fn(*mut f32, *mut Bs, *mut ScaleInfo, c_int) -> c_int;

#[derive(Clone)]
struct Input {
    bits: Vec<u8>,
    grbuf: Vec<f32>,
    pos: c_int,
    limit: c_int,
    sci: ScaleInfo,
    group_size: c_int,
}

#[derive(Debug)]
struct Outcome {
    ret: c_int,
    pos: c_int,
    limit: c_int,
    grbuf: Vec<u32>,
    bits: Vec<u8>,
    scf: Vec<u32>,
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn range(&mut self, start: u8, end_inclusive: u8) -> u8 {
        start + self.u8() % (end_inclusive - start + 1)
    }
}

fn c_library_path() -> PathBuf {
    let build = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build.display()))
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
        "C shared-library candidates: {candidates:?}"
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdequantize_granule_lib.so")
}

fn with_functions<T>(body: impl FnOnce(Dequantize, Dequantize) -> T) -> T {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing release Rust library: {}; run cargo build --release",
        rust_path.display()
    );

    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rust_lib = Library::new(&rust_path).unwrap();
        let c_symbol: Symbol<Dequantize> = c_lib.get(b"dequantize_granule\0").unwrap();
        let rust_symbol: Symbol<Dequantize> = rust_lib.get(b"dequantize_granule\0").unwrap();
        body(*c_symbol, *rust_symbol)
    }
}

fn allocation_at(sci: &ScaleInfo, index: usize) -> u8 {
    if index < 64 {
        sci.bitalloc[index]
    } else {
        sci.scfcod[index - 64]
    }
}

fn grouped_width(ba: u8) -> c_int {
    let modulus = (2_u32 << (ba - 17)) + 1;
    (modulus + 2 - (modulus >> 3)) as c_int
}

fn consumed_bits(sci: &ScaleInfo, group_size: c_int) -> c_int {
    let mut per_four_groups = 0_i64;
    for i in 0..2 * usize::from(sci.total_bands) {
        let ba = allocation_at(sci, i);
        if (1..17).contains(&ba) {
            per_four_groups += i64::from(ba) * i64::from(group_size.max(0));
        } else if ba >= 17 {
            per_four_groups += i64::from(grouped_width(ba));
        }
    }
    c_int::try_from(per_four_groups * 4).unwrap()
}

fn make_input(
    rng: &mut Rng,
    total_bands: u8,
    group_size: c_int,
    pos: c_int,
    exact_limit: bool,
    mut allocation: impl FnMut(usize, &mut Rng) -> u8,
) -> Input {
    let mut sci = ScaleInfo {
        scf: std::array::from_fn(|_| f32::from_bits(rng.next_u64() as u32)),
        total_bands,
        stereo_bands: rng.u8(),
        bitalloc: [0; 64],
        scfcod: [0; 64],
    };
    for i in 0..2 * usize::from(total_bands) {
        let value = allocation(i, rng);
        if i < 64 {
            sci.bitalloc[i] = value;
        } else {
            sci.scfcod[i - 64] = value;
        }
    }
    for value in &mut sci.scfcod[usize::from(total_bands > 32) * 2..] {
        *value ^= rng.u8();
    }

    let consumed = consumed_bits(&sci, group_size);
    let limit = pos + consumed + if exact_limit { 0 } else { 64 };
    let byte_len = usize::try_from((limit.max(pos) + 7) / 8 + 16)
        .unwrap()
        .max(64);
    let bits = (0..byte_len).map(|_| rng.u8()).collect();
    let grbuf = (0..8192)
        .map(|_| f32::from_bits(rng.next_u64() as u32))
        .collect();

    Input {
        bits,
        grbuf,
        pos,
        limit,
        sci,
        group_size,
    }
}

unsafe fn call(function: Dequantize, input: &Input) -> Outcome {
    let mut grbuf = input.grbuf.clone();
    let bits = input.bits.clone();
    let mut sci = input.sci;
    let mut bs = Bs {
        buf: bits.as_ptr(),
        pos: input.pos,
        limit: input.limit,
    };
    let ret = unsafe { function(grbuf.as_mut_ptr(), &mut bs, &mut sci, input.group_size) };
    Outcome {
        ret,
        pos: bs.pos,
        limit: bs.limit,
        grbuf: grbuf.into_iter().map(f32::to_bits).collect(),
        bits,
        scf: sci.scf.into_iter().map(f32::to_bits).collect(),
        total_bands: sci.total_bands,
        stereo_bands: sci.stereo_bands,
        bitalloc: sci.bitalloc,
        scfcod: sci.scfcod,
    }
}

fn compare_one(c: Dequantize, rust: Dequantize, input: &Input) -> Outcome {
    let c_out = unsafe { call(c, input) };
    let rust_out = unsafe { call(rust, input) };
    assert_eq!(rust_out.ret, c_out.ret, "return mismatch");
    assert_eq!(rust_out.pos, c_out.pos, "bit position mismatch");
    assert_eq!(rust_out.limit, c_out.limit, "bit limit mismatch");
    assert_eq!(rust_out.grbuf, c_out.grbuf, "output bytes mismatch");
    assert_eq!(
        rust_out.bits, c_out.bits,
        "input bit-buffer mutation mismatch"
    );
    assert_eq!(rust_out.scf, c_out.scf, "scale-factor mutation mismatch");
    assert_eq!(
        rust_out.total_bands, c_out.total_bands,
        "total_bands mutation mismatch"
    );
    assert_eq!(
        rust_out.stereo_bands, c_out.stereo_bands,
        "stereo_bands mutation mismatch"
    );
    assert_eq!(
        rust_out.bitalloc, c_out.bitalloc,
        "bitalloc mutation mismatch"
    );
    assert_eq!(rust_out.scfcod, c_out.scfcod, "scfcod mutation mismatch");
    c_out
}

fn randomized(seed: u64, iterations: usize, mut generate: impl FnMut(&mut Rng, usize) -> Input) {
    with_functions(|c, rust| {
        let mut rng = Rng::new(seed);
        for iteration in 0..iterations {
            let input = generate(&mut rng, iteration);
            compare_one(c, rust, &input);
        }
    });
}

macro_rules! config_test {
    ($name:ident, $seed:expr, $iterations:expr, $generator:expr) => {
        #[test]
        fn $name() {
            randomized($seed, $iterations, $generator);
        }
    };
}

config_test!(c01_empty, 0xC01, 64, |rng, _| {
    make_input(rng, 0, 0, 0, false, |_, _| 0)
});

config_test!(c02_no_bands_return_only, 0xC02, 96, |rng, i| {
    make_input(rng, 0, [1, 3, 12][i % 3], (i % 8) as c_int, false, |_, _| 0)
});

config_test!(c03_skipped_allocations, 0xC03, 96, |rng, i| {
    make_input(rng, 1, [1, 3, 12][i % 3], (i % 8) as c_int, false, |_, _| 0)
});

config_test!(c04_direct_one_bit, 0xC04, 128, |rng, i| {
    make_input(rng, 1, [1, 3, 12][i % 3], 0, false, |_, _| 1)
});

config_test!(c05_direct_small_aligned, 0xC05, 160, |rng, i| {
    let ba = 2 + (i % 6) as u8;
    make_input(rng, 1, [1, 3, 12][i % 3], 0, false, move |_, _| ba)
});

config_test!(c06_direct_small_unaligned, 0xC06, 192, |rng, i| {
    let ba = 2 + (i % 6) as u8;
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        1 + (i % 7) as c_int,
        false,
        move |_, _| ba,
    )
});

config_test!(c07_direct_wide, 0xC07, 192, |rng, i| {
    let ba = 8 + (i % 8) as u8;
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        move |_, _| ba,
    )
});

config_test!(c08_direct_sixteen, 0xC08, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 16,
    )
});

config_test!(c09_grouped_seventeen, 0xC09, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 17,
    )
});

config_test!(c10_grouped_eighteen, 0xC10, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 18,
    )
});

config_test!(c11_grouped_nineteen, 0xC11, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 19,
    )
});

config_test!(c12_grouped_twenty, 0xC12, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 20,
    )
});

config_test!(c13_grouped_twenty_one, 0xC13, 128, |rng, i| {
    make_input(
        rng,
        1,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, _| 21,
    )
});

config_test!(c14_many_direct_single, 0xC14, 128, |rng, i| {
    make_input(
        rng,
        1 + (i % 16) as u8,
        1,
        (i % 8) as c_int,
        false,
        |_, rng| rng.range(1, 16),
    )
});

config_test!(c15_many_direct_groups, 0xC15, 128, |rng, i| {
    make_input(
        rng,
        1 + (i % 16) as u8,
        [3, 12][i % 2],
        (i % 8) as c_int,
        false,
        |_, rng| rng.range(1, 16),
    )
});

config_test!(c16_many_grouped, 0xC16, 128, |rng, i| {
    make_input(
        rng,
        1 + (i % 16) as u8,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |_, rng| rng.range(17, 21),
    )
});

config_test!(c17_mixed_allocations, 0xC17, 192, |rng, i| {
    const VALUES: [u8; 12] = [0, 1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(
        rng,
        1 + (i % 24) as u8,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        false,
        |index, rng| VALUES[(index + usize::from(rng.u8())) % VALUES.len()],
    )
});

config_test!(c18_max_bands_single, 0xC18, 96, |rng, i| {
    const VALUES: [u8; 12] = [0, 1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(rng, 32, 1, (i % 8) as c_int, false, |index, _| {
        VALUES[index % VALUES.len()]
    })
});

config_test!(c19_max_bands_groups, 0xC19, 96, |rng, i| {
    const VALUES: [u8; 12] = [0, 1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(
        rng,
        32,
        [3, 12][i % 2],
        (i % 8) as c_int,
        false,
        |index, _| VALUES[index % VALUES.len()],
    )
});

config_test!(c20_exact_limit, 0xC20, 160, |rng, i| {
    const VALUES: [u8; 11] = [1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(
        rng,
        1 + (i % 16) as u8,
        [1, 3, 12][i % 3],
        (i % 8) as c_int,
        true,
        |index, _| VALUES[index % VALUES.len()],
    )
});

config_test!(c21_random_aligned, 0xC21, 256, |rng, i| {
    const VALUES: [u8; 12] = [0, 1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(
        rng,
        (i % 33) as u8,
        [0, 1, 3, 12][i % 4],
        0,
        false,
        |_, rng| VALUES[usize::from(rng.u8()) % VALUES.len()],
    )
});

config_test!(c22_random_unaligned, 0xC22, 256, |rng, i| {
    const VALUES: [u8; 12] = [0, 1, 2, 7, 8, 15, 16, 17, 18, 19, 20, 21];
    make_input(
        rng,
        (i % 33) as u8,
        [0, 1, 3, 12][i % 4],
        1 + (i % 7) as c_int,
        false,
        |_, rng| VALUES[usize::from(rng.u8()) % VALUES.len()],
    )
});

config_test!(c23_zero_group_direct_vs_grouped, 0xC23, 128, |rng, i| {
    let ba = if i % 2 == 0 { 16 } else { 17 + (i % 5) as u8 };
    make_input(
        rng,
        1 + (i % 16) as u8,
        0,
        (i % 8) as c_int,
        false,
        move |_, _| ba,
    )
});

#[test]
fn e01_reads_past_limit_yield_zero_and_advance() {
    with_functions(|c, rust| {
        let mut rng = Rng::new(0xE01);
        for i in 0..128 {
            let mut input = make_input(
                &mut rng,
                1 + (i % 8) as u8,
                [1, 3, 12][i % 3],
                (i % 8) as c_int,
                false,
                |_, rng| rng.range(1, 21),
            );
            input.limit = input.pos;
            let out = compare_one(c, rust, &input);
            assert_eq!(out.ret, input.group_size * 4);
            assert!(out.pos > out.limit);
        }
    });
}

#[test]
fn g04_zero_group_size() {
    randomized(0xE040, 128, |rng, i| {
        make_input(
            rng,
            1 + (i % 32) as u8,
            0,
            (i % 8) as c_int,
            false,
            |_, rng| rng.range(0, 21),
        )
    });
}

#[test]
fn g05_zero_total_bands() {
    randomized(0xE050, 128, |rng, i| {
        make_input(
            rng,
            0,
            [0, 1, 3, 12, 128][i % 5],
            (i % 8) as c_int,
            false,
            |_, _| 0,
        )
    });
}

#[test]
fn g06_zero_bit_limit() {
    with_functions(|c, rust| {
        let mut rng = Rng::new(0xE060);
        for i in 0..128 {
            let mut input = make_input(&mut rng, 1, [1, 3, 12][i % 3], 0, false, |_, rng| {
                rng.range(1, 21)
            });
            input.limit = 0;
            compare_one(c, rust, &input);
        }
    });
}

#[test]
fn g07_very_large_limit() {
    with_functions(|c, rust| {
        let mut rng = Rng::new(0xE070);
        for i in 0..128 {
            let mut input = make_input(
                &mut rng,
                1 + (i % 16) as u8,
                [1, 3, 12][i % 3],
                (i % 8) as c_int,
                false,
                |_, rng| rng.range(0, 21),
            );
            input.limit = c_int::MAX;
            compare_one(c, rust, &input);
        }
    });
}

#[test]
fn g08_maximum_in_array_bands() {
    randomized(0xE080, 128, |rng, i| {
        make_input(
            rng,
            32,
            [1, 3, 12][i % 3],
            (i % 8) as c_int,
            false,
            |_, rng| rng.range(0, 21),
        )
    });
}

#[test]
fn g09_one_past_bitalloc_capacity() {
    randomized(0xE090, 128, |rng, i| {
        make_input(
            rng,
            33,
            [1, 3, 12][i % 3],
            (i % 8) as c_int,
            false,
            |index, rng| {
                if index >= 64 {
                    [1, 17][index - 64]
                } else {
                    rng.range(0, 21)
                }
            },
        )
    });
}

#[test]
fn g10_oversized_allocated_group() {
    randomized(0xE100, 64, |rng, i| {
        make_input(
            rng,
            1 + (i % 4) as u8,
            128,
            (i % 8) as c_int,
            false,
            |_, rng| rng.range(0, 21),
        )
    });
}

#[test]
fn ffi_boundary_child() {
    let Ok(path) = std::env::var("FFI_BOUNDARY_LIBRARY") else {
        return;
    };
    let case = std::env::var("FFI_BOUNDARY_CASE").unwrap();
    unsafe {
        let library = Library::new(path).unwrap();
        let symbol: Symbol<Dequantize> = library.get(b"dequantize_granule\0").unwrap();
        let function = *symbol;
        let bits = [0xA5_u8; 64];
        let mut grbuf = [0.0_f32; 2048];
        let mut bs = Bs {
            buf: bits.as_ptr(),
            pos: 0,
            limit: 512,
        };
        let mut sci = ScaleInfo {
            scf: [0.0; 192],
            total_bands: 1,
            stereo_bands: 0,
            bitalloc: [0; 64],
            scfcod: [0; 64],
        };
        sci.bitalloc[0] = 4;
        match case.as_str() {
            "grbuf" => {
                function(std::ptr::null_mut(), &mut bs, &mut sci, 1);
            }
            "bs" => {
                function(grbuf.as_mut_ptr(), std::ptr::null_mut(), &mut sci, 1);
            }
            "sci" => {
                function(grbuf.as_mut_ptr(), &mut bs, std::ptr::null_mut(), 1);
            }
            _ => panic!("unknown boundary case"),
        }
    }
}

fn boundary_status(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("ffi_boundary_child")
        .arg("--exact")
        .arg("--nocapture")
        .env("FFI_BOUNDARY_LIBRARY", library)
        .env("FFI_BOUNDARY_CASE", case)
        .status()
        .unwrap()
}

fn assert_same_null_boundary(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = boundary_status(&c_library_path(), case);
    let rust_status = boundary_status(&rust_library_path(), case);
    assert!(!c_status.success(), "C unexpectedly accepted null {case}");
    assert!(
        !rust_status.success(),
        "Rust unexpectedly accepted null {case}"
    );
    assert_eq!(
        (rust_status.code(), rust_status.signal()),
        (c_status.code(), c_status.signal()),
        "different process-level rejection for null {case}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
fn g01_null_grbuf() {
    assert_same_null_boundary("grbuf");
}

#[test]
fn g02_null_bs() {
    assert_same_null_boundary("bs");
}

#[test]
fn g03_null_sci() {
    assert_same_null_boundary("sci");
}
