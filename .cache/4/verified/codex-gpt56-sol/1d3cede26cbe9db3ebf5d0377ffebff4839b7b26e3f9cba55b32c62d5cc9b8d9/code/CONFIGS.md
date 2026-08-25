# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or preprocessor branches. The complete feature
combination set is therefore:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features` (empty set) | default and only configuration |

## Runtime Configurations

The `overunder` rows use these mechanically derived shape names:

- `r`: the `a % 6` switch path (`D` is the default path for `-1..=-5`).
- `t1`: classification of `(double)a * 1.5` by `safe_double_to_int`.
- `t2`: classification of `(double)b * 2.7` by `safe_double_to_int`.
- `s`: sign of the compiled C `int` result of `d*d + a*a` before `sqrt`.
- `L`, `I`, `H`: below `INT_MIN`, in the inclusive integer range, or above
  `INT_MAX`.
- `N`, `P`: negative or nonnegative.

Each row's randomized corpus also varies `c` over the full `int` domain.
Impossible combinations are pruned: positive remainders cannot have `t1=L`,
and default negative remainders cannot have `t1=H`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `safe_double_to_int` | finite `d` in inclusive `INT_MIN..=INT_MAX`, including signed zero, fractions, and exact boundaries | [x] |
| 2 | `process_with_fallthrough` | `code=0`; randomized `base_value` | [x] |
| 3 | `process_with_fallthrough` | `code=1`; randomized `base_value` | [x] |
| 4 | `process_with_fallthrough` | `code=2`; randomized `base_value` | [x] |
| 5 | `process_with_fallthrough` | `code=3`; randomized `base_value` | [x] |
| 6 | `process_with_fallthrough` | `code=4`; randomized `base_value` | [x] |
| 7 | `process_with_fallthrough` | `code=5`; randomized `base_value` | [x] |
| 8 | `copy_data_block` | distinct, nonnull `DataBlock` pointers; all 40 source bytes randomized, including padding | [x] |
| 9 | `handle_pointer_operations` | randomized `value` over the full C `int` bit pattern domain | [x] |
| 10 | `overunder` | `r=0; t1=L; t2=L; s=N` | [x] |
| 11 | `overunder` | `r=0; t1=L; t2=L; s=P` | [x] |
| 12 | `overunder` | `r=0; t1=L; t2=I; s=N` | [x] |
| 13 | `overunder` | `r=0; t1=L; t2=I; s=P` | [x] |
| 14 | `overunder` | `r=0; t1=L; t2=H; s=N` | [x] |
| 15 | `overunder` | `r=0; t1=L; t2=H; s=P` | [x] |
| 16 | `overunder` | `r=0; t1=I; t2=L; s=N` | [x] |
| 17 | `overunder` | `r=0; t1=I; t2=L; s=P` | [x] |
| 18 | `overunder` | `r=0; t1=I; t2=I; s=N` | [x] |
| 19 | `overunder` | `r=0; t1=I; t2=I; s=P` | [x] |
| 20 | `overunder` | `r=0; t1=I; t2=H; s=N` | [x] |
| 21 | `overunder` | `r=0; t1=I; t2=H; s=P` | [x] |
| 22 | `overunder` | `r=0; t1=H; t2=L; s=N` | [x] |
| 23 | `overunder` | `r=0; t1=H; t2=L; s=P` | [x] |
| 24 | `overunder` | `r=0; t1=H; t2=I; s=N` | [x] |
| 25 | `overunder` | `r=0; t1=H; t2=I; s=P` | [x] |
| 26 | `overunder` | `r=0; t1=H; t2=H; s=N` | [x] |
| 27 | `overunder` | `r=0; t1=H; t2=H; s=P` | [x] |
| 28 | `overunder` | `r=1; t1=I; t2=L; s=N` | [x] |
| 29 | `overunder` | `r=1; t1=I; t2=L; s=P` | [x] |
| 30 | `overunder` | `r=1; t1=I; t2=I; s=N` | [x] |
| 31 | `overunder` | `r=1; t1=I; t2=I; s=P` | [x] |
| 32 | `overunder` | `r=1; t1=I; t2=H; s=N` | [x] |
| 33 | `overunder` | `r=1; t1=I; t2=H; s=P` | [x] |
| 34 | `overunder` | `r=1; t1=H; t2=L; s=N` | [x] |
| 35 | `overunder` | `r=1; t1=H; t2=L; s=P` | [x] |
| 36 | `overunder` | `r=1; t1=H; t2=I; s=N` | [x] |
| 37 | `overunder` | `r=1; t1=H; t2=I; s=P` | [x] |
| 38 | `overunder` | `r=1; t1=H; t2=H; s=N` | [x] |
| 39 | `overunder` | `r=1; t1=H; t2=H; s=P` | [x] |
| 40 | `overunder` | `r=2; t1=I; t2=L; s=N` | [x] |
| 41 | `overunder` | `r=2; t1=I; t2=L; s=P` | [x] |
| 42 | `overunder` | `r=2; t1=I; t2=I; s=N` | [x] |
| 43 | `overunder` | `r=2; t1=I; t2=I; s=P` | [x] |
| 44 | `overunder` | `r=2; t1=I; t2=H; s=N` | [x] |
| 45 | `overunder` | `r=2; t1=I; t2=H; s=P` | [x] |
| 46 | `overunder` | `r=2; t1=H; t2=L; s=N` | [x] |
| 47 | `overunder` | `r=2; t1=H; t2=L; s=P` | [x] |
| 48 | `overunder` | `r=2; t1=H; t2=I; s=N` | [x] |
| 49 | `overunder` | `r=2; t1=H; t2=I; s=P` | [x] |
| 50 | `overunder` | `r=2; t1=H; t2=H; s=N` | [x] |
| 51 | `overunder` | `r=2; t1=H; t2=H; s=P` | [x] |
| 52 | `overunder` | `r=3; t1=I; t2=L; s=N` | [x] |
| 53 | `overunder` | `r=3; t1=I; t2=L; s=P` | [x] |
| 54 | `overunder` | `r=3; t1=I; t2=I; s=N` | [x] |
| 55 | `overunder` | `r=3; t1=I; t2=I; s=P` | [x] |
| 56 | `overunder` | `r=3; t1=I; t2=H; s=N` | [x] |
| 57 | `overunder` | `r=3; t1=I; t2=H; s=P` | [x] |
| 58 | `overunder` | `r=3; t1=H; t2=L; s=N` | [x] |
| 59 | `overunder` | `r=3; t1=H; t2=L; s=P` | [x] |
| 60 | `overunder` | `r=3; t1=H; t2=I; s=N` | [x] |
| 61 | `overunder` | `r=3; t1=H; t2=I; s=P` | [x] |
| 62 | `overunder` | `r=3; t1=H; t2=H; s=N` | [x] |
| 63 | `overunder` | `r=3; t1=H; t2=H; s=P` | [x] |
| 64 | `overunder` | `r=4; t1=I; t2=L; s=N` | [x] |
| 65 | `overunder` | `r=4; t1=I; t2=L; s=P` | [x] |
| 66 | `overunder` | `r=4; t1=I; t2=I; s=N` | [x] |
| 67 | `overunder` | `r=4; t1=I; t2=I; s=P` | [x] |
| 68 | `overunder` | `r=4; t1=I; t2=H; s=N` | [x] |
| 69 | `overunder` | `r=4; t1=I; t2=H; s=P` | [x] |
| 70 | `overunder` | `r=4; t1=H; t2=L; s=N` | [x] |
| 71 | `overunder` | `r=4; t1=H; t2=L; s=P` | [x] |
| 72 | `overunder` | `r=4; t1=H; t2=I; s=N` | [x] |
| 73 | `overunder` | `r=4; t1=H; t2=I; s=P` | [x] |
| 74 | `overunder` | `r=4; t1=H; t2=H; s=N` | [x] |
| 75 | `overunder` | `r=4; t1=H; t2=H; s=P` | [x] |
| 76 | `overunder` | `r=5; t1=I; t2=L; s=N` | [x] |
| 77 | `overunder` | `r=5; t1=I; t2=L; s=P` | [x] |
| 78 | `overunder` | `r=5; t1=I; t2=I; s=N` | [x] |
| 79 | `overunder` | `r=5; t1=I; t2=I; s=P` | [x] |
| 80 | `overunder` | `r=5; t1=I; t2=H; s=N` | [x] |
| 81 | `overunder` | `r=5; t1=I; t2=H; s=P` | [x] |
| 82 | `overunder` | `r=5; t1=H; t2=L; s=N` | [x] |
| 83 | `overunder` | `r=5; t1=H; t2=L; s=P` | [x] |
| 84 | `overunder` | `r=5; t1=H; t2=I; s=N` | [x] |
| 85 | `overunder` | `r=5; t1=H; t2=I; s=P` | [x] |
| 86 | `overunder` | `r=5; t1=H; t2=H; s=N` | [x] |
| 87 | `overunder` | `r=5; t1=H; t2=H; s=P` | [x] |
| 88 | `overunder` | `r=D; t1=L; t2=L; s=N` | [x] |
| 89 | `overunder` | `r=D; t1=L; t2=L; s=P` | [x] |
| 90 | `overunder` | `r=D; t1=L; t2=I; s=N` | [x] |
| 91 | `overunder` | `r=D; t1=L; t2=I; s=P` | [x] |
| 92 | `overunder` | `r=D; t1=L; t2=H; s=N` | [x] |
| 93 | `overunder` | `r=D; t1=L; t2=H; s=P` | [x] |
| 94 | `overunder` | `r=D; t1=I; t2=L; s=N` | [x] |
| 95 | `overunder` | `r=D; t1=I; t2=L; s=P` | [x] |
| 96 | `overunder` | `r=D; t1=I; t2=I; s=N` | [x] |
| 97 | `overunder` | `r=D; t1=I; t2=I; s=P` | [x] |
| 98 | `overunder` | `r=D; t1=I; t2=H; s=N` | [x] |
| 99 | `overunder` | `r=D; t1=I; t2=H; s=P` | [x] |
