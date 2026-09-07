# Error and Boundary Surface

Mechanical searches covered `RETURN_ERROR`, negative/NULL returns, `assert`,
error identifiers, all `if`/`switch` branches, pointer-null checks, enum cases,
array/count limits, and floating-point thresholds in `c_src/src/lib.c`.

The C source contains no explicit error return, error enum, assertion, or
rejection code. The rows below capture every defined boundary/default behavior
that substitutes for rejection, plus the generic FFI boundaries required by
the verification protocol. Required-pointer null dereferences are C undefined
behavior; they have no portable C result and are identified separately rather
than assigned an invented sentinel.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `c2MakeProxy` | `type` is any integer other than 0, 1, or 2 | no `switch` case executes; `*p` remains byte-unchanged | [x] |
| E2 | `c2Support` | `count == 0` with `verts` still pointing to one readable element | reads `verts[0]`, loop is skipped, returns 0 | [x] |
| E3 | `c2Support` | `count < 0` with `verts` still pointing to one readable element | reads `verts[0]`, loop is skipped, returns 0 | [x] |
| E4 | `c2GJKSimplexMetric` | `s->count` is outside 1, 2, 3 | default branch returns `0.0f` | [x] |
| E5 | `c2D` | `s->count` is outside 1, 2 | default branch returns `{0.0f, 0.0f}` | [x] |
| E6 | `c2Witness` | `s->count` is outside 1, 2, 3 | writes `{0.0f, 0.0f}` to both outputs | [x] |
| E7 | `c2L` | `s->count` is outside 1, 2 | default branch returns `{0.0f, 0.0f}` | [x] |
| E8 | `c2Div` | divisor is `+0.0f` or `-0.0f` | IEEE-754 infinities/NaNs from `1.0f / b`, with no rejection | [x] |
| E9 | `c2Norm` | input vector has zero length | divides zero components by zero length, returning NaN components | [x] |
| E10 | `c2GJK` | `ax_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| E11 | `c2GJK` | `bx_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| E12 | `c2GJK` | any of `outA`, `outB`, or `iterations` is `NULL` | corresponding output write is skipped; return distance is unchanged | [x] |
| E13 | `c2GJK` | `cache == NULL` | cache read and write are skipped | [x] |
| E14 | `c2GJK` | `cache != NULL` and `cache->count == 0` | cache is treated as cold and overwritten with the resulting simplex | [x] |
| E15 | `c2GJK` | `use_radius` is any nonzero integer, including values outside 0/1 | radius adjustment branch is enabled | [x] |
| E16 | `gjk_cache` | `a9 == NULL` and/or `b9 == NULL` | accepted because both parameters are unused; function returns normally | [x] |
| E17 | pointer-taking exports | a required pointer (`shape`, `p`, `out`, `bb`, `verts`, `s`, witness output, or GJK shape) is `NULL` | no C null check; representative dereferences terminate with `SIGSEGV` on the verification platform | [x] |
| E18 | `c2Support` | `count == 9` with nine readable vertices (one above the internal proxy capacity of 8) | scans all nine readable vertices and returns the first maximum index | [x] |

The following inputs do not have a defined C result and therefore cannot have
a byte-identical portable oracle: invalid `c2GJK` shape enums (leave a local
proxy uninitialized), counts larger than the actually supplied readable vertex
array, cache counts outside 0..3, and cached indices outside a proxy's vertex
count. They are excluded from pass/fail rows because assigning an expected
sentinel would second-guess the C implementation.
