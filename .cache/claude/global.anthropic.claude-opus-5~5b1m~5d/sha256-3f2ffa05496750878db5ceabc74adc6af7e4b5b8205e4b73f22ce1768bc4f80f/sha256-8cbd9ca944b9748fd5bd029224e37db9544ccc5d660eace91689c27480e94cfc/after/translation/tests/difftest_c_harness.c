/*
 * Test-only harness.  Lives OUTSIDE c_src/ and does not modify it: it simply
 * #includes the original translation unit verbatim so that the `static`
 * (internal-linkage) predictors can be reached from a differential test.
 *
 * Built into its own shared object by tests/difftest.rs.  The exported
 * `__difftest_predict` mirrors, symbol-for-symbol and semantics-for-semantics,
 * the `#[cfg(feature = "difftest")]` hook in translation/src/lib.rs:
 *
 *   which in 0..=11  ->  BTAC1C2_PredictSample_Pfn<which>
 *   otherwise        ->  BTAC1C2_PredictSample   (the generic dispatcher)
 *
 * which is exactly what BTAC1C2_GetPredictFunc(which) already returns, so the
 * harness reuses the real C dispatcher rather than re-implementing it.
 */

#include <stddef.h>

#include "lib.c"

typedef int (*btac1c_predict_fn)(int *, int, int, btac1c_idxstate *);

int __difftest_predict(int which, int *psamp, int idx, int pfcn,
                       btac1c_idxstate *ridx) {
    btac1c_predict_fn f = (btac1c_predict_fn)BTAC1C2_GetPredictFunc(which);
    return f(psamp, idx, pfcn, ridx);
}

/* Layout probe: mirrors __difftest_layout in translation/src/lib.rs. */
int __difftest_layout(int what) {
    switch (what) {
    case 0:
        return (int)sizeof(btac1c_idxstate);
    case 1:
        return (int)_Alignof(btac1c_idxstate);
    case 2:
        return (int)offsetof(btac1c_idxstate, idx);
    case 3:
        return (int)offsetof(btac1c_idxstate, lpred);
    case 4:
        return (int)offsetof(btac1c_idxstate, rpred);
    case 5:
        return (int)offsetof(btac1c_idxstate, tag);
    case 6:
        return (int)offsetof(btac1c_idxstate, bcfcn);
    case 7:
        return (int)offsetof(btac1c_idxstate, bsfcn);
    case 8:
        return (int)offsetof(btac1c_idxstate, usefx);
    case 9:
        return (int)offsetof(btac1c_idxstate, firfx);
    case 10:
        return (int)sizeof(((btac1c_idxstate *)0)->firfx);
    default:
        return -1;
    }
}

/*
 * Identity probe for BTAC1C2_GetPredictFunc: returns a small integer tag
 * identifying WHICH function pointer the dispatcher handed back, so the test
 * can compare dispatcher behaviour (not just get_predict_func's boolean).
 *   0..11 -> the matching PfnN,  12 -> the generic BTAC1C2_PredictSample,
 *   -1    -> something else entirely (must never happen).
 */
int __difftest_dispatch_tag(int pfcn) {
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
    if (f == (void *)BTAC1C2_PredictSample) return 12;
    return -1;
}
