use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

type Tfm = unsafe extern "C" fn(*mut f32, *const f32, c_int);

const CASES: usize = 1_024;

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-rjuFwk.so")
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libtfm_lib.so")
}

fn with_apis<R>(body: impl FnOnce(Tfm, Tfm) -> R) -> R {
    unsafe {
        let c_library = Library::new(c_library_path()).expect("load C shared library");
        let rust_library = Library::new(rust_library_path()).expect("load Rust shared library");
        let c_symbol: Symbol<Tfm> = c_library.get(b"tfm\0").expect("load C tfm");
        let rust_symbol: Symbol<Tfm> = rust_library.get(b"tfm\0").expect("load Rust tfm");
        body(*c_symbol, *rust_symbol)
    }
}

fn from_bits(bits: &[u32]) -> Vec<f32> {
    bits.iter().copied().map(f32::from_bits).collect()
}

fn to_bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn compare_disjoint(src_bits: &[u32], count: c_int, dest_bits: &[u32], label: &str) {
    with_apis(|c_tfm, rust_tfm| {
        let c_src = from_bits(src_bits);
        let rust_src = from_bits(src_bits);
        let mut c_dest = from_bits(dest_bits);
        let mut rust_dest = from_bits(dest_bits);

        unsafe {
            c_tfm(c_dest.as_mut_ptr(), c_src.as_ptr(), count);
            rust_tfm(rust_dest.as_mut_ptr(), rust_src.as_ptr(), count);
        }

        assert_eq!(
            to_bits(&c_dest),
            to_bits(&rust_dest),
            "{label}: destination"
        );
        assert_eq!(to_bits(&c_src), to_bits(&rust_src), "{label}: source");
    });
}

fn compare_overlap(
    initial_bits: &[u32],
    dest_offset: usize,
    src_offset: usize,
    count: c_int,
    label: &str,
) {
    with_apis(|c_tfm, rust_tfm| {
        let mut c_buffer = from_bits(initial_bits);
        let mut rust_buffer = from_bits(initial_bits);

        unsafe {
            c_tfm(
                c_buffer.as_mut_ptr().add(dest_offset),
                c_buffer.as_ptr().add(src_offset),
                count,
            );
            rust_tfm(
                rust_buffer.as_mut_ptr().add(dest_offset),
                rust_buffer.as_ptr().add(src_offset),
                count,
            );
        }

        assert_eq!(to_bits(&c_buffer), to_bits(&rust_buffer), "{label}");
    });
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn finite(&mut self) -> f32 {
        let magnitude = (self.next_u32() % 2_000_001) as i32 - 1_000_000;
        magnitude as f32 / 64.0
    }

    fn finite_bits(&mut self) -> u32 {
        loop {
            let bits = self.next_u32();
            if bits & 0x7f80_0000 != 0x7f80_0000 {
                return bits;
            }
        }
    }

    fn nan_bits(&mut self) -> u32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x007f_ffff).max(1);
        sign | 0x7f80_0000 | payload
    }
}

#[cfg(target_arch = "x86_64")]
mod scalar {
    use std::arch::asm;

    pub fn add(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "addss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }

    pub fn sub(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "subss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }

    pub fn mul(mut left: f32, right: f32) -> f32 {
        unsafe {
            asm!(
                "mulss {left}, {right}",
                left = inout(xmm_reg) left,
                right = in(xmm_reg) right,
                options(nomem, nostack, preserves_flags)
            );
        }
        left
    }
}

#[cfg(target_arch = "x86_64")]
fn c_ordered_sqd(first: f32, second: f32, dxy: f32) -> f32 {
    let (dx2, dy2) = if first < second {
        (first, second)
    } else {
        (second, first)
    };
    let mut sqd = scalar::mul(dy2, dy2);
    sqd = scalar::sub(sqd, scalar::mul(scalar::add(dx2, dx2), dy2));
    sqd = scalar::add(sqd, scalar::mul(dx2, dx2));
    scalar::add(scalar::mul(scalar::mul(4.0, dxy), dxy), sqd)
}

#[test]
fn c1_zero_count_preserves_buffers() {
    let mut rng = Rng::new(0xc100_0000_0000_0001);
    for case in 0..CASES {
        let src = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let dest = [rng.next_u32(), rng.next_u32()];
        compare_disjoint(&src, 0, &dest, &format!("C1 case {case}"));
    }
}

#[test]
fn c2_single_less_finite_nonnegative_sqd() {
    let mut rng = Rng::new(0xc200_0000_0000_0002);
    for case in 0..CASES {
        let mut first = rng.finite();
        let mut second = rng.finite();
        if first >= second {
            std::mem::swap(&mut first, &mut second);
        }
        if first == second {
            second = first + 1.0;
        }
        let src = [first.to_bits(), second.to_bits(), rng.finite().to_bits()];
        compare_disjoint(
            &src,
            1,
            &[0xdead_beef, 0xfeed_face],
            &format!("C2 case {case}"),
        );
    }
}

#[test]
fn c3_single_greater_finite_nonnegative_sqd() {
    let mut rng = Rng::new(0xc300_0000_0000_0003);
    for case in 0..CASES {
        let mut first = rng.finite();
        let mut second = rng.finite();
        if first <= second {
            std::mem::swap(&mut first, &mut second);
        }
        if first == second {
            first = second + 1.0;
        }
        let src = [first.to_bits(), second.to_bits(), rng.finite().to_bits()];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C3 case {case}"),
        );
    }
}

#[test]
fn c4_single_equal_values() {
    let mut rng = Rng::new(0xc400_0000_0000_0004);
    for case in 0..CASES {
        let equal = rng.finite();
        let src = [equal.to_bits(), equal.to_bits(), rng.finite().to_bits()];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C4 case {case}"),
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[test]
fn c5_single_negative_rounded_sqd_clamps() {
    let mut rng = Rng::new(0xc500_0000_0000_0005);
    let mut matched = 0;
    for attempt in 0..5_000_000 {
        let first = f32::from_bits(rng.finite_bits());
        let second = f32::from_bits(rng.finite_bits());
        let dxy = f32::from_bits(rng.finite_bits() & 0x807f_ffff);
        let sqd = c_ordered_sqd(first, second, dxy);
        if sqd < 0.0 {
            let src = [first.to_bits(), second.to_bits(), dxy.to_bits()];
            compare_disjoint(
                &src,
                1,
                &[rng.next_u32(), rng.next_u32()],
                &format!("C5 attempt {attempt}"),
            );
            matched += 1;
            if matched == 256 {
                break;
            }
        }
    }
    assert!(
        matched >= 32,
        "found only {matched} rounded-negative sqd cases"
    );
}

#[test]
fn c6_signed_zero_and_subnormals() {
    let mut rng = Rng::new(0xc600_0000_0000_0006);
    let special = [0, 0x8000_0000, 1, 0x8000_0001, 0x007f_ffff, 0x807f_ffff];
    for case in 0..CASES {
        let src = [
            special[rng.next_u32() as usize % special.len()],
            special[rng.next_u32() as usize % special.len()],
            special[rng.next_u32() as usize % special.len()],
        ];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C6 case {case}"),
        );
    }
}

#[test]
fn c7_infinity_and_overflow() {
    let mut rng = Rng::new(0xc700_0000_0000_0007);
    let special = [
        0x7f80_0000,
        0xff80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f00_0000,
        0xff00_0000,
    ];
    for case in 0..CASES {
        let src = [
            special[rng.next_u32() as usize % special.len()],
            special[rng.next_u32() as usize % special.len()],
            special[rng.next_u32() as usize % special.len()],
        ];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C7 case {case}"),
        );
    }
}

#[test]
fn c8_nan_in_first_operand() {
    let mut rng = Rng::new(0xc800_0000_0000_0008);
    for case in 0..CASES {
        let src = [rng.nan_bits(), rng.next_u32(), rng.next_u32()];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C8 case {case}"),
        );
    }
}

#[test]
fn c9_nan_in_second_operand() {
    let mut rng = Rng::new(0xc900_0000_0000_0009);
    for case in 0..CASES {
        let src = [rng.next_u32(), rng.nan_bits(), rng.next_u32()];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C9 case {case}"),
        );
    }
}

#[test]
fn c10_nan_in_dxy() {
    let mut rng = Rng::new(0xca00_0000_0000_000a);
    for case in 0..CASES {
        let src = [rng.next_u32(), rng.next_u32(), rng.nan_bits()];
        compare_disjoint(
            &src,
            1,
            &[rng.next_u32(), rng.next_u32()],
            &format!("C10 case {case}"),
        );
    }
}

#[test]
fn c11_many_mixed_records() {
    let mut rng = Rng::new(0xcb00_0000_0000_000b);
    for case in 0..256 {
        let count = 2 + (rng.next_u32() % 63) as usize;
        let src: Vec<u32> = (0..count * 3).map(|_| rng.next_u32()).collect();
        let dest: Vec<u32> = (0..count * 2).map(|_| rng.next_u32()).collect();
        compare_disjoint(&src, count as c_int, &dest, &format!("C11 case {case}"));
    }
}

#[test]
fn c12_single_exact_in_place() {
    let mut rng = Rng::new(0xcc00_0000_0000_000c);
    for case in 0..CASES {
        let initial = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        compare_overlap(&initial, 0, 0, 1, &format!("C12 case {case}"));
    }
}

#[test]
fn c13_many_exact_in_place() {
    let mut rng = Rng::new(0xcd00_0000_0000_000d);
    for case in 0..256 {
        let count = 2 + (rng.next_u32() % 63) as usize;
        let initial: Vec<u32> = (0..count * 3).map(|_| rng.next_u32()).collect();
        compare_overlap(&initial, 0, 0, count as c_int, &format!("C13 case {case}"));
    }
}

#[test]
fn c14_many_partial_overlap() {
    let mut rng = Rng::new(0xce00_0000_0000_000e);
    for case in 0..256 {
        let count = 2 + (rng.next_u32() % 31) as usize;
        let len = count * 3 + 4;
        let initial: Vec<u32> = (0..len).map(|_| rng.next_u32()).collect();
        compare_overlap(
            &initial,
            2,
            0,
            count as c_int,
            &format!("C14 forward {case}"),
        );
        compare_overlap(
            &initial,
            0,
            2,
            count as c_int,
            &format!("C14 backward {case}"),
        );
    }
}

#[test]
fn b1_b2_nonpositive_counts_accept_null_pointers() {
    with_apis(|c_tfm, rust_tfm| unsafe {
        for count in [0, -1, c_int::MIN] {
            c_tfm(std::ptr::null_mut(), std::ptr::null(), count);
            rust_tfm(std::ptr::null_mut(), std::ptr::null(), count);
        }
    });
}

fn child_status(library: &str, scenario: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_ub_child")
        .arg("--nocapture")
        .env("TFM_CHILD_LIBRARY", library)
        .env("TFM_CHILD_SCENARIO", scenario)
        .status()
        .expect("run UB child")
}

#[cfg(unix)]
fn comparable_termination(status: ExitStatus) -> (bool, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.success(), status.signal())
}

#[test]
fn b3_b4_b5_invalid_positive_ranges_terminate_identically() {
    for scenario in ["null-dest", "null-src", "oversized"] {
        let c_status = child_status("c", scenario);
        let rust_status = child_status("rust", scenario);
        assert_eq!(
            comparable_termination(c_status),
            comparable_termination(rust_status),
            "{scenario}"
        );
        assert!(
            !c_status.success(),
            "{scenario}: C unexpectedly returned normally"
        );
    }
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 2;
const MAP_ANONYMOUS: c_int = 0x20;

unsafe extern "C" {
    fn mmap(
        address: *mut c_void,
        length: usize,
        protection: c_int,
        flags: c_int,
        fd: c_int,
        offset: isize,
    ) -> *mut c_void;
    fn mprotect(address: *mut c_void, length: usize, protection: c_int) -> c_int;
    fn munmap(address: *mut c_void, length: usize) -> c_int;
}

unsafe fn guarded_tail(float_count: usize) -> (*mut c_void, usize, *mut f32) {
    let page = 4096usize;
    let length = page * 2;
    let mapping = unsafe {
        mmap(
            std::ptr::null_mut(),
            length,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(mapping as isize, -1, "mmap failed");
    assert_eq!(
        unsafe { mprotect((mapping as *mut u8).add(page).cast(), page, PROT_NONE) },
        0,
        "mprotect failed"
    );
    let tail = unsafe {
        (mapping as *mut u8)
            .add(page - float_count * std::mem::size_of::<f32>())
            .cast::<f32>()
    };
    (mapping, length, tail)
}

#[test]
fn ffi_ub_child() {
    let Ok(library_choice) = std::env::var("TFM_CHILD_LIBRARY") else {
        return;
    };
    let scenario = std::env::var("TFM_CHILD_SCENARIO").expect("child scenario");
    let library_path = if library_choice == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };

    unsafe {
        let library = Library::new(library_path).expect("load child library");
        let symbol: Symbol<Tfm> = library.get(b"tfm\0").expect("load child tfm");
        match scenario.as_str() {
            "null-dest" => {
                let src = [1.0f32, 2.0, 3.0];
                symbol(std::ptr::null_mut(), src.as_ptr(), 1);
            }
            "null-src" => {
                let mut dest = [0.0f32; 2];
                symbol(dest.as_mut_ptr(), std::ptr::null(), 1);
            }
            "oversized" => {
                let (src_mapping, src_length, src) = guarded_tail(3);
                let (dest_mapping, dest_length, dest) = guarded_tail(2);
                src.write(1.0);
                src.add(1).write(2.0);
                src.add(2).write(3.0);
                symbol(dest, src, 2);
                munmap(src_mapping, src_length);
                munmap(dest_mapping, dest_length);
            }
            other => panic!("unknown child scenario {other}"),
        }
    }
}
