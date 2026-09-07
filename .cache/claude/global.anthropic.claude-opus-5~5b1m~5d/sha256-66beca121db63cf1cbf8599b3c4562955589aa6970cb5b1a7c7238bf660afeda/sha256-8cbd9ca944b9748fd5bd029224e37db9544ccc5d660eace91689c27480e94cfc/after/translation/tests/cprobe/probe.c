/* TEST-ONLY probe translation unit.
 *
 * c_src/ is NEVER modified. This file lives in translation/tests/cprobe/ and
 * simply #includes the untouched C source so that the functions which are
 * `static` there become callable, then re-exports them under `probe_*` names
 * with external linkage. That lets the Rust `probe_*` wrappers be compared
 * against the real C arithmetic, function by function.
 *
 * C_LIB_SOURCE is supplied on the command line as an absolute path, e.g.
 *   -DC_LIB_SOURCE="/abs/path/c_src/src/lib.c"
 */

#include C_LIB_SOURCE

#include <stddef.h>

#define PROBE(name, inner)                                                     \
    int name(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx) {            \
        return inner(psamp, idx, pfcn, ridx);                                   \
    }

PROBE(probe_PredictSample, BTAC1C2_PredictSample)
PROBE(probe_Pfn0, BTAC1C2_PredictSample_Pfn0)
PROBE(probe_Pfn1, BTAC1C2_PredictSample_Pfn1)
PROBE(probe_Pfn2, BTAC1C2_PredictSample_Pfn2)
PROBE(probe_Pfn3, BTAC1C2_PredictSample_Pfn3)
PROBE(probe_Pfn4, BTAC1C2_PredictSample_Pfn4)
PROBE(probe_Pfn5, BTAC1C2_PredictSample_Pfn5)
PROBE(probe_Pfn6, BTAC1C2_PredictSample_Pfn6)
PROBE(probe_Pfn7, BTAC1C2_PredictSample_Pfn7)
PROBE(probe_Pfn8, BTAC1C2_PredictSample_Pfn8)
PROBE(probe_Pfn9, BTAC1C2_PredictSample_Pfn9)
PROBE(probe_Pfn10, BTAC1C2_PredictSample_Pfn10)
PROBE(probe_Pfn11, BTAC1C2_PredictSample_Pfn11)

typedef int (*probe_predict_fn)(int *, int, int, btac1c_idxstate *);

int probe_GetPredictFunc_index(int pfcn) {
    void *fcn = BTAC1C2_GetPredictFunc(pfcn);
    probe_predict_fn table[12] = {
        BTAC1C2_PredictSample_Pfn0, BTAC1C2_PredictSample_Pfn1,
        BTAC1C2_PredictSample_Pfn2, BTAC1C2_PredictSample_Pfn3,
        BTAC1C2_PredictSample_Pfn4, BTAC1C2_PredictSample_Pfn5,
        BTAC1C2_PredictSample_Pfn6, BTAC1C2_PredictSample_Pfn7,
        BTAC1C2_PredictSample_Pfn8, BTAC1C2_PredictSample_Pfn9,
        BTAC1C2_PredictSample_Pfn10, BTAC1C2_PredictSample_Pfn11};
    int i;
    for (i = 0; i < 12; i++) {
        if (fcn == (void *)table[i]) {
            return i;
        }
    }
    if (fcn == (void *)BTAC1C2_PredictSample) {
        return -1;
    }
    return -2;
}

size_t probe_idxstate_size(void) { return sizeof(btac1c_idxstate); }
size_t probe_idxstate_align(void) { return _Alignof(btac1c_idxstate); }
size_t probe_idxstate_firfx_offset(void) {
    return offsetof(btac1c_idxstate, firfx);
}
