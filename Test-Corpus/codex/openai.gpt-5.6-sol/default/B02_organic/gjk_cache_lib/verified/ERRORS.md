# Error surface

The mechanical scan covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, every `if`/`switch`, null check, enum case, loop bound, and numeric
limit in `src/lib.c`. This C source contains no error return, error enum,
assertion, or explicit rejection branch. The rows below are the defined generic
boundary cases that accept unusual input instead of rejecting it.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|---|---|---|
| 1 | `c2MakeProxy` | `type` is `-1`, `3`, or another value outside `C2_TYPE_CIRCLE..C2_TYPE_CAPSULE` while `p` is valid | [x] Returns `void`; no switch arm runs and every byte of `*p` remains unchanged |
| 2 | `c2Support` | `count == 0` with `verts` pointing to at least one element | [x] Reads `verts[0]`, executes zero loop iterations, returns `0` |
| 3 | `c2Support` | `count < 0` with `verts` pointing to at least one element | [x] Reads `verts[0]`, executes zero loop iterations, returns `0` |
| 4 | `c2Support` | oversized positive `count` with that many readable elements | [x] Scans all `count` elements and returns the first index having the greatest dot product |
| 5 | `c2GJK` | `ax_ptr == NULL` | [x] Uses `c2xIdentity()` for A |
| 6 | `c2GJK` | `bx_ptr == NULL` | [x] Uses `c2xIdentity()` for B |
| 7 | `c2GJK` | `outA == NULL` | [x] Returns the same distance and omits the A witness write |
| 8 | `c2GJK` | `outB == NULL` | [x] Returns the same distance and omits the B witness write |
| 9 | `c2GJK` | `iterations == NULL` | [x] Returns the same distance and omits the iteration-count write |
| 10 | `c2GJK` | `cache == NULL` | [x] Starts with the default simplex and omits cache read/write |
| 11 | `gjk_cache` | `a9 == NULL` | [x] Safe because `a9` is never read or written; returns `void` |
| 12 | `gjk_cache` | `b9 == NULL` | [x] Safe because `b9` is never read or written; returns `void` |

## Undefined C calls

The remaining generic invalid calls have no C result to compare. Passing null
to a dereferenced pointer parameter (`c2BBVerts`, `c2MakeProxy`,
`c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2Support`, `c2Witness`, `c2L`,
or the required A/B shape pointers of `c2GJK`) invokes undefined behavior.
Likewise, an out-of-range shape enum passed to `c2GJK` leaves a local `c2Proxy`
uninitialized and then reads it. These are not rejection paths and cannot have
a byte-identical expected C result. The independently exported `c2MakeProxy`
out-of-range-enum behavior is defined and is row 1 above.

`null_pointer_termination_matches` nevertheless covers every dereferenced-null
case in an isolated subprocess and verifies that the built C and Rust shared
libraries terminate with the same signal. Isolation prevents undefined calls
from aborting the differential test process.
