# CONFIGS.md — Phase A: configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from
the branches the C actually takes.

## Axes the C code branches on

| axis | where in C | distinct states |
|------|-----------|-----------------|
| A1 — `printLine` null test | `driver.c:30` `if (line != NULL)` | non-null / null |
| A2 — payload shape passed to `puts` | `driver.c:32` `printf("%s\n", line)` | empty, 1 byte, short, long/oversized, high-bytes, format-looking, whitespace/newline-embedded |
| A3 — `driver` truthiness test | `driver.c:51` `if (useGood)` | zero / non-zero (incl. negative, INT_MIN, INT_MAX, high-bit-only) |
| A4 — entry point / call level | 4 exported symbols | `printLine` (lowest level), `good`, `bad` (mid level), `driver` (top-level wrapper) |
| A5 — call sequencing / state | none — library is **stateless**, no globals, no init/teardown | single call / repeated calls / interleaved order |
| A6 — build-time options | `CMakeLists.txt` has no `option()`/`#ifdef`; `Cargo.toml` has **no `[features]`** | exactly one configuration (default) |

There are no runtime option setters, no modes, no flags, no byte-order or
element-type axes: the only "configuration" the C exposes is the `int`
argument to `driver` and the pointer argument to `printLine`.

## Row set (pruned cross-product of A1–A5)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-null, empty string `""` (len 0) | [x] |
| 2 | `printLine` | non-null, single byte, randomized over all 1..=255 byte values | [x] |
| 3 | `printLine` | non-null, short random ASCII strings, len 1..32, 256 seeded random cases | [x] |
| 4 | `printLine` | non-null, random strings over full non-zero byte range `0x01..=0xFF` (high bytes / invalid UTF-8), len 1..64, 256 seeded cases | [x] |
| 5 | `printLine` | non-null, long/oversized buffer: len 1 KiB, 8 KiB, 64 KiB, 1 MiB | [x] |
| 6 | `printLine` | non-null, payload containing `printf` directives: `%s`, `%d`, `%n`, `%%`, `%99999d` | [x] |
| 7 | `printLine` | non-null, payload with embedded `\n`, `\t`, `\r` (multi-line output through `puts`) | [x] |
| 8 | `printLine` | non-null, interior pointer into a larger buffer (`buf.as_ptr().add(k)`), randomized `k` | [x] |
| 9 | `printLine` | non-null, called repeatedly (1000×) to prove statelessness / stable stdout ordering | [x] |
| 10 | `good` | no arguments, single call | [x] |
| 11 | `good` | no arguments, repeated 100× (statelessness) | [x] |
| 12 | `driver` | `useGood = 1` (canonical truthy) → `good` path | [x] |
| 13 | `driver` | `useGood` = seeded random non-zero `i32`, 512 cases → `good` path | [x] |
| 14 | `driver` | `useGood` = boundary truthy values: `-1`, `i32::MIN`, `i32::MAX`, `1<<8`, `1<<16`, `1<<31`-as-min, `0x0000_0100` | [x] |
| 15 | `driver` | `useGood = 0` → `bad` path (UB payload; see ERRORS.md row 11) | [x] |
| 16 | `bad` | no arguments, single call (lowest-level entry to the defect) | [x] |
| 17 | mixed pipeline | `driver(1)` → `printLine("x")` → `good()` → `driver(0)` → `printLine(NULL)` in one process, stdout compared as one stream | [x] |
| 18 | all four | seeded random *interleaving* of the 4 entry points, 200 ops, one captured stdout stream per library | [x] |

Each row is implemented as `rowNN_*` in `tests/phase_b_configs.rs` and is
checked with **many seeded-random inputs** (SplitMix64, fixed seeds
`0x5EED_00NN`), not a single hand-picked value: 255 exhaustive single bytes,
2×256 random strings, 512 random `i32`s, 8193 exhaustive `driver` values in
Phase C, 1000-call and 200-op composed streams.

Row 15/16 (the `bad()` path) is the one row that is not byte-compared: the C
reads an uninitialized `char *` and its output is caller-stack dependent — and
it SIGSEGVs for the `driver(0)` call shape. See `ERRORS.md` row 11 for the
measurement and for the contract that is asserted instead.

## Feature / profile matrix

`Cargo.toml` declares **no `[features]` table** and no optional dependencies,
so the feature powerset is `{default} == {--no-default-features}`. Verified
with `cargo metadata --no-deps` → `features: {}`. Both spellings are still run,
across both profiles (`--release` additionally enables `panic = "abort"`, a
different codegen path), by `./run_all_configs.sh`:

| profile | features | result |
|---------|----------|--------|
| debug   | default | 36/36 pass |
| debug   | `--no-default-features` | 36/36 pass |
| release | default | 36/36 pass |
| release | `--no-default-features` | 36/36 pass |

## Binary executable: N/A

Neither build produces a driver executable — `c_src/CMakeLists.txt` has 0
`add_executable()` calls (only `add_library(driver SHARED …)`) and
`translation/Cargo.toml` has no `[[bin]]` target and no `src/main.rs`. There is
no stdout-of-binary comparison to make; the library's stdout is compared
directly through the `.so` exports instead.
