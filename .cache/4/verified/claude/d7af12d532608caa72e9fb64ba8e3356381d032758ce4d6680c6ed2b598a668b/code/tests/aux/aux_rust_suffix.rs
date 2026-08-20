// ===========================================================================
// Auxiliary Rust shim used ONLY by the differential test-suite.
//
// The test harness concatenates `src/lib.rs` (verbatim, unmodified) with this
// file into `$CARGO_TARGET_TMPDIR/aux_rust.rs` and compiles the result with
// `rustc --crate-type cdylib`.  That mirrors `tests/aux/aux_c.c`, which
// `#include`s `c_src/src/lib.c`: both shims expose the *private* / `static`
// internals of the respective translation unit so the differential test can
// compare them.  Neither shim changes the shipped libraries.
// ===========================================================================

const AUX_BAD_WHICH: c_int = 0x7EC0FFEE;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aux_predict_sample(
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe { BTAC1C2_PredictSample(psamp, idx, pfcn, ridx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aux_pfn(
    which: c_int,
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        match which {
            0 => BTAC1C2_PredictSample_Pfn0(psamp, idx, pfcn, ridx),
            1 => BTAC1C2_PredictSample_Pfn1(psamp, idx, pfcn, ridx),
            2 => BTAC1C2_PredictSample_Pfn2(psamp, idx, pfcn, ridx),
            3 => BTAC1C2_PredictSample_Pfn3(psamp, idx, pfcn, ridx),
            4 => BTAC1C2_PredictSample_Pfn4(psamp, idx, pfcn, ridx),
            5 => BTAC1C2_PredictSample_Pfn5(psamp, idx, pfcn, ridx),
            6 => BTAC1C2_PredictSample_Pfn6(psamp, idx, pfcn, ridx),
            7 => BTAC1C2_PredictSample_Pfn7(psamp, idx, pfcn, ridx),
            8 => BTAC1C2_PredictSample_Pfn8(psamp, idx, pfcn, ridx),
            9 => BTAC1C2_PredictSample_Pfn9(psamp, idx, pfcn, ridx),
            10 => BTAC1C2_PredictSample_Pfn10(psamp, idx, pfcn, ridx),
            11 => BTAC1C2_PredictSample_Pfn11(psamp, idx, pfcn, ridx),
            _ => AUX_BAD_WHICH,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aux_getpredict_call(
    sel: c_int,
    psamp: *mut c_int,
    idx: c_int,
    pfcn: c_int,
    ridx: *mut btac1c_idxstate,
) -> c_int {
    unsafe {
        let p = BTAC1C2_GetPredictFunc(sel);
        let f: PredictFn = core::mem::transmute::<*mut c_void, PredictFn>(p);
        f(psamp, idx, pfcn, ridx)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn aux_getpredict_is_null(sel: c_int) -> c_int {
    (BTAC1C2_GetPredictFunc(sel) == core::ptr::null_mut()) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn aux_getpredict_identity(sel: c_int) -> c_int {
    let f = BTAC1C2_GetPredictFunc(sel);
    let mut bits: c_int = 0;
    if f == fptr(BTAC1C2_PredictSample_Pfn0) {
        bits |= 1 << 0;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn1) {
        bits |= 1 << 1;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn2) {
        bits |= 1 << 2;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn3) {
        bits |= 1 << 3;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn4) {
        bits |= 1 << 4;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn5) {
        bits |= 1 << 5;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn6) {
        bits |= 1 << 6;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn7) {
        bits |= 1 << 7;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn8) {
        bits |= 1 << 8;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn9) {
        bits |= 1 << 9;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn10) {
        bits |= 1 << 10;
    }
    if f == fptr(BTAC1C2_PredictSample_Pfn11) {
        bits |= 1 << 11;
    }
    if f == fptr(BTAC1C2_PredictSample) {
        bits |= 1 << 12;
    }
    bits
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aux_layout(out: *mut usize) {
    unsafe {
        *out.add(0) = core::mem::size_of::<btac1c_idxstate>();
        *out.add(1) = core::mem::align_of::<btac1c_idxstate>();
        *out.add(2) = core::mem::offset_of!(btac1c_idxstate, idx);
        *out.add(3) = core::mem::offset_of!(btac1c_idxstate, lpred);
        *out.add(4) = core::mem::offset_of!(btac1c_idxstate, rpred);
        *out.add(5) = core::mem::offset_of!(btac1c_idxstate, tag);
        *out.add(6) = core::mem::offset_of!(btac1c_idxstate, bcfcn);
        *out.add(7) = core::mem::offset_of!(btac1c_idxstate, bsfcn);
        *out.add(8) = core::mem::offset_of!(btac1c_idxstate, usefx);
        *out.add(9) = core::mem::offset_of!(btac1c_idxstate, firfx);
        *out.add(10) = core::mem::size_of::<btac1c_u16>();
        *out.add(11) = core::mem::size_of::<btac1c_s16>();
        *out.add(12) = core::mem::size_of::<btac1c_byte>();
        *out.add(13) = core::mem::size_of::<c_int>();
    }
}
