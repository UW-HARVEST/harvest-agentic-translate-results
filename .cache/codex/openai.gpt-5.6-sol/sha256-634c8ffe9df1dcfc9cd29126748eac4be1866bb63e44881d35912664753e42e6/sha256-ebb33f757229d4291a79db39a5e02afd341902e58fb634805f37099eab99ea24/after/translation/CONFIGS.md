# Configuration surface

There are no Cargo features, C preprocessor feature flags, runtime option
objects, modes, byte-order choices, element-type choices, or length/count
parameters. The crate has one effective feature configuration: the empty
default feature set (also exercised explicitly with `--no-default-features`).

The rows below are derived from every exported entry point and every branch
that valid scalar inputs can reach. For compactness, `overunder` rows use:

- `A=N`: `-1431655765 <= a <= 1431655764`, so `a * 1.5` stays in `int` range.
- `A=L`: `a <= -1431655766`, so `a * 1.5 < INT_MIN`.
- `A=H`: `a >= 1431655765`, so `a * 1.5 > INT_MAX`.
- `R=0..5`: `a % 6` selects that exact `switch` case.
- `R=D`: negative nonzero `a % 6` selects the `default` case.
- `B=N`: `-795364314 <= b <= 795364314`, so `b * 2.7` stays in `int` range.
- `B=L`: `b <= -795364315`, so `b * 2.7 < INT_MIN`.
- `B=H`: `b >= 795364315`, so `b * 2.7 > INT_MAX`.
- `Q=+`: wrapping `d*d + a*a` is nonnegative before conversion to `double`.
- `Q=-`: wrapping `d*d + a*a` is negative, so `sqrt` produces NaN.

`c` is randomized over the full `int32_t` domain in every `overunder` row.
Impossible combinations are pruned: `A=L` permits only `R=0` or `R=D`, while
`A=H` permits only `R=0..5`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `safe_double_to_int` | finite `double` in `INT_MIN..=INT_MAX`, including negative, zero, positive, integral, fractional, and exact-boundary values | [x] |
| 2 | `process_with_fallthrough` | `code=0`, randomized `base_value` | [x] |
| 3 | `process_with_fallthrough` | `code=1`, randomized `base_value` | [x] |
| 4 | `process_with_fallthrough` | `code=2`, randomized `base_value` | [x] |
| 5 | `process_with_fallthrough` | `code=3`, randomized `base_value` | [x] |
| 6 | `process_with_fallthrough` | `code=4`, randomized `base_value` | [x] |
| 7 | `process_with_fallthrough` | `code=5`, randomized `base_value` | [x] |
| 8 | `process_with_fallthrough` | default class (`code<0` or `code>5`), randomized `base_value` | [x] |
| 9 | `copy_data_block` | aligned, non-null source/destination; randomized bytes across the complete C layout, including padding and all label/double bit patterns | [x] |
| 10 | `handle_pointer_operations` | randomized `value` across the full `int32_t` domain | [x] |
| 11 | `overunder` | `A=N,R=0; B=L; Q=+` | [x] |
| 12 | `overunder` | `A=N,R=0; B=L; Q=-` | [x] |
| 13 | `overunder` | `A=N,R=0; B=N; Q=+` | [x] |
| 14 | `overunder` | `A=N,R=0; B=N; Q=-` | [x] |
| 15 | `overunder` | `A=N,R=0; B=H; Q=+` | [x] |
| 16 | `overunder` | `A=N,R=0; B=H; Q=-` | [x] |
| 17 | `overunder` | `A=N,R=1; B=L; Q=+` | [x] |
| 18 | `overunder` | `A=N,R=1; B=L; Q=-` | [x] |
| 19 | `overunder` | `A=N,R=1; B=N; Q=+` | [x] |
| 20 | `overunder` | `A=N,R=1; B=N; Q=-` | [x] |
| 21 | `overunder` | `A=N,R=1; B=H; Q=+` | [x] |
| 22 | `overunder` | `A=N,R=1; B=H; Q=-` | [x] |
| 23 | `overunder` | `A=N,R=2; B=L; Q=+` | [x] |
| 24 | `overunder` | `A=N,R=2; B=L; Q=-` | [x] |
| 25 | `overunder` | `A=N,R=2; B=N; Q=+` | [x] |
| 26 | `overunder` | `A=N,R=2; B=N; Q=-` | [x] |
| 27 | `overunder` | `A=N,R=2; B=H; Q=+` | [x] |
| 28 | `overunder` | `A=N,R=2; B=H; Q=-` | [x] |
| 29 | `overunder` | `A=N,R=3; B=L; Q=+` | [x] |
| 30 | `overunder` | `A=N,R=3; B=L; Q=-` | [x] |
| 31 | `overunder` | `A=N,R=3; B=N; Q=+` | [x] |
| 32 | `overunder` | `A=N,R=3; B=N; Q=-` | [x] |
| 33 | `overunder` | `A=N,R=3; B=H; Q=+` | [x] |
| 34 | `overunder` | `A=N,R=3; B=H; Q=-` | [x] |
| 35 | `overunder` | `A=N,R=4; B=L; Q=+` | [x] |
| 36 | `overunder` | `A=N,R=4; B=L; Q=-` | [x] |
| 37 | `overunder` | `A=N,R=4; B=N; Q=+` | [x] |
| 38 | `overunder` | `A=N,R=4; B=N; Q=-` | [x] |
| 39 | `overunder` | `A=N,R=4; B=H; Q=+` | [x] |
| 40 | `overunder` | `A=N,R=4; B=H; Q=-` | [x] |
| 41 | `overunder` | `A=N,R=5; B=L; Q=+` | [x] |
| 42 | `overunder` | `A=N,R=5; B=L; Q=-` | [x] |
| 43 | `overunder` | `A=N,R=5; B=N; Q=+` | [x] |
| 44 | `overunder` | `A=N,R=5; B=N; Q=-` | [x] |
| 45 | `overunder` | `A=N,R=5; B=H; Q=+` | [x] |
| 46 | `overunder` | `A=N,R=5; B=H; Q=-` | [x] |
| 47 | `overunder` | `A=N,R=D; B=L; Q=+` | [x] |
| 48 | `overunder` | `A=N,R=D; B=L; Q=-` | [x] |
| 49 | `overunder` | `A=N,R=D; B=N; Q=+` | [x] |
| 50 | `overunder` | `A=N,R=D; B=N; Q=-` | [x] |
| 51 | `overunder` | `A=N,R=D; B=H; Q=+` | [x] |
| 52 | `overunder` | `A=N,R=D; B=H; Q=-` | [x] |
| 53 | `overunder` | `A=L,R=0; B=L; Q=+` | [x] |
| 54 | `overunder` | `A=L,R=0; B=L; Q=-` | [x] |
| 55 | `overunder` | `A=L,R=0; B=N; Q=+` | [x] |
| 56 | `overunder` | `A=L,R=0; B=N; Q=-` | [x] |
| 57 | `overunder` | `A=L,R=0; B=H; Q=+` | [x] |
| 58 | `overunder` | `A=L,R=0; B=H; Q=-` | [x] |
| 59 | `overunder` | `A=L,R=D; B=L; Q=+` | [x] |
| 60 | `overunder` | `A=L,R=D; B=L; Q=-` | [x] |
| 61 | `overunder` | `A=L,R=D; B=N; Q=+` | [x] |
| 62 | `overunder` | `A=L,R=D; B=N; Q=-` | [x] |
| 63 | `overunder` | `A=L,R=D; B=H; Q=+` | [x] |
| 64 | `overunder` | `A=L,R=D; B=H; Q=-` | [x] |
| 65 | `overunder` | `A=H,R=0; B=L; Q=+` | [x] |
| 66 | `overunder` | `A=H,R=0; B=L; Q=-` | [x] |
| 67 | `overunder` | `A=H,R=0; B=N; Q=+` | [x] |
| 68 | `overunder` | `A=H,R=0; B=N; Q=-` | [x] |
| 69 | `overunder` | `A=H,R=0; B=H; Q=+` | [x] |
| 70 | `overunder` | `A=H,R=0; B=H; Q=-` | [x] |
| 71 | `overunder` | `A=H,R=1; B=L; Q=+` | [x] |
| 72 | `overunder` | `A=H,R=1; B=L; Q=-` | [x] |
| 73 | `overunder` | `A=H,R=1; B=N; Q=+` | [x] |
| 74 | `overunder` | `A=H,R=1; B=N; Q=-` | [x] |
| 75 | `overunder` | `A=H,R=1; B=H; Q=+` | [x] |
| 76 | `overunder` | `A=H,R=1; B=H; Q=-` | [x] |
| 77 | `overunder` | `A=H,R=2; B=L; Q=+` | [x] |
| 78 | `overunder` | `A=H,R=2; B=L; Q=-` | [x] |
| 79 | `overunder` | `A=H,R=2; B=N; Q=+` | [x] |
| 80 | `overunder` | `A=H,R=2; B=N; Q=-` | [x] |
| 81 | `overunder` | `A=H,R=2; B=H; Q=+` | [x] |
| 82 | `overunder` | `A=H,R=2; B=H; Q=-` | [x] |
| 83 | `overunder` | `A=H,R=3; B=L; Q=+` | [x] |
| 84 | `overunder` | `A=H,R=3; B=L; Q=-` | [x] |
| 85 | `overunder` | `A=H,R=3; B=N; Q=+` | [x] |
| 86 | `overunder` | `A=H,R=3; B=N; Q=-` | [x] |
| 87 | `overunder` | `A=H,R=3; B=H; Q=+` | [x] |
| 88 | `overunder` | `A=H,R=3; B=H; Q=-` | [x] |
| 89 | `overunder` | `A=H,R=4; B=L; Q=+` | [x] |
| 90 | `overunder` | `A=H,R=4; B=L; Q=-` | [x] |
| 91 | `overunder` | `A=H,R=4; B=N; Q=+` | [x] |
| 92 | `overunder` | `A=H,R=4; B=N; Q=-` | [x] |
| 93 | `overunder` | `A=H,R=4; B=H; Q=+` | [x] |
| 94 | `overunder` | `A=H,R=4; B=H; Q=-` | [x] |
| 95 | `overunder` | `A=H,R=5; B=L; Q=+` | [x] |
| 96 | `overunder` | `A=H,R=5; B=L; Q=-` | [x] |
| 97 | `overunder` | `A=H,R=5; B=N; Q=+` | [x] |
| 98 | `overunder` | `A=H,R=5; B=N; Q=-` | [x] |
| 99 | `overunder` | `A=H,R=5; B=H; Q=+` | [x] |
| 100 | `overunder` | `A=H,R=5; B=H; Q=-` | [x] |
