# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branches the C code actually takes.

## Build-time configuration axes

* `Cargo.toml` has **no `[features]` table** → the only Cargo configuration is
  the default (== `--no-default-features`). Verified with
  `grep -n features Cargo.toml` (no matches).
* `c_src/CMakeLists.txt` has **no** `option()`, no `add_definitions`, no
  `#ifdef`-selected sources — a single `SHARED` library from `src/lib.c` with no
  preprocessor knobs. `grep -c '#if' c_src/src/lib.c` → 0.

⇒ **Exactly one build configuration exists.** Every row below is therefore
verified under "every feature combination" trivially, and the test suite is
additionally run with `--no-default-features` and `--all-features`.

## Runtime configuration axes (from the source)

| axis | where | distinct values the C branches on |
|------|-------|------------------------------------|
| mode string | `classify_mode` 4× `strcmp` (`lib.c:30-37`) | `"standard"`, `"enhanced"`, `"turbo"`, `"extreme"`, anything else |
| `level` | `apply_multiplier` `switch` (`lib.c:45-59`) | `4`, `3`, `2`, `1`, `0` (all fall through), `default` |
| `base` | `apply_multiplier` accumulator | `0xA0` (what `modeselect` passes), `0`, negative, `INT_MAX`, `INT_MIN`, random |
| double magnitude | `cvttsd2si` in both converters | in-`int`-range product, `>= 2^31`, `< -2^31`, `NaN`, `±inf`, `±0`, subnormal, exact `±2^31` boundary |
| `offset_days`, `offset_hours` | `get_modified_time` `int` arithmetic (`lib.c:81`) | `0`, small +, small -, values whose `int` product/sum overflows, `INT_MIN`/`INT_MAX` |
| `time_t` byte pattern | `hash_time_value` loop over 8 bytes (`lib.c:89-92`) | `0`, small +, small -, `INT64_MIN`, `INT64_MAX`, high-bit-set bytes (`>= 0x80` at `i%4==3`), random |
| `mode_selector % 4` | `modeselect` index (`lib.c:101`) | `0`, `1`, `2`, `3` (valid); `-1`, `-2`, `-3` → OOB (see `ERRORS.md` row 19) |
| `complexity % 5` | `modeselect` (`lib.c:108`) | `0`, `1`, `2`, `3`, `4` (valid); `-1..-4` → `default` |
| `seed % 24` | `modeselect` → `offset_hours` (`lib.c:114`) | `0`, positive, negative |
| `seed` | `modeselect` → `factor1` (`lib.c:120`) | `0` (in-range product) vs `!= 0` (overflowing product) |
| `time_offset` | `modeselect` → `offset_days` **and** `factor2` (`lib.c:114,121`) | `0` vs `!= 0`, sign, overflowing magnitude |
| stdout | every `printf` in `modeselect` (`lib.c:105,111,117,123,126,128,130,137`) | byte-exact text is part of the observable output and is compared for every row |

## Configuration rows

Every row is exercised with **many randomized inputs** (fixed seed
`0x5A5A_5A5A_DEAD_BEEF`, splitmix64) over the free parameters of that row, and
both the return value **and the captured stdout bytes** are compared.

### `classify_mode` (lowest level)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `classify_mode` | exact literal `"standard"` | [x] |
| 2  | `classify_mode` | exact literal `"enhanced"` | [x] |
| 3  | `classify_mode` | exact literal `"turbo"` | [x] |
| 4  | `classify_mode` | exact literal `"extreme"` | [x] |
| 5  | `classify_mode` | empty string `""` | [x] |
| 6  | `classify_mode` | proper prefix of a literal (`"standar"`, `"turb"`, …) | [x] |
| 7  | `classify_mode` | literal + extra bytes (`"standardX"`, `"turbo "`, …) | [x] |
| 8  | `classify_mode` | case-changed literal (`"STANDARD"`, `"Turbo"`) | [x] |
| 9  | `classify_mode` | randomized non-matching byte strings, lengths 1..32, all byte values `1..=255` | [x] |
| 10 | `classify_mode` | strings containing an embedded literal after the NUL terminator | [x] |

### `apply_multiplier`

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 11 | `apply_multiplier` | `level = 0`, `base` ∈ {`0xA0`, `0`, random, `INT_MAX`, `INT_MIN`} | [x] |
| 12 | `apply_multiplier` | `level = 1`, same `base` set | [x] |
| 13 | `apply_multiplier` | `level = 2`, same `base` set | [x] |
| 14 | `apply_multiplier` | `level = 3`, same `base` set | [x] |
| 15 | `apply_multiplier` | `level = 4` (longest fall-through chain), same `base` set | [x] |
| 16 | `apply_multiplier` | `level` random over full `int` range (mostly `default`) × random `base` | [x] |

### `convert_time_factor` / `convert_negative_overflow`

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 17 | `convert_time_factor` | `factor` such that `factor*1e12` lands in `int` range (`|factor| < 2.1e-3`), randomized | [x] |
| 18 | `convert_time_factor` | `factor` straddling the exact `±2^31 / 1e12` boundary (one ULP either side) | [x] |
| 19 | `convert_time_factor` | `factor` ∈ {`0.0`, `-0.0`, `f64::MIN_POSITIVE`, subnormal, `1e-320`} | [x] |
| 20 | `convert_time_factor` | large `|factor|` (overflowing), both signs, randomized exponents `1e-3 .. 1e300` | [x] |
| 21 | `convert_time_factor` | `factor` ∈ {`NaN`, `-NaN`, `+inf`, `-inf`} | [x] |
| 22 | `convert_time_factor` | `factor` random over **all** finite bit patterns (raw `u64` → `f64`, non-NaN filtered in) | [x] |
| 23 | `convert_negative_overflow` | `value` such that `value*-1e15` lands in `int` range (`|value| < 2.1e-6`), randomized | [x] |
| 24 | `convert_negative_overflow` | `value` straddling the exact `±2^31 / 1e15` boundary (one ULP either side) | [x] |
| 25 | `convert_negative_overflow` | `value` ∈ {`0.0`, `-0.0`, subnormals} (sign of the product flips) | [x] |
| 26 | `convert_negative_overflow` | large `|value|` (overflowing), both signs, randomized exponents | [x] |
| 27 | `convert_negative_overflow` | `value` ∈ {`NaN`, `-NaN`, `+inf`, `-inf`} | [x] |
| 28 | `convert_negative_overflow` | `value` random over all finite bit patterns | [x] |

### `get_modified_time`

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 29 | `get_modified_time` | `offset_days = 0`, `offset_hours = 0` (isolates the `time()>>29` term) | [x] |
| 30 | `get_modified_time` | small positive / small negative day+hour combinations | [x] |
| 31 | `get_modified_time` | `offset_hours` restricted to the `seed % 24` range `-23..=23` × random days | [x] |
| 32 | `get_modified_time` | `offset_days` large enough that `days*86400` overflows `int` (both signs) | [x] |
| 33 | `get_modified_time` | `offset_hours` large enough that `hours*3600` overflows `int` (both signs) | [x] |
| 34 | `get_modified_time` | both terms in range but their `int` sum overflows | [x] |
| 35 | `get_modified_time` | `{INT_MIN, INT_MAX, -1, 1}` cross-product of both parameters | [x] |
| 36 | `get_modified_time` | fully random `int` × `int` | [x] |

### `hash_time_value`

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 37 | `hash_time_value` | `t = 0` | [x] |
| 38 | `hash_time_value` | `t` ∈ small positives `1..=64` (only byte 0 varies) | [x] |
| 39 | `hash_time_value` | `t` negative (`-1`, `-2`, …) — all 8 bytes `0xFF`, exercises the `<<24` overflow | [x] |
| 40 | `hash_time_value` | `t` ∈ {`i64::MIN`, `i64::MAX`, `i32::MIN as i64`, `i32::MAX as i64`, `1<<29`, `1<<31`, `1<<63`} | [x] |
| 41 | `hash_time_value` | `t` with every byte `>= 0x80` (high-bit shifts) | [x] |
| 42 | `hash_time_value` | fully random 64-bit patterns | [x] |
| 43 | `hash_time_value` | `t` values actually produced by `get_modified_time` (pipeline composition) | [x] |

### `modeselect` (top-level pipeline, return value **and** stdout)

`mode_index = mode_selector % 4` × `complexity_level = complexity % 5` is the
core cross-product the C distinguishes; `seed`/`time_offset` are randomized
inside each row (and pinned in rows 64–71).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 44 | `modeselect` | `mode_index = 0` ("standard"), `complexity_level = 0`, random `time_offset`/`seed` | [x] |
| 45 | `modeselect` | `mode_index = 0`, `complexity_level = 1` | [x] |
| 46 | `modeselect` | `mode_index = 0`, `complexity_level = 2` | [x] |
| 47 | `modeselect` | `mode_index = 0`, `complexity_level = 3` | [x] |
| 48 | `modeselect` | `mode_index = 0`, `complexity_level = 4` | [x] |
| 49 | `modeselect` | `mode_index = 1` ("enhanced"), `complexity_level = 0` | [x] |
| 50 | `modeselect` | `mode_index = 1`, `complexity_level = 1` | [x] |
| 51 | `modeselect` | `mode_index = 1`, `complexity_level = 2` | [x] |
| 52 | `modeselect` | `mode_index = 1`, `complexity_level = 3` | [x] |
| 53 | `modeselect` | `mode_index = 1`, `complexity_level = 4` | [x] |
| 54 | `modeselect` | `mode_index = 2` ("turbo"), `complexity_level = 0` | [x] |
| 55 | `modeselect` | `mode_index = 2`, `complexity_level = 1` | [x] |
| 56 | `modeselect` | `mode_index = 2`, `complexity_level = 2` | [x] |
| 57 | `modeselect` | `mode_index = 2`, `complexity_level = 3` | [x] |
| 58 | `modeselect` | `mode_index = 2`, `complexity_level = 4` | [x] |
| 59 | `modeselect` | `mode_index = 3` ("extreme"), `complexity_level = 0` | [x] |
| 60 | `modeselect` | `mode_index = 3`, `complexity_level = 1` | [x] |
| 61 | `modeselect` | `mode_index = 3`, `complexity_level = 2` | [x] |
| 62 | `modeselect` | `mode_index = 3`, `complexity_level = 3` | [x] |
| 63 | `modeselect` | `mode_index = 3`, `complexity_level = 4` | [x] |
| 64 | `modeselect` | `seed = 0` (the only value for which `result1 != INT_MIN`) × all 4 mode indices | [x] |
| 65 | `modeselect` | `time_offset = 0` (the only value for which `result2 != INT_MIN`) × all 4 mode indices | [x] |
| 66 | `modeselect` | `seed = 0` **and** `time_offset = 0` (both converters in range) | [x] |
| 67 | `modeselect` | `seed % 24` negative (negative `seed`) with `mode_selector % 4 == 0` | [x] |
| 68 | `modeselect` | `seed` a positive multiple of 24 (`offset_hours == 0`) | [x] |
| 69 | `modeselect` | `time_offset` large positive/negative so `days*86400` overflows `int` | [x] |
| 70 | `modeselect` | `complexity` negative (`complexity_level < 0` → `0xDEAD`), `mode_index` valid | [x] |
| 71 | `modeselect` | `{INT_MIN, INT_MAX, 0, -1, 1}` cross-product over all four parameters, skipping the `SIGSEGV` combinations of `ERRORS.md` row 19 | [x] |
| 72 | `modeselect` | fully random 4-tuples with `mode_selector` forced non-negative | [x] |
| 73 | `modeselect` | `mode_selector` a negative multiple of 4 (`mode_index == 0`, no OOB) | [x] |
| 74 | `modeselect` | stdout byte-exactness for the `%.2e` / `%ld` / `%X` formats over random `seed`/`time_offset` (covered by capturing stdout in every row above) | [x] |

## Additional build axis found during verification

`cargo test` does **not** rebuild a `cdylib`-only library target (no test target
links it), so a test that `dlopen`s `target/<profile>/libmodeselect_lib.so`
silently exercises a **stale** artifact. This was caught by mutation testing
(ten deliberate bugs were injected into `src/lib.rs` and *all* were reported as
passing). `build.rs` now rebuilds both libraries on every source change and
hands their paths to the tests via `env!()`, and it produces the Rust `.so`
twice:

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 75 | all 7 | Rust `.so` built with `-C opt-level=0` vs the C `.so` | [x] |
| 76 | all 7 | Rust `.so` built with `-C opt-level=3` vs the C `.so` | [x] |
| 77 | all 7 | `-C opt-level=0` Rust `.so` vs `-C opt-level=3` Rust `.so` | [x] |
| 78 | all 7 | whole suite under `--no-default-features`, default, `--all-features` and `--release` | [x] |

Rows 75–78 are covered by `tests/phase_d_parity.rs` (`battery_*`) and by
`scripts/verify.sh`.

## How to run

```sh
./scripts/verify.sh          # every feature combination + symbol diff
cargo test                   # build.rs refreshes both .so files first
./scripts/symbol_diff.sh     # Phase D symbol parity only
```
