# Configuration Surface

The public header declares `driver`; `nm -D` additionally exposes the
lower-level `print_foo` called by `driver`. There are no runtime options,
formats, element types, byte-order modes, counts, preprocessor configurations,
or Cargo features.

The source mechanically exposes these data-shape axes:

- `foo_t.x` is a 2-bit unsigned field: source values `0..=3` are retained and
  values `4..=UINT_MAX` are truncated modulo 4.
- `foo_t.y` is a 3-bit unsigned field: source values `0..=7` are retained and
  values `8..=UINT_MAX` are truncated modulo 8.
- `foo_t.b` is a 1-bit C `bool`: `false` and `true`.
- `foo_t.z` is printed as signed `int`: negative, zero, and positive values,
  including `INT_MIN` and `INT_MAX`.

Every row exercises both `driver` and `print_foo` directly through each shared
library. For `print_foo`, the test constructs the ABI layout produced by the C
compiler, including randomized ignored padding bits.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver`, `print_foo` | x retained; y retained; b false; z negative | [x] |
| 2 | `driver`, `print_foo` | x retained; y retained; b false; z zero | [x] |
| 3 | `driver`, `print_foo` | x retained; y retained; b false; z positive | [x] |
| 4 | `driver`, `print_foo` | x retained; y retained; b true; z negative | [x] |
| 5 | `driver`, `print_foo` | x retained; y retained; b true; z zero | [x] |
| 6 | `driver`, `print_foo` | x retained; y retained; b true; z positive | [x] |
| 7 | `driver`, `print_foo` | x retained; y truncated; b false; z negative | [x] |
| 8 | `driver`, `print_foo` | x retained; y truncated; b false; z zero | [x] |
| 9 | `driver`, `print_foo` | x retained; y truncated; b false; z positive | [x] |
| 10 | `driver`, `print_foo` | x retained; y truncated; b true; z negative | [x] |
| 11 | `driver`, `print_foo` | x retained; y truncated; b true; z zero | [x] |
| 12 | `driver`, `print_foo` | x retained; y truncated; b true; z positive | [x] |
| 13 | `driver`, `print_foo` | x truncated; y retained; b false; z negative | [x] |
| 14 | `driver`, `print_foo` | x truncated; y retained; b false; z zero | [x] |
| 15 | `driver`, `print_foo` | x truncated; y retained; b false; z positive | [x] |
| 16 | `driver`, `print_foo` | x truncated; y retained; b true; z negative | [x] |
| 17 | `driver`, `print_foo` | x truncated; y retained; b true; z zero | [x] |
| 18 | `driver`, `print_foo` | x truncated; y retained; b true; z positive | [x] |
| 19 | `driver`, `print_foo` | x truncated; y truncated; b false; z negative | [x] |
| 20 | `driver`, `print_foo` | x truncated; y truncated; b false; z zero | [x] |
| 21 | `driver`, `print_foo` | x truncated; y truncated; b false; z positive | [x] |
| 22 | `driver`, `print_foo` | x truncated; y truncated; b true; z negative | [x] |
| 23 | `driver`, `print_foo` | x truncated; y truncated; b true; z zero | [x] |
| 24 | `driver`, `print_foo` | x truncated; y truncated; b true; z positive | [x] |
