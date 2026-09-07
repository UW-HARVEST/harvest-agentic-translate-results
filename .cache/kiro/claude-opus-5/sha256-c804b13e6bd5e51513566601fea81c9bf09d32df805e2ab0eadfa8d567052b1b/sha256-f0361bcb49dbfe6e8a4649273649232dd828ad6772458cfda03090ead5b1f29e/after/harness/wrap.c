/*
 * Differential-test harness for the STATIC helpers in c_src/src/lib.c.
 *
 * c_src/ is read-only and its .so exports only `call_predict`, so the twelve
 * `BTAC1C2_PredictSample_Pfn*` helpers, the generic `BTAC1C2_PredictSample`
 * dispatcher body, and `BTAC1C2_GetPredictFunc` are unreachable across the ABI
 * boundary. This file lives OUTSIDE c_src/, textually includes the unmodified C
 * translation unit, and re-exports thin `extern` wrappers so the Rust
 * translation's private helpers can be compared against the real C code.
 *
 * Nothing in c_src/ is modified. Compile with the same flags CMake used for the
 * shipped .so (-fPIC, no optimisation) so codegen semantics match:
 *
 *   gcc -fPIC -shared -I c_src/include -o harness/libcwrap.so harness/wrap.c
 */

#include "../c_src/src/lib.c"

typedef int (*wrap_predict_fn)(int *, int, int, btac1c_idxstate *);

int wrap_predict_sample(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample(psamp, idx, pfcn, ridx);
}

int wrap_pfn0(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn0(psamp, idx, pfcn, ridx);
}
int wrap_pfn1(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn1(psamp, idx, pfcn, ridx);
}
int wrap_pfn2(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn2(psamp, idx, pfcn, ridx);
}
int wrap_pfn3(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn3(psamp, idx, pfcn, ridx);
}
int wrap_pfn4(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn4(psamp, idx, pfcn, ridx);
}
int wrap_pfn5(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn5(psamp, idx, pfcn, ridx);
}
int wrap_pfn6(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn6(psamp, idx, pfcn, ridx);
}
int wrap_pfn7(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn7(psamp, idx, pfcn, ridx);
}
int wrap_pfn8(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn8(psamp, idx, pfcn, ridx);
}
int wrap_pfn9(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn9(psamp, idx, pfcn, ridx);
}
int wrap_pfn10(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn10(psamp, idx, pfcn, ridx);
}
int wrap_pfn11(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample_Pfn11(psamp, idx, pfcn, ridx);
}

/*
 * Identity of the pointer BTAC1C2_GetPredictFunc hands back, encoded as an int
 * so it can be compared across the two libraries:
 *   0..11 -> BTAC1C2_PredictSample_Pfn<n>
 *      -1 -> BTAC1C2_PredictSample (the generic dispatcher, i.e. default:)
 *      -2 -> unrecognised (must never happen)
 */
int wrap_get_predict_func_index(int pfcn) {
    void *f = BTAC1C2_GetPredictFunc(pfcn);
    if (f == (void *)BTAC1C2_PredictSample_Pfn0) return 0;
    if (f == (void *)BTAC1C2_PredictSample_Pfn1) return 1;
    if (f == (void *)BTAC1C2_PredictSample_Pfn2) return 2;
    if (f == (void *)BTAC1C2_PredictSample_Pfn3) return 3;
    if (f == (void *)BTAC1C2_PredictSample_Pfn4) return 4;
    if (f == (void *)BTAC1C2_PredictSample_Pfn5) return 5;
    if (f == (void *)BTAC1C2_PredictSample_Pfn6) return 6;
    if (f == (void *)BTAC1C2_PredictSample_Pfn7) return 7;
    if (f == (void *)BTAC1C2_PredictSample_Pfn8) return 8;
    if (f == (void *)BTAC1C2_PredictSample_Pfn9) return 9;
    if (f == (void *)BTAC1C2_PredictSample_Pfn10) return 10;
    if (f == (void *)BTAC1C2_PredictSample_Pfn11) return 11;
    if (f == (void *)BTAC1C2_PredictSample) return -1;
    return -2;
}

/* End-to-end composed path: dispatch, then call through the returned pointer. */
int wrap_call_through(int pfcn, int *psamp, int idx, btac1c_idxstate *ridx) {
    wrap_predict_fn f = (wrap_predict_fn)BTAC1C2_GetPredictFunc(pfcn);
    return f(psamp, idx, pfcn, ridx);
}

/* sizeof/offsetof probes, so the Rust #[repr(C)] struct layout can be checked. */
int wrap_sizeof_idxstate(void) { return (int)sizeof(btac1c_idxstate); }
int wrap_alignof_idxstate(void) { return (int)__alignof__(btac1c_idxstate); }
int wrap_offsetof_firfx(void) { return (int)__builtin_offsetof(btac1c_idxstate, firfx); }
int wrap_offsetof_usefx(void) { return (int)__builtin_offsetof(btac1c_idxstate, usefx); }
int wrap_offsetof_lpred(void) { return (int)__builtin_offsetof(btac1c_idxstate, lpred); }
