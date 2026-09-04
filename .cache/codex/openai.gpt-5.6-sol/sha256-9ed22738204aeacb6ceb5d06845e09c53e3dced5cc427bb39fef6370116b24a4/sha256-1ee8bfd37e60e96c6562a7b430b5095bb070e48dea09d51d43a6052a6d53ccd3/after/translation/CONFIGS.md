# Configuration Surface

The public headers expose one entry point and no runtime options, modes, flags,
pointers, element types, counts, formats, or byte-order settings. `Cargo.toml`
declares no features. The meaningful valid-input axes arise from the signed C
`div` operation used by `driver`:

- numerator shape: zero, positive, or negative;
- denominator sign: positive or negative;
- result shape for nonzero numerators: exact division or nonzero remainder;
- C `int` boundaries, excluding the two rejection cases in `ERRORS.md`.

Each row is exercised with many deterministic randomized inputs through both
shared-library FFI exports. Exact output includes the complete bytes written by
`printf`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `driver` | no options; `x == 0`, `y > 0` | [x] |
| 2 | `driver` | no options; `x == 0`, `y < 0` | [x] |
| 3 | `driver` | no options; `x > 0`, `y > 0`, exact division | [x] |
| 4 | `driver` | no options; `x > 0`, `y > 0`, nonzero remainder | [x] |
| 5 | `driver` | no options; `x > 0`, `y < 0`, exact division | [x] |
| 6 | `driver` | no options; `x > 0`, `y < 0`, nonzero remainder | [x] |
| 7 | `driver` | no options; `x < 0`, `y > 0`, exact division | [x] |
| 8 | `driver` | no options; `x < 0`, `y > 0`, nonzero remainder | [x] |
| 9 | `driver` | no options; `x < 0`, `y < 0`, exact division | [x] |
| 10 | `driver` | no options; `x < 0`, `y < 0`, nonzero remainder | [x] |
| 11 | `driver` | no options; valid `x` boundary values (`INT_MIN`, `INT_MAX`) | [x] |
| 12 | `driver` | no options; valid `y` boundary values (`INT_MIN`, `INT_MAX`) | [x] |

Feature/build configurations:

| # | Cargo feature configuration | verified |
|---|-----------------------------|----------|
| F1 | default (no features declared) | [x] |
| F2 | `--no-default-features` (equivalent empty feature set) | [x] |
