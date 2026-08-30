/*
 * Test-only shim for differential testing.
 *
 * c_src is READ-ONLY and is never modified.  This file lives outside c_src and
 * textually includes the original translation unit so that the `static`
 * functions inside it become reachable from a tiny exported dispatcher with the
 * exact same shape as the Rust crate's `__difftest_predict` hook.
 *
 * Everything below the #include is additive; the included C is untouched.
 */
#include "../../c_src/src/lib.c"

/* Mirrors:
 *   #[cfg(feature = "difftest")] #[no_mangle]
 *   pub unsafe extern "C" fn __difftest_predict(
 *       which: c_int, psamp: *mut c_int, idx: c_int, pfcn: c_int,
 *       ridx: *mut btac1c_idxstate) -> c_int
 *
 * `which` selects the lowest-level entry point:
 *   0..=11  -> BTAC1C2_PredictSample_Pfn{which}
 *   other   -> BTAC1C2_PredictSample  (the generic switch-based dispatcher)
 */
int __difftest_predict(int which, int *psamp, int idx, int pfcn,
                       btac1c_idxstate *ridx) {
    int (*f)(int *, int, int, btac1c_idxstate *);
    switch (which) {
    case 0:  f = BTAC1C2_PredictSample_Pfn0;  break;
    case 1:  f = BTAC1C2_PredictSample_Pfn1;  break;
    case 2:  f = BTAC1C2_PredictSample_Pfn2;  break;
    case 3:  f = BTAC1C2_PredictSample_Pfn3;  break;
    case 4:  f = BTAC1C2_PredictSample_Pfn4;  break;
    case 5:  f = BTAC1C2_PredictSample_Pfn5;  break;
    case 6:  f = BTAC1C2_PredictSample_Pfn6;  break;
    case 7:  f = BTAC1C2_PredictSample_Pfn7;  break;
    case 8:  f = BTAC1C2_PredictSample_Pfn8;  break;
    case 9:  f = BTAC1C2_PredictSample_Pfn9;  break;
    case 10: f = BTAC1C2_PredictSample_Pfn10; break;
    case 11: f = BTAC1C2_PredictSample_Pfn11; break;
    default: f = BTAC1C2_PredictSample;       break;
    }
    return f(psamp, idx, pfcn, ridx);
}

/* Mirrors the Rust `__difftest_selector`: exposes which function
 * BTAC1C2_GetPredictFunc picked.  0..=11 -> _PfnN, 12 -> the generic
 * BTAC1C2_PredictSample, -1 -> unrecognised. */
int __difftest_selector(int pfcn) {
    void *fcn = BTAC1C2_GetPredictFunc(pfcn);
    void *table[12];
    int i;
    table[0]  = (void *)BTAC1C2_PredictSample_Pfn0;
    table[1]  = (void *)BTAC1C2_PredictSample_Pfn1;
    table[2]  = (void *)BTAC1C2_PredictSample_Pfn2;
    table[3]  = (void *)BTAC1C2_PredictSample_Pfn3;
    table[4]  = (void *)BTAC1C2_PredictSample_Pfn4;
    table[5]  = (void *)BTAC1C2_PredictSample_Pfn5;
    table[6]  = (void *)BTAC1C2_PredictSample_Pfn6;
    table[7]  = (void *)BTAC1C2_PredictSample_Pfn7;
    table[8]  = (void *)BTAC1C2_PredictSample_Pfn8;
    table[9]  = (void *)BTAC1C2_PredictSample_Pfn9;
    table[10] = (void *)BTAC1C2_PredictSample_Pfn10;
    table[11] = (void *)BTAC1C2_PredictSample_Pfn11;
    for (i = 0; i < 12; i++) {
        if (fcn == table[i]) {
            return i;
        }
    }
    if (fcn == (void *)BTAC1C2_PredictSample) {
        return 12;
    }
    return -1;
}

/* Mirrors the Rust `__difftest_call_selected`: invokes whatever the selector
 * returned, exercising selector + predictor as a composed pipeline. */
int __difftest_call_selected(int *psamp, int idx, int pfcn,
                             btac1c_idxstate *ridx) {
    void *fcn = BTAC1C2_GetPredictFunc(pfcn);
    int (*f)(int *, int, int, btac1c_idxstate *) =
        (int (*)(int *, int, int, btac1c_idxstate *))fcn;
    return f(psamp, idx, pfcn, ridx);
}

/* Report the size/offsets of btac1c_idxstate so the Rust test can assert the
 * struct layout it uses for `firfx` matches the C compiler's exactly. */
int __difftest_layout(int what) {
    switch (what) {
    case 0: return (int)sizeof(btac1c_idxstate);
    case 1: return (int)__builtin_offsetof(btac1c_idxstate, idx);
    case 2: return (int)__builtin_offsetof(btac1c_idxstate, lpred);
    case 3: return (int)__builtin_offsetof(btac1c_idxstate, rpred);
    case 4: return (int)__builtin_offsetof(btac1c_idxstate, tag);
    case 5: return (int)__builtin_offsetof(btac1c_idxstate, bcfcn);
    case 6: return (int)__builtin_offsetof(btac1c_idxstate, bsfcn);
    case 7: return (int)__builtin_offsetof(btac1c_idxstate, usefx);
    case 8: return (int)__builtin_offsetof(btac1c_idxstate, firfx);
    case 9: return (int)sizeof(((btac1c_idxstate *)0)->firfx);
    case 10: return (int)__alignof__(btac1c_idxstate);
    default: return -1;
    }
}
