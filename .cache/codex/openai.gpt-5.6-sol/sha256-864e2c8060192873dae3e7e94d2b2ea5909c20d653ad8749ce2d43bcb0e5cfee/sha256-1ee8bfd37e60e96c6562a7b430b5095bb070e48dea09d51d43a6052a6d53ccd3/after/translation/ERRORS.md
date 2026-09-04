# Error and rejection surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return -1|return NULL|assert|switch|default:|NULL|!.*ptr|count|1\.192092|3\.402823|1\.0e8|iter <' ../c_src
```

The C source contains no `assert`, `RETURN_ERROR`, `return -1`, or `return
NULL`. It does not validate required pointers. The rows below are every defined
default/rejection branch plus the count boundaries that the API evaluates
without rejecting. Required-null-pointer and invalid-`c2GJK`-tag cases have
undefined behavior in C; Phase C probes them in isolated child processes so a
fault cannot terminate the test runner. Repeated fresh-process probes showed
that invalid tags passed directly to `c2GJK` produce different C bytes from the
same input because the uninitialized proxy is consumed. Those two probes
therefore compare process status only; all defined enum-rejection branches
compare exact return/output bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---:|----------|---------------------------------------------|-------------------|:-:|
| 1 | `c2MakeProxy` | `type` is not `0`, `1`, or `2` | Falls through the switch; returns `void` without changing `*p` and without reading `shape` | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` is neither `2` nor `3` (including invalid counts; `1` is also routed here) | Returns float `0.0` | [x] |
| 3 | `c2D` | `s->count` is neither `1` nor `2` (including invalid counts; `3` is also routed here) | Returns vector `{0.0, 0.0}` | [x] |
| 4 | `c2Witness` | `s->count` is not `1`, `2`, or `3` | Writes `{0.0, 0.0}` to both output vectors | [x] |
| 5 | `c2L` | `s->count` is neither `1` nor `2` (including invalid counts; `3` is also routed here) | Returns vector `{0.0, 0.0}` | [x] |
| 6 | `c2Collided` | `typeA == CIRCLE` and `typeB` is not `0`, `1`, or `2` | Returns integer `0` without dereferencing `A` or `B` | [x] |
| 7 | `c2Collided` | `typeA == AABB` and `typeB` is not `0`, `1`, or `2` | Returns integer `0` without dereferencing `A` or `B` | [x] |
| 8 | `c2Collided` | `typeA == CAPSULE` and `typeB` is not `0`, `1`, or `2` | Returns integer `0` without dereferencing `A` or `B` | [x] |
| 9 | `c2Collided` | `typeA` is not `0`, `1`, or `2` (any `typeB`) | Returns integer `0` without dereferencing `A` or `B` | [x] |
| 10 | `c2Support` | `count == 0` with `verts` pointing to at least one readable vector | Reads `verts[0]`, executes no loop iterations, and returns index `0` | [x] |
| 11 | `c2Support` | `count < 0` with `verts` pointing to at least one readable vector | Reads `verts[0]`, executes no loop iterations, and returns index `0` | [x] |
| 12 | `c2Support` | `count == 9` (one past the proxy's eight-slot capacity) with nine readable vectors supplied by the caller | Scans all nine vectors and returns the first strict maximum index | [x] |

Additional C constants/range gates are algorithmic valid-path branches rather
than rejections and are enumerated in `CONFIGS.md`: the 20-iteration GJK cap,
`FLT_MAX` initial distance, epsilon-squared direction cutoff, epsilon radius
cutoff, cache metric factor `2.0`, and cache metric threshold `-1.0e8`.
