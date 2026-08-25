# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
options or conditional sources. There is exactly one valid build
configuration:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| 1 | `--no-default-features` (empty feature set) | default, with position-independent code | [x] |

## Runtime Configurations

The only public entry point is the lowest-level function `encode_quant`.
The rows below are the full cross-product of branches that independently
change candidate construction:

- `uni & 7`: all eight values, including the decrement clamp at 0 and
  increment clamp at 7.
- `uni & 8`: clear or set, selecting the positive or negative `diff` branch
  for all candidates.
- `lsbit`: zero, exactly 4, nonzero odd, or nonzero even other than 4.

For every row, randomized `step`, `pred`, `tgt`, and `tgt2` values cover the
three prediction calculations, both candidate-selection comparisons, zero,
both signs, full-width values, and integer extrema. Multiple representatives
are used for the odd and even `lsbit` classes.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `encode_quant` | `uni&7=0`, `uni&8=0`; `lsbit=0` | [x] |
| 2 | `encode_quant` | `uni&7=1`, `uni&8=0`; `lsbit=0` | [x] |
| 3 | `encode_quant` | `uni&7=2`, `uni&8=0`; `lsbit=0` | [x] |
| 4 | `encode_quant` | `uni&7=3`, `uni&8=0`; `lsbit=0` | [x] |
| 5 | `encode_quant` | `uni&7=4`, `uni&8=0`; `lsbit=0` | [x] |
| 6 | `encode_quant` | `uni&7=5`, `uni&8=0`; `lsbit=0` | [x] |
| 7 | `encode_quant` | `uni&7=6`, `uni&8=0`; `lsbit=0` | [x] |
| 8 | `encode_quant` | `uni&7=7`, `uni&8=0`; `lsbit=0` | [x] |
| 9 | `encode_quant` | `uni&7=0`, `uni&8=8`; `lsbit=0` | [x] |
| 10 | `encode_quant` | `uni&7=1`, `uni&8=8`; `lsbit=0` | [x] |
| 11 | `encode_quant` | `uni&7=2`, `uni&8=8`; `lsbit=0` | [x] |
| 12 | `encode_quant` | `uni&7=3`, `uni&8=8`; `lsbit=0` | [x] |
| 13 | `encode_quant` | `uni&7=4`, `uni&8=8`; `lsbit=0` | [x] |
| 14 | `encode_quant` | `uni&7=5`, `uni&8=8`; `lsbit=0` | [x] |
| 15 | `encode_quant` | `uni&7=6`, `uni&8=8`; `lsbit=0` | [x] |
| 16 | `encode_quant` | `uni&7=7`, `uni&8=8`; `lsbit=0` | [x] |
| 17 | `encode_quant` | `uni&7=0`, `uni&8=0`; `lsbit=4` | [x] |
| 18 | `encode_quant` | `uni&7=1`, `uni&8=0`; `lsbit=4` | [x] |
| 19 | `encode_quant` | `uni&7=2`, `uni&8=0`; `lsbit=4` | [x] |
| 20 | `encode_quant` | `uni&7=3`, `uni&8=0`; `lsbit=4` | [x] |
| 21 | `encode_quant` | `uni&7=4`, `uni&8=0`; `lsbit=4` | [x] |
| 22 | `encode_quant` | `uni&7=5`, `uni&8=0`; `lsbit=4` | [x] |
| 23 | `encode_quant` | `uni&7=6`, `uni&8=0`; `lsbit=4` | [x] |
| 24 | `encode_quant` | `uni&7=7`, `uni&8=0`; `lsbit=4` | [x] |
| 25 | `encode_quant` | `uni&7=0`, `uni&8=8`; `lsbit=4` | [x] |
| 26 | `encode_quant` | `uni&7=1`, `uni&8=8`; `lsbit=4` | [x] |
| 27 | `encode_quant` | `uni&7=2`, `uni&8=8`; `lsbit=4` | [x] |
| 28 | `encode_quant` | `uni&7=3`, `uni&8=8`; `lsbit=4` | [x] |
| 29 | `encode_quant` | `uni&7=4`, `uni&8=8`; `lsbit=4` | [x] |
| 30 | `encode_quant` | `uni&7=5`, `uni&8=8`; `lsbit=4` | [x] |
| 31 | `encode_quant` | `uni&7=6`, `uni&8=8`; `lsbit=4` | [x] |
| 32 | `encode_quant` | `uni&7=7`, `uni&8=8`; `lsbit=4` | [x] |
| 33 | `encode_quant` | `uni&7=0`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 34 | `encode_quant` | `uni&7=1`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 35 | `encode_quant` | `uni&7=2`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 36 | `encode_quant` | `uni&7=3`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 37 | `encode_quant` | `uni&7=4`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 38 | `encode_quant` | `uni&7=5`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 39 | `encode_quant` | `uni&7=6`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 40 | `encode_quant` | `uni&7=7`, `uni&8=0`; `lsbit` nonzero odd | [x] |
| 41 | `encode_quant` | `uni&7=0`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 42 | `encode_quant` | `uni&7=1`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 43 | `encode_quant` | `uni&7=2`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 44 | `encode_quant` | `uni&7=3`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 45 | `encode_quant` | `uni&7=4`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 46 | `encode_quant` | `uni&7=5`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 47 | `encode_quant` | `uni&7=6`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 48 | `encode_quant` | `uni&7=7`, `uni&8=8`; `lsbit` nonzero odd | [x] |
| 49 | `encode_quant` | `uni&7=0`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 50 | `encode_quant` | `uni&7=1`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 51 | `encode_quant` | `uni&7=2`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 52 | `encode_quant` | `uni&7=3`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 53 | `encode_quant` | `uni&7=4`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 54 | `encode_quant` | `uni&7=5`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 55 | `encode_quant` | `uni&7=6`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 56 | `encode_quant` | `uni&7=7`, `uni&8=0`; `lsbit` nonzero even and not 4 | [x] |
| 57 | `encode_quant` | `uni&7=0`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 58 | `encode_quant` | `uni&7=1`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 59 | `encode_quant` | `uni&7=2`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 60 | `encode_quant` | `uni&7=3`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 61 | `encode_quant` | `uni&7=4`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 62 | `encode_quant` | `uni&7=5`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 63 | `encode_quant` | `uni&7=6`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
| 64 | `encode_quant` | `uni&7=7`, `uni&8=8`; `lsbit` nonzero even and not 4 | [x] |
