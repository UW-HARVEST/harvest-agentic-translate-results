/*
 * Auxiliary test harness -- NOT part of c_src.
 *
 * c_src/src/lib.c keeps every predictor `static`, so the only symbol the real
 * shared library exports is `get_predict_func`. To differentially test the
 * predictor arithmetic itself we textually include the untouched C source and
 * publish thin `extern` wrappers around the static functions.
 *
 * c_src is read-only and unmodified; this file only #includes it.
 */

#include "lib.c"

/* Re-export the struct layout so the caller can build one. */
int harness_idxstate_size(void) { return (int)sizeof(btac1c_idxstate); }
int harness_idxstate_align(void) { return (int)_Alignof(btac1c_idxstate); }
int harness_idxstate_firfx_offset(void) {
    return (int)((char *)&((btac1c_idxstate *)0)->firfx - (char *)0);
}

/* The generic switch-based predictor. */
int harness_predict_generic(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    return BTAC1C2_PredictSample(psamp, idx, pfcn, ridx);
}

/* Each specialization. */
#define HARNESS_PFN(n)                                                         \
    int harness_predict_pfn##n(int *psamp, int idx, int pfcn,                  \
                               btac1c_idxstate *ridx) {                        \
        return BTAC1C2_PredictSample_Pfn##n(psamp, idx, pfcn, ridx);           \
    }

HARNESS_PFN(0)
HARNESS_PFN(1)
HARNESS_PFN(2)
HARNESS_PFN(3)
HARNESS_PFN(4)
HARNESS_PFN(5)
HARNESS_PFN(6)
HARNESS_PFN(7)
HARNESS_PFN(8)
HARNESS_PFN(9)
HARNESS_PFN(10)
HARNESS_PFN(11)

/*
 * Call whatever BTAC1C2_GetPredictFunc hands back for `pfcn`, so the selector's
 * dispatch (not just its address comparison) is observable.
 */
int harness_call_selected(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {
    typedef int (*pfn_t)(int *, int, int, btac1c_idxstate *);
    pfn_t f = (pfn_t)BTAC1C2_GetPredictFunc(pfcn);
    return f(psamp, idx, pfcn, ridx);
}

/*
 * Identify which specialization the selector returned: 0..11 for the dedicated
 * predictors, 100 for the generic fallback, -1 if it matched nothing.
 */
int harness_selector_id(int pfcn) {
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
    if (f == (void *)BTAC1C2_PredictSample) return 100;
    return -1;
}
