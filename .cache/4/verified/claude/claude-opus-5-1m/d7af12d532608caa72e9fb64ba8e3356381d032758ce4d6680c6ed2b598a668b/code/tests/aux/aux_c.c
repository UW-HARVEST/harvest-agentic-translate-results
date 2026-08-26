/*
 * Auxiliary C shim used ONLY by the differential test-suite.
 *
 * c_src/ is never modified.  This file lives outside c_src/ and #include's the
 * original translation unit verbatim so that the `static` (file-local)
 * functions of lib.c become reachable from an exported shim.  That lets the
 * test compare the *internal* predictor math and the function-pointer dispatch
 * table of the C code against the Rust translation, which is impossible through
 * the single exported `call_predict` alone.
 *
 * Compile with:
 *   cc -O2 -fPIC -shared -I<c_src/src> -I<c_src/include> aux_c.c -o libaux_c.so
 */

#include <stddef.h>

#include "lib.c"

typedef int (*aux_pred_fn)(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx);

#define AUX_BAD_WHICH 0x7EC0FFEE

/* --- generic 17-arm predictor (lib.c:18) ------------------------------- */
int aux_predict_sample(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample(psamp, idx, pfcn, ridx);
}

/* --- the 12 specialised predictors (lib.c:105..181) -------------------- */
int aux_pfn(int which, int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
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
        return AUX_BAD_WHICH;
    }
}

/* --- dispatcher (lib.c:183) + call through the returned pointer -------- */
int aux_getpredict_call(int sel, int *psamp, int idx, int pfcn,
                        btac1c_idxstate *ridx) {
    aux_pred_fn f = (aux_pred_fn)BTAC1C2_GetPredictFunc(sel);
    return f(psamp, idx, pfcn, ridx);
}

/* Non-NULL-ness of the dispatcher result (it never returns NULL). */
int aux_getpredict_is_null(int sel) {
    return BTAC1C2_GetPredictFunc(sel) == (void *)0;
}

/*
 * Pairwise identity of the 12 dispatch-table entries, expressed as a bitmap so
 * it can be compared across languages without comparing raw addresses:
 * bit k of the result is set iff GetPredictFunc(sel) == &Pfn<k>.
 */
int aux_getpredict_identity(int sel) {
    void *f = BTAC1C2_GetPredictFunc(sel);
    int bits = 0;
    if (f == (void *)BTAC1C2_PredictSample_Pfn0) bits |= 1 << 0;
    if (f == (void *)BTAC1C2_PredictSample_Pfn1) bits |= 1 << 1;
    if (f == (void *)BTAC1C2_PredictSample_Pfn2) bits |= 1 << 2;
    if (f == (void *)BTAC1C2_PredictSample_Pfn3) bits |= 1 << 3;
    if (f == (void *)BTAC1C2_PredictSample_Pfn4) bits |= 1 << 4;
    if (f == (void *)BTAC1C2_PredictSample_Pfn5) bits |= 1 << 5;
    if (f == (void *)BTAC1C2_PredictSample_Pfn6) bits |= 1 << 6;
    if (f == (void *)BTAC1C2_PredictSample_Pfn7) bits |= 1 << 7;
    if (f == (void *)BTAC1C2_PredictSample_Pfn8) bits |= 1 << 8;
    if (f == (void *)BTAC1C2_PredictSample_Pfn9) bits |= 1 << 9;
    if (f == (void *)BTAC1C2_PredictSample_Pfn10) bits |= 1 << 10;
    if (f == (void *)BTAC1C2_PredictSample_Pfn11) bits |= 1 << 11;
    if (f == (void *)BTAC1C2_PredictSample) bits |= 1 << 12;
    return bits;
}

/* --- struct btac1c_idxstate layout (lib.c:7..16) ----------------------- */
void aux_layout(size_t *out) {
    out[0] = sizeof(btac1c_idxstate);
    out[1] = __alignof__(btac1c_idxstate);
    out[2] = offsetof(btac1c_idxstate, idx);
    out[3] = offsetof(btac1c_idxstate, lpred);
    out[4] = offsetof(btac1c_idxstate, rpred);
    out[5] = offsetof(btac1c_idxstate, tag);
    out[6] = offsetof(btac1c_idxstate, bcfcn);
    out[7] = offsetof(btac1c_idxstate, bsfcn);
    out[8] = offsetof(btac1c_idxstate, usefx);
    out[9] = offsetof(btac1c_idxstate, firfx);
    out[10] = sizeof(btac1c_u16);
    out[11] = sizeof(btac1c_s16);
    out[12] = sizeof(btac1c_byte);
    out[13] = sizeof(int);
}
