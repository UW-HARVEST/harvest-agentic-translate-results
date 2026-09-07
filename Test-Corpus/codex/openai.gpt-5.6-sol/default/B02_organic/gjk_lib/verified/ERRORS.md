# Error surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, null checks, range comparisons, enum switches, count/length uses,
and numeric limits in `src/lib.c` and `include/lib.h`.

The C implementation has no error-return statements, error enums, assertions,
or explicit input-rejection branches. Consequently, the exact rejection table
has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

The following mandatory FFI boundary cases are not C rejections, but are
tracked for Phase C because they define observable boundary behavior:

| # | function | boundary input | expected C result | tested |
|---|----------|----------------|-------------------|--------|
| B1 | `c2MakeProxy` | enum value outside `0..=2` | switch executes no case; destination bytes remain unchanged | [x] |
| B2 | `c2Support` | `count == 0` with a readable first vertex | reads vertex zero and returns index `0` | [x] |
| B3 | `c2Support` | count larger than proxy maximum (`16`) with 16 readable vertices | scans the full supplied count; no hard maximum | [x] |
| B4 | `c2GJK` | null transform pointers | substitutes identity transforms | [x] |
| B5 | `c2GJK` | null `outA`, `outB`, `iterations`, and `cache` | skips those writes and returns distance normally | [x] |
| B6 | `gjk` | null output pointers | accepted because `c2GJK` treats outputs as optional | [x] |
| B7 | `c2GJKSimplexMetric`, `c2D`, `c2Witness`, `c2L` | simplex count outside handled cases | default branch returns/writes zero vectors or metric zero | [x] |
| B8 | `c2Div`, `c2Norm` | zero divisor / zero vector | IEEE-754 infinities or NaNs; no rejection | [x] |
| B9 | pointer-taking low-level API | required data pointer is null | undefined behavior; the C API defines no error sentinel to compare | [x] |

