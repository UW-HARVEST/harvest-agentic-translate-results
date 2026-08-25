# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime Configurations

Rows are derived from all externally linked C entry points and from the
`switch (mode)`, iteration loop boundaries, threshold branch, and
`UINT16_MAX` count branch in `c_src/src/lib.c`. "Random values" includes
ordinary and arithmetic-boundary `int` values.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `process_value` | random `int` values, including wrapping boundary values; unused scalar/context arguments varied | [x] |
| 2 | `double_value` | random `int` values, including wrapping boundary values; unused scalar/context arguments varied | [x] |
| 3 | `triple_value` | random `int` values, including wrapping boundary values; unused scalar/context arguments varied | [x] |
| 4 | `gotomach` | mode `0`; zero iterations (empty input) | [x] |
| 5 | `gotomach` | mode `1`; zero iterations (empty input) | [x] |
| 6 | `gotomach` | mode `2`; zero iterations (empty input) | [x] |
| 7 | `gotomach` | invalid mode (default branch); zero iterations (empty input) | [x] |
| 8 | `gotomach` | mode `0`; one iteration; transformed value rejected by threshold | [x] |
| 9 | `gotomach` | mode `1`; one iteration; transformed value rejected by threshold | [x] |
| 10 | `gotomach` | mode `2`; one iteration; transformed value rejected by threshold | [x] |
| 11 | `gotomach` | invalid mode (default branch); one iteration; transformed value rejected by threshold | [x] |
| 12 | `gotomach` | mode `0`; one iteration; transformed value accepted by threshold | [x] |
| 13 | `gotomach` | mode `1`; one iteration; transformed value accepted by threshold | [x] |
| 14 | `gotomach` | mode `2`; one iteration; transformed value accepted by threshold | [x] |
| 15 | `gotomach` | invalid mode (default branch); one iteration; transformed value accepted by threshold | [x] |
| 16 | `gotomach` | mode `0`; many iterations; threshold rejects every transformed value | [x] |
| 17 | `gotomach` | mode `1`; many iterations; threshold rejects every transformed value | [x] |
| 18 | `gotomach` | mode `2`; many iterations; threshold rejects every transformed value | [x] |
| 19 | `gotomach` | invalid mode (default branch); many iterations; threshold rejects every transformed value | [x] |
| 20 | `gotomach` | mode `0`; many iterations; threshold accepts and rejects transformed values | [x] |
| 21 | `gotomach` | mode `1`; many iterations; threshold accepts and rejects transformed values | [x] |
| 22 | `gotomach` | mode `2`; many iterations; threshold accepts and rejects transformed values | [x] |
| 23 | `gotomach` | invalid mode (default branch); many iterations; threshold accepts and rejects transformed values | [x] |
| 24 | `gotomach` | mode `0`; many iterations; threshold accepts every transformed value | [x] |
| 25 | `gotomach` | mode `1`; many iterations; threshold accepts every transformed value | [x] |
| 26 | `gotomach` | mode `2`; many iterations; threshold accepts every transformed value | [x] |
| 27 | `gotomach` | invalid mode (default branch); many iterations; threshold accepts every transformed value | [x] |
| 28 | `gotomach` | mode `0`; `65535` iterations; all accepted, reaching `count >= UINT16_MAX` | [x] |
| 29 | `gotomach` | mode `1`; `65535` iterations; all accepted, reaching `count >= UINT16_MAX` | [x] |
| 30 | `gotomach` | mode `2`; `65535` iterations; all accepted, reaching `count >= UINT16_MAX` | [x] |
| 31 | `gotomach` | invalid mode (default branch); `65535` iterations; all accepted, reaching `count >= UINT16_MAX` | [x] |
| 32 | `gotomach` | mode `0`; `65535` iterations; not all accepted, so the full loop completes without the count stop | [x] |
| 33 | `gotomach` | mode `1`; `65535` iterations; not all accepted, so the full loop completes without the count stop | [x] |
| 34 | `gotomach` | mode `2`; `65535` iterations; not all accepted, so the full loop completes without the count stop | [x] |
| 35 | `gotomach` | invalid mode (default branch); `65535` iterations; not all accepted, so the full loop completes without the count stop | [x] |

Scalar boundaries `iterations = 0`, `iterations = 65535`, `seed = 0`,
`seed = 65535`, and `threshold = INT_MIN/INT_MAX` are randomized within the
applicable rows. Invalid scalar boundaries are listed in `ERRORS.md`.
