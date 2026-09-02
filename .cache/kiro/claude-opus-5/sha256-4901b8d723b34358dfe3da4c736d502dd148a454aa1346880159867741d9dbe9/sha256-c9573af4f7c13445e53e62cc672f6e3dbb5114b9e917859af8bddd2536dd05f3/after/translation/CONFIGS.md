# CONFIGS.md — Configuration surface table (Phase B)

The mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes, not from a guess at what matters.

## Axes the C branches on

There is no `#ifdef` and no `switch` in the library, and the Rust crate declares
no `[features]`, so the only configuration is **runtime**:

**A. Runtime options — all five are environment variables** (`grep -n getenv`):

| var | read in | what it toggles | distinct states the C distinguishes |
|-----|---------|-----------------|--------------------------------------|
| `PROG_VERBOSE`     | `init_config_from_env` | `flags.verbose` → `<<1` in `apply_bit_operations`, plus 6 `printf` sites | absent / present-without-`'1'` / present-containing-`'1'` (test is `strchr(v,'1')`, **not** equality, so `"a1b"` and `"31"` count) |
| `PROG_DEBUG`       | `init_config_from_env` | `flags.debug` → 2 `printf`s in `perform_operation`, 2 in `envy`, 1 gated on `second_colon` | absent / present-without-`'1'` / present-containing-`'1'` |
| `PROG_OPTIMIZE`    | `init_config_from_env` | `flags.optimize` → selects `val1+val2` vs `val1*log_level + val2/2` | absent / present (**content never inspected** — `""` and `"0"` both enable it) |
| `PROG_BASE_OFFSET` | `parse_env_numeric`, default `0100` = 64 | additive term added after the bit ops | absent / accepted numeric / rejected (`,` or `;`) → default |
| `PROG_MULTIPLIER`  | `parse_env_numeric`, default `012` = 10 | `state.multiplier`, multiplied by `param3` | absent / accepted numeric / rejected → default |

`flags.cache_enabled` is hard-wired to `1` and `flags.log_level` to `03` by
`init_config_from_env`, so via `envy` they are constant — **but** the low-level
entry points `perform_operation` / `apply_bit_operations` take a
`struct ConfigFlags*` straight from the caller, so all 2^8 low-byte patterns
(including `cache_enabled == 0` and `log_level` in `0..7`) are reachable
configurations that only the low-level API exposes. Those are rows 12–19.

**B. Input shapes the code special-cases:**

* `param3 == 0` vs `!= 0` (`lib.c:145`) — gates a whole term.
* `param4 == 0` vs `!= 0` (`lib.c:149`) — gates a whole term; sign of `param4`
  changes the arithmetic-shift result.
* sign of the accumulated `result` (`lib.c:171`) — gates the backup-restore path.
* `flags.optimize` (`lib.c:87`) — selects between two entirely different formulas.
* `flags.verbose` (`lib.c:104`) — gates the `<<1`.
* `flags.cache_enabled` (`lib.c:108`) — gates the `| 0x0F`.
* `log_level` value `0..7` — a multiplier, so `0` collapses the term and `7`
  maximises overflow pressure.
* boundary values `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`
  for every `int` parameter.
* `struct ConfigFlags` upper 24 padding bits: zero vs garbage.

**C. Full set of public entry points** — all five, driven directly, not only via
the `envy` one-shot wrapper: `parse_env_numeric`, `init_config_from_env`,
`perform_operation`, `apply_bit_operations`, `envy`.

## Rows

Every row is exercised by `tests/phase_b_configs.rs` against **both** `.so`
files loaded with `libloading`, comparing the returned `int`, the mutated
`ConfigFlags` bytes, **and** the captured `stdout` + `stderr` bytes. Each row
uses many pseudo-random inputs from a fixed-seed SplitMix64 generator (seed
`0x5EED_1234_ABCD_0001`) mixed with the boundary values listed above — never a
single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `parse_env_numeric` | variable absent; `default_val` swept over boundaries + 256 random `i32` | [x] |
| 2  | `parse_env_numeric` | variable present with an accepted decimal value; 256 random `i32` round-tripped through the environment, plus `"+7"`, `" 42"`, `"007"`, `"-0"` | [x] |
| 3  | `parse_env_numeric` | variable present, value rejected by the `,` branch → default; random defaults | [x] |
| 4  | `parse_env_numeric` | variable present, value rejected by the `;` branch → default; random defaults | [x] |
| 5  | `init_config_from_env` | all 3×3×2 = 18 combinations of the `PROG_VERBOSE` / `PROG_DEBUG` / `PROG_OPTIMIZE` states, into zeroed storage; compare all 4 storage bytes | [x] |
| 6  | `init_config_from_env` | same 18 combinations, into storage pre-filled with 8 different garbage patterns (`0xFFFFFFFF`, `0xDEADBEEF`, …) — checks the padding-bit preservation | [x] |
| 7  | `init_config_from_env` | `'1'`-detection shapes: `"1"`, `"01"`, `"10"`, `"a1"`, `"1a"`, `"111"`, `"21"`, `""`, `"0"`, `"true"`, `"one"` for both `PROG_VERBOSE` and `PROG_DEBUG` | [x] |
| 8  | `perform_operation` | `optimize = 1` (`val1 + val2` path), `log_level` irrelevant → swept `0..7` anyway; boundary × boundary grid + 512 random `(val1, val2)` | [x] |
| 9  | `perform_operation` | `optimize = 0`, `log_level = 0` (multiply term collapses to `val2/2`); boundary grid + 512 random pairs | [x] |
| 10 | `perform_operation` | `optimize = 0`, `log_level = 3` (the value `init_config_from_env` actually installs); boundary grid + 512 random pairs | [x] |
| 11 | `perform_operation` | `optimize = 0`, `log_level` = each of `1,2,4,5,6,7`; boundary grid + 256 random pairs each | [x] |
| 12 | `perform_operation` | `debug = 1` with `optimize` both ways — enables the two `printf`s; compares captured stdout bytes for the `%o` and `%d` formats | [x] |
| 13 | `perform_operation` | **all 256** low-byte `ConfigFlags` patterns × boundary values × 64 random pairs each | [x] |
| 14 | `perform_operation` | `ConfigFlags` upper 24 bits set to garbage, low byte swept `0..255` | [x] |
| 15 | `apply_bit_operations` | `verbose = 0`, `cache_enabled = 0` (identity path); boundaries + 512 random values | [x] |
| 16 | `apply_bit_operations` | `verbose = 0`, `cache_enabled = 1` (`\| 0x0F` only); boundaries + 512 random values | [x] |
| 17 | `apply_bit_operations` | `verbose = 1`, `cache_enabled = 0` (`<<1` only, incl. sign-bit overflow); boundaries + 512 random values | [x] |
| 18 | `apply_bit_operations` | `verbose = 1`, `cache_enabled = 1` (`<<1` then `\| 0x0F`); boundaries + 512 random values | [x] |
| 19 | `apply_bit_operations` | **all 256** low-byte `ConfigFlags` patterns (+ garbage padding) × boundaries × 64 random values | [x] |
| 20 | `envy` | all env vars absent (pure default config: verbose=0 debug=0 optimize=0, base_offset=64, multiplier=10); boundary 4-tuples + 512 random 4-tuples | [x] |
| 21 | `envy` | `PROG_OPTIMIZE` set only → the `val1+val2` formula; boundaries + 512 random 4-tuples | [x] |
| 22 | `envy` | `PROG_VERBOSE=1` only → `<<1` path plus 6 stdout lines incl. `"Found colon at position: %ld"`; boundaries + 256 random 4-tuples | [x] |
| 23 | `envy` | `PROG_DEBUG=1` only → 5 debug stdout lines incl. the `%o` octal one; boundaries + 256 random 4-tuples | [x] |
| 24 | `envy` | all 8 combinations of `verbose` × `debug` × `optimize` effective flag values, defaults for the two numeric vars; boundaries + 128 random 4-tuples each | [x] |
| 25 | `envy` | `PROG_BASE_OFFSET` swept over accepted values (0, ±1, ±random, `INT_MIN`, `INT_MAX`) × `PROG_MULTIPLIER` swept likewise — the interaction that drives `result` across the `result < 0` boundary | [x] |
| 26 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected (`,` / `;`) so defaults apply, × all 8 flag combos — checks the stderr warnings interleave with stdout identically | [x] |
| 27 | `envy` | input shape grid: `param3 ∈ {0, nonzero}` × `param4 ∈ {0, positive, negative}` × `param1` sign × `param2` sign (2×3×2×2 = 24 shapes), each with 64 random magnitudes | [x] |
| 28 | `envy` | configurations tuned so the final `result < 0` backup-restore branch fires (large negative `PROG_BASE_OFFSET`, negative params), × all 8 flag combos | [x] |
| 29 | `envy` | `param4` sweep over `{INT_MIN, -5, -4, -3, -1, 1, 3, 4, 5, INT_MAX}` to pin the arithmetic `>> 2` rounding for negatives, × optimize on/off | [x] |
| 30 | full pipeline | `init_config_from_env` → `perform_operation` → `apply_bit_operations` composed **by the test** (not via `envy`), across the 18 env states × 256 random pairs — catches divergence that per-wrapper tests hide | [x] |

**Rows: 30. Unchecked rows: 0.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one; `--features`
combos do not exist. Verified mechanically by
`tests/symbol_parity.rs::cargo_toml_declares_no_features` and by the
`check_all_feature_combos.sh` script in the crate root, which extracts the
feature list from `cargo metadata` and loops `cargo check` over every subset
(the loop degenerates to the two runs `--no-default-features` and default).
