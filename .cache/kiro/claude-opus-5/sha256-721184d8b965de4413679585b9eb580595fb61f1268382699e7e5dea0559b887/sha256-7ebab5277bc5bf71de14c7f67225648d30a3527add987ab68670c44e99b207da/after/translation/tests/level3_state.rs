//! Level 3: `compute_checksum`, `init_state`, `apply_operation`.
//!
//! These take pointers, so the struct layout and the exact bytes written into
//! caller memory are compared as well as the printed output.

mod common;

use common::*;
use std::ffi::{c_int, c_uint};

fn compute_state_layout() {
    assert_eq!(size_of::<ComputeState>(), 12, "ComputeState size");
    assert_eq!(align_of::<ComputeState>(), 4, "ComputeState alignment");
}

fn checksum_case(values: &[c_int], count: c_int) {
    let libs = impls();
    let c: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::C, "compute_checksum");
    let r: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::Rust, "compute_checksum");

    // Separate buffers so a stray write by one side cannot influence the other.
    let mut cbuf = values.to_vec();
    let mut rbuf = values.to_vec();

    let (cv, cout) = capture_stdout(|| unsafe { c(cbuf.as_mut_ptr(), count) });
    let (rv, rout) = capture_stdout(|| unsafe { r(rbuf.as_mut_ptr(), count) });

    assert_eq!(
        cv, rv,
        "compute_checksum({values:?}, {count}): C=0x{cv:08X} Rust=0x{rv:08X}"
    );
    assert_eq!(cout, rout, "compute_checksum stdout differs");
    assert!(cout.is_empty(), "compute_checksum printed: {}", show(&cout));
    assert_eq!(cbuf, rbuf, "compute_checksum must not modify its input");
    assert_eq!(cv & !0xFFFF, 0, "C contract: result is masked to 16 bits");
}

fn compute_checksum_matches() {
    // The C code copies min(count, 4) ints, so always hand it at least 4.
    let vals = sample_ints();
    for &a in &vals {
        for &b in &vals {
            checksum_case(&[a, b, a ^ b, a.wrapping_add(b)], 4);
        }
    }
}

fn compute_checksum_all_counts() {
    let vals = sample_ints();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    // Provide 8 ints of backing storage so even count>4 (clamped to 4) is safe.
    for _ in 0..3000 {
        let buf: Vec<c_int> = (0..8)
            .map(|_| vals[(rng.next_u64() as usize) % vals.len()])
            .collect();
        for count in [
            c_int::MIN,
            -100,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            100,
            c_int::MAX,
        ] {
            checksum_case(&buf, count);
        }
    }
}

fn compute_checksum_null_pointer_matches() {
    let libs = impls();
    let c: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::C, "compute_checksum");
    let r: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::Rust, "compute_checksum");

    for count in [c_int::MIN, -1, 0, 1, 4, 5, c_int::MAX] {
        let (cv, cout) = capture_stdout(|| unsafe { c(std::ptr::null_mut(), count) });
        let (rv, rout) = capture_stdout(|| unsafe { r(std::ptr::null_mut(), count) });
        assert_eq!(cv, rv, "compute_checksum(NULL, {count})");
        assert_eq!(cout, rout, "compute_checksum(NULL, {count}) stdout");
        assert_eq!(cv, 0, "C contract: NULL input yields 0");
    }
}

/// Byte patterns chosen to exercise the `checksum = (checksum << 1) ^ byte`
/// accumulation, including patterns whose high bits shift out of the 16-bit mask.
fn compute_checksum_byte_patterns() {
    let patterns: [u8; 8] = [0x00, 0x01, 0x7F, 0x80, 0xAA, 0x55, 0xFE, 0xFF];
    for &p0 in &patterns {
        for &p1 in &patterns {
            for &p2 in &patterns {
                let mut bytes = [0u8; 32];
                for (i, b) in bytes.iter_mut().enumerate() {
                    *b = match i % 3 {
                        0 => p0,
                        1 => p1,
                        _ => p2,
                    };
                }
                let vals: Vec<c_int> = bytes
                    .chunks_exact(4)
                    .map(|c| c_int::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                for count in 1..=5 {
                    checksum_case(&vals, count);
                }
            }
        }
    }
}

fn probe_state() -> ComputeState {
    // Distinctive filler so we can see exactly which fields get overwritten.
    ComputeState {
        accumulator: 0x1111_1111u32 as c_int,
        operation_count: 0x2222_2222u32 as c_int,
        checksum: 0x3333_3333,
    }
}

fn init_state_matches() {
    let libs = impls();
    let c: libloading::Symbol<FnInitState> = libs.sym(Which::C, "init_state");
    let r: libloading::Symbol<FnInitState> = libs.sym(Which::Rust, "init_state");

    for v in sample_ints() {
        let mut cs = probe_state();
        let mut rs = probe_state();
        let (_, cout) = capture_stdout(|| unsafe { c(&mut cs, v) });
        let (_, rout) = capture_stdout(|| unsafe { r(&mut rs, v) });
        assert_eq!(cs, rs, "init_state({v}) wrote different state");
        assert_eq!(
            cout,
            rout,
            "init_state({v}) stdout:\nC   ={}\nRust={}",
            show(&cout),
            show(&rout)
        );
        // Raw byte comparison of the whole struct, padding included.
        let cb: [u8; 12] = unsafe { std::mem::transmute(cs) };
        let rb: [u8; 12] = unsafe { std::mem::transmute(rs) };
        assert_eq!(cb, rb, "init_state({v}) raw bytes differ");
    }
}

fn init_state_null_matches() {
    let libs = impls();
    let c: libloading::Symbol<FnInitState> = libs.sym(Which::C, "init_state");
    let r: libloading::Symbol<FnInitState> = libs.sym(Which::Rust, "init_state");
    let (_, cout) = capture_stdout(|| unsafe { c(std::ptr::null_mut(), 7) });
    let (_, rout) = capture_stdout(|| unsafe { r(std::ptr::null_mut(), 7) });
    assert_eq!(
        cout,
        rout,
        "init_state(NULL) stdout:\nC   ={}\nRust={}",
        show(&cout),
        show(&rout)
    );
}

fn apply_operation_matches() {
    let libs = impls();
    let c_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");
    let c_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::C, "apply_operation");
    let r_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::Rust, "apply_operation");

    let vals = sample_ints();
    for opcode in 0..4 {
        let cf = unsafe { c_get(opcode) };
        let rf = unsafe { r_get(opcode) };
        for &acc in &vals {
            for &v in &vals {
                let mut cs = ComputeState {
                    accumulator: acc,
                    operation_count: 5,
                    checksum: 0xBEEF,
                };
                let mut rs = cs;
                let (_, cout) = capture_stdout(|| unsafe { c_ap(&mut cs, v, cf) });
                let (_, rout) = capture_stdout(|| unsafe { r_ap(&mut rs, v, rf) });
                assert_eq!(cs, rs, "apply_operation(op{opcode}, acc={acc}, v={v})");
                assert_eq!(cout, rout, "apply_operation(op{opcode}) stdout differs");
            }
        }
    }
}

/// The C code increments `operation_count` unconditionally; make sure the
/// wraparound at INT_MAX behaves identically.
fn apply_operation_count_overflow_matches() {
    let libs = impls();
    let c_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");
    let c_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::C, "apply_operation");
    let r_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::Rust, "apply_operation");

    for start in [c_int::MAX, c_int::MAX - 1, -1, c_int::MIN] {
        for opcode in 0..4 {
            let mut cs = ComputeState {
                accumulator: 3,
                operation_count: start,
                checksum: 0,
            };
            let mut rs = cs;
            unsafe { c_ap(&mut cs, 4, c_get(opcode)) };
            unsafe { r_ap(&mut rs, 4, r_get(opcode)) };
            assert_eq!(cs, rs, "operation_count overflow from {start}, op{opcode}");
        }
    }
}

fn apply_operation_null_cases_match() {
    let libs = impls();
    let c_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");
    let c_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::C, "apply_operation");
    let r_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::Rust, "apply_operation");

    // NULL state (with a valid func)
    let (_, cout) = capture_stdout(|| unsafe { c_ap(std::ptr::null_mut(), 1, c_get(0)) });
    let (_, rout) = capture_stdout(|| unsafe { r_ap(std::ptr::null_mut(), 1, r_get(0)) });
    assert_eq!(cout, rout, "apply_operation(NULL state) stdout differs");

    // NULL func (with a valid state) - state must be left untouched
    let mut cs = probe_state();
    let mut rs = probe_state();
    let (_, cout) = capture_stdout(|| unsafe { c_ap(&mut cs, 1, None) });
    let (_, rout) = capture_stdout(|| unsafe { r_ap(&mut rs, 1, None) });
    assert_eq!(cout, rout, "apply_operation(NULL func) stdout differs");
    assert_eq!(cs, rs, "apply_operation(NULL func) state differs");
    assert_eq!(cs, probe_state(), "NULL func must not touch state");

    // Both NULL: the state check comes first in the C source.
    let (_, cout) = capture_stdout(|| unsafe { c_ap(std::ptr::null_mut(), 1, None) });
    let (_, rout) = capture_stdout(|| unsafe { r_ap(std::ptr::null_mut(), 1, None) });
    assert_eq!(cout, rout, "apply_operation(NULL, NULL) stdout differs");
}

/// Chained application, mirroring how `checkshift` drives the state machine.
fn apply_operation_sequences_match() {
    let libs = impls();
    let c_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");
    let c_init: libloading::Symbol<FnInitState> = libs.sym(Which::C, "init_state");
    let r_init: libloading::Symbol<FnInitState> = libs.sym(Which::Rust, "init_state");
    let c_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::C, "apply_operation");
    let r_ap: libloading::Symbol<FnApplyOperation> = libs.sym(Which::Rust, "apply_operation");

    let mut rng = Rng::new(0x0BAD_F00D_0000_0001);
    for _ in 0..500 {
        let init = rng.next_i32();
        let mut cs = ComputeState::default();
        let mut rs = ComputeState::default();
        let steps: Vec<(c_int, c_int)> = (0..12)
            .map(|_| ((rng.next_u64() % 5) as c_int - 1, rng.next_i32()))
            .collect();

        let (_, cout) = capture_stdout(|| unsafe {
            c_init(&mut cs, init);
            for &(op, v) in &steps {
                c_ap(&mut cs, v, c_get(op));
            }
        });
        let (_, rout) = capture_stdout(|| unsafe {
            r_init(&mut rs, init);
            for &(op, v) in &steps {
                r_ap(&mut rs, v, r_get(op));
            }
        });
        assert_eq!(cs, rs, "sequence from init={init}, steps={steps:?}");
        assert_eq!(cout, rout, "sequence stdout differs (init={init})");
    }
}

/// Guard the checksum field width: `unsigned int` must round-trip unchanged.
fn checksum_field_is_unsigned() {
    let libs = impls();
    let c: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::C, "compute_checksum");
    let r: libloading::Symbol<FnComputeChecksum> = libs.sym(Which::Rust, "compute_checksum");
    let mut buf: Vec<c_int> = vec![-1, -1, -1, -1];
    let cv: c_uint = unsafe { c(buf.as_mut_ptr(), 4) };
    let rv: c_uint = unsafe { r(buf.as_mut_ptr(), 4) };
    assert_eq!(cv, rv, "all-0xFF input");
}

fn main() {
    let mut r = Runner::new();
    r.case("compute_state_layout", compute_state_layout);
    r.case("compute_checksum_matches", compute_checksum_matches);
    r.case("compute_checksum_all_counts", compute_checksum_all_counts);
    r.case("compute_checksum_null_pointer_matches", compute_checksum_null_pointer_matches);
    r.case("compute_checksum_byte_patterns", compute_checksum_byte_patterns);
    r.case("init_state_matches", init_state_matches);
    r.case("init_state_null_matches", init_state_null_matches);
    r.case("apply_operation_matches", apply_operation_matches);
    r.case("apply_operation_count_overflow_matches", apply_operation_count_overflow_matches);
    r.case("apply_operation_null_cases_match", apply_operation_null_cases_match);
    r.case("apply_operation_sequences_match", apply_operation_sequences_match);
    r.case("checksum_field_is_unsigned", checksum_field_is_unsigned);
    r.finish();
}
