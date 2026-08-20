/* Differential-test shim for the C side.
 *
 * c_src/ is READ-ONLY, so instead of editing it we #include the translation
 * unit verbatim. That gives this file visibility of the 14 `static`
 * (internal-linkage) routines, which we re-export under `diffshim_*` names.
 * The Rust crate exports the identical names under its `diff_internals`
 * feature, so the tests can dlopen both objects and call the same symbols.
 *
 * Build with the SAME optimisation settings as c_src/build (CMake sets no
 * -O flag, i.e. -O0) so that signed-overflow behaviour is identical.
 */

#include "../../c_src/src/lib.c"

/* Call BTAC1C2_PredictSample (the big switch) directly. */
int diffshim_predict_sample(int *psamp, int idx, int pfcn,
                            btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample(psamp, idx, pfcn, ridx);
}

/* Dispatch to BTAC1C2_PredictSample_Pfn<which>.
 * `which` outside 0..11 yields the sentinel 0x5EEDBAD, matching the Rust shim.
 */
int diffshim_pfn(int which, int *psamp, int idx, int pfcn,
                 btac1c_idxstate *ridx) {
    switch (which) {
    case 0:
        return BTAC1C2_PredictSample_Pfn0(psamp, idx, pfcn, ridx);
    case 1:
        return BTAC1C2_PredictSample_Pfn1(psamp, idx, pfcn, ridx);
    case 2:
        return BTAC1C2_PredictSample_Pfn2(psamp, idx, pfcn, ridx);
    case 3:
        return BTAC1C2_PredictSample_Pfn3(psamp, idx, pfcn, ridx);
    case 4:
        return BTAC1C2_PredictSample_Pfn4(psamp, idx, pfcn, ridx);
    case 5:
        return BTAC1C2_PredictSample_Pfn5(psamp, idx, pfcn, ridx);
    case 6:
        return BTAC1C2_PredictSample_Pfn6(psamp, idx, pfcn, ridx);
    case 7:
        return BTAC1C2_PredictSample_Pfn7(psamp, idx, pfcn, ridx);
    case 8:
        return BTAC1C2_PredictSample_Pfn8(psamp, idx, pfcn, ridx);
    case 9:
        return BTAC1C2_PredictSample_Pfn9(psamp, idx, pfcn, ridx);
    case 10:
        return BTAC1C2_PredictSample_Pfn10(psamp, idx, pfcn, ridx);
    case 11:
        return BTAC1C2_PredictSample_Pfn11(psamp, idx, pfcn, ridx);
    default:
        return 0x5EEDBAD;
    }
}

/* Identity of the pointer BTAC1C2_GetPredictFunc returns, as an index:
 * 0..11 = _Pfn0.._Pfn11, 12 = BTAC1C2_PredictSample, -1 = matched nothing. */
int diffshim_getpredictfunc_index(int pfcn) {
    void *fcn = BTAC1C2_GetPredictFunc(pfcn);
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn0) return 0;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn1) return 1;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn2) return 2;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn3) return 3;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn4) return 4;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn5) return 5;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn6) return 6;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn7) return 7;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn8) return 8;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn9) return 9;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn10) return 10;
    if (fcn == (void *)BTAC1C2_PredictSample_Pfn11) return 11;
    if (fcn == (void *)BTAC1C2_PredictSample) return 12;
    return -1;
}

/* Layout probe for struct btac1c_idxstate_s. See the Rust twin for the
 * meaning of `what`. */
int diffshim_idxstate_layout(int what) {
    switch (what) {
    case 0:
        return (int)sizeof(btac1c_idxstate);
    case 1:
        return (int)__alignof__(btac1c_idxstate);
    case 2:
        return (int)__builtin_offsetof(btac1c_idxstate, idx);
    case 3:
        return (int)__builtin_offsetof(btac1c_idxstate, lpred);
    case 4:
        return (int)__builtin_offsetof(btac1c_idxstate, rpred);
    case 5:
        return (int)__builtin_offsetof(btac1c_idxstate, tag);
    case 6:
        return (int)__builtin_offsetof(btac1c_idxstate, bcfcn);
    case 7:
        return (int)__builtin_offsetof(btac1c_idxstate, bsfcn);
    case 8:
        return (int)__builtin_offsetof(btac1c_idxstate, usefx);
    case 9:
        return (int)__builtin_offsetof(btac1c_idxstate, firfx);
    default:
        return -1;
    }
}
