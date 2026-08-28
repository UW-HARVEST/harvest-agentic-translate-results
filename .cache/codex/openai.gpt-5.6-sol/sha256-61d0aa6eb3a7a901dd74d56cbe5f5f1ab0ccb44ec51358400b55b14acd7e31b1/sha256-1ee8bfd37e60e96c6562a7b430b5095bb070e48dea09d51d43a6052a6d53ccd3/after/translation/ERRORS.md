# Error Surface

The following mechanical rejection scan was applied to `c_src/src/lib.c`:

```sh
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|ERROR|EINVAL|ERANGE' c_src/src/lib.c
```

It finds **zero explicit rejection/error branches**. The C code has no error
enum, error-return macro, assertion, documented min/max input range, or
error sentinel. Consequently, the required rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Defined Boundary Behavior

These are the generic boundary cases that the C source handles without
dereferencing an invalid mandatory pointer. Null mandatory pointers and an
invalid shape type passed to `c2GJK` have undefined behavior in C, so there
is no C result to compare for those cases.

| # | function | boundary condition | expected C result | status |
|---:|----------|--------------------|-------------------|-----|
| E01 | `c2MakeProxy` | enum value `-1` or `3` | output proxy remains byte-unchanged | [x] |
| E02 | `c2Support` | count `0`, valid backing pointer | returns `0` after reading element zero | [x] |
| E03 | `c2Support` | count `-1`, valid backing pointer | returns `0` after reading element zero | [x] |
| E04 | `c2Support` | oversized count `9`, nine-element backing array | scans all nine elements and returns strict first maximum | [x] |
| E05 | `c2GJKSimplexMetric` | count outside `1..=3` | returns `0.0` | [x] |
| E06 | `c2D` | count outside `1..=3` | returns `(0.0, 0.0)` | [x] |
| E07 | `c2Witness` | count outside `1..=3` | writes `(0.0, 0.0)` to both outputs | [x] |
| E08 | `c2L` | count outside `1..=2` | returns `(0.0, 0.0)` | [x] |
| E09 | `c2GJK` | null `ax_ptr` | uses identity transform | [x] |
| E10 | `c2GJK` | null `bx_ptr` | uses identity transform | [x] |
| E11 | `c2GJK` | null `outA` | skips first witness write | [x] |
| E12 | `c2GJK` | null `outB` | skips second witness write | [x] |
| E13 | `c2GJK` | null `iterations` | skips iteration-count write | [x] |
| E14 | `c2GJK` | null `cache` | skips cache read and write | [x] |
| E15 | `c2GJK` | non-null cache with count `0` | ignores initial fields, then writes resulting cache | [x] |
| E16 | `gjk` | null output `a` | operation completes and only `b` is written | [x] |
| E17 | `gjk` | null output `b` | operation completes and only `a` is written | [x] |
| E18 | `c2Div` | divisor `+0.0` or `-0.0` | returns C/IEEE-754 infinities or NaNs component-wise | [x] |
| E19 | `c2Norm` | zero vector | returns two NaNs | [x] |
