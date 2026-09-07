# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the `if`/`else`/ternary branches in
`c_src/src/lib.c`. Every row is exercised against **both** `.so`s through
`libloading` with **many randomized inputs** (fixed seed, SplitMix64 PRNG in
`tests/common/mod.rs`), comparing the returned `int` **and** the bytes written
to stdout **and** stderr.

## Axes the C code actually branches on

### Runtime options (there are no compile-time `#ifdef`s; all configuration is environment-driven)

| axis | read by | distinct states the C distinguishes |
|------|---------|--------------------------------------|
| `PROG_VERBOSE` | `init_config_from_env` | (a) unset → `verbose=0`; (b) set **containing `'1'`** → `verbose=1`; (c) set **without** `'1'` → `verbose=0` |
| `PROG_DEBUG` | `init_config_from_env` | (a) unset → `debug=0`; (b) set containing `'1'` → `debug=1`; (c) set without `'1'` → `debug=0` |
| `PROG_OPTIMIZE` | `init_config_from_env` | (a) unset → `optimize=0`; (b) set to `""` → `optimize=1`; (c) set to anything → `optimize=1` (content ignored) |
| `PROG_BASE_OFFSET` | `parse_env_numeric` in `envy` | (a) unset → default `0100`=64; (b) parseable → `atoi`; (c) contains `,` → warn+default; (d) contains `;` → warn+default; (e) `""` → 0 |
| `PROG_MULTIPLIER` | `parse_env_numeric` in `envy` | same five states, default `012`=10 |

### `ConfigFlags` bit-field state (settable directly by callers of the low-level entry points)

`verbose` (bit 0), `debug` (bit 1), `optimize` (bit 2), `cache_enabled` (bit 3),
`log_level` (bits 4–6, values 0–7), `reserved` (bit 7). `init_config_from_env`
only ever produces `cache_enabled=1, log_level=3, reserved=0`, so **the
low-level entry points must be driven directly** to reach `cache_enabled=0`,
`log_level != 3`, and `reserved=1`.

### Input shapes

* `int` params: zero, ±1, small random, large random, `INT_MIN`, `INT_MAX`,
  `INT_MIN+1`, `INT_MAX-1`, powers of two, values with bit 30/31 set.
* `envy` guard shapes: `param3 == 0` vs `!= 0`; `param4 == 0` vs `> 0` vs `< 0`.
* result sign: `result >= 0` (no rollback) vs `result < 0` (rollback).
* `ConfigFlags` backing memory: pre-zeroed, pre-filled `0xFF`, random
  (tests that padding bytes 1–3 are handled identically).
* env-var string shapes: empty, decimal, signed, whitespace-padded,
  trailing junk, hex-looking, overflowing, 300 bytes long.

## Table

### `parse_env_numeric` (lowest-level entry point)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `parse_env_numeric` | var **unset**; `default_val` = 64 randomized ints incl. `INT_MIN`/`INT_MAX`/0 | [x] |
| C2 | `parse_env_numeric` | var set to plain decimal (64 randomized `i32` rendered as decimal, incl. negatives) | [x] |
| C3 | `parse_env_numeric` | var set to `""` (empty) | [x] |
| C4 | `parse_env_numeric` | var contains `','` at start / middle / end (randomized positions in random digit strings) | [x] |
| C5 | `parse_env_numeric` | var contains `';'` (no comma), randomized positions | [x] |
| C6 | `parse_env_numeric` | var contains **both** `','` and `';'` in both orders (comma check must win) | [x] |
| C7 | `parse_env_numeric` | var = whitespace-padded / explicitly-signed / trailing-junk numbers (`" 42"`, `"+7"`, `"-0"`, `"12abc"`, `"0x1f"`, `"007"`) | [x] |
| C8 | `parse_env_numeric` | var = `int`-overflowing decimal, both signs (`atoi` UB path, must match libc exactly) | [x] |
| C9 | `parse_env_numeric` | var = 300-byte value (no length limit in C); with and without `,` | [x] |
| C10 | `parse_env_numeric` | `env_name` = `""` (getenv of empty name) and a name that is never in the environment | [x] |

### `init_config_from_env`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C11 | `init_config_from_env` | full **3×3×3 = 27 cross-product** of `PROG_VERBOSE` × `PROG_DEBUG` × `PROG_OPTIMIZE` states, on **pre-zeroed** `ConfigFlags`; all 4 bytes compared | [x] |
| C12 | `init_config_from_env` | same 27 combos on `ConfigFlags` pre-filled `0xFF` (dirty byte 0 **and** dirty padding bytes 1–3) | [x] |
| C13 | `init_config_from_env` | same 27 combos on `ConfigFlags` pre-filled with randomized 4-byte patterns | [x] |
| C14 | `init_config_from_env` | `PROG_VERBOSE`/`PROG_DEBUG` set to randomized strings where `'1'` appears at a random index vs. strings drawn from `[02-9a-z]` only | [x] |

### `perform_operation`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C15 | `perform_operation` | `optimize=1`, `debug=0` → `val1+val2`; randomized `val1`,`val2` incl. overflow pairs | [x] |
| C16 | `perform_operation` | `optimize=1`, `debug=1` → same plus 2 `printf` lines (stdout compared byte-for-byte) | [x] |
| C17 | `perform_operation` | `optimize=0`, `debug=0`, `log_level` = **each of 0..7** → `val1*log_level + val2/2`; randomized vals per level | [x] |
| C18 | `perform_operation` | `optimize=0`, `debug=1`, `log_level` = each of 0..7 (stdout compared) | [x] |
| C19 | `perform_operation` | **all 256** `ConfigFlags` byte-0 patterns × randomized `val1`,`val2` (covers every optimize/debug/log_level/reserved/cache combination, incl. out-of-range-looking ones) | [x] |
| C20 | `perform_operation` | boundary vals: `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` × same set × `log_level` 0..7 (signed overflow + negative truncating division) | [x] |
| C21 | `perform_operation` | dirty padding: `ConfigFlags` bytes 1–3 = `0xFF` (must not affect the result) | [x] |

### `apply_bit_operations`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C22 | `apply_bit_operations` | `verbose=0, cache_enabled=0` → identity; randomized + boundary values | [x] |
| C23 | `apply_bit_operations` | `verbose=0, cache_enabled=1` → `\| 0x0F` | [x] |
| C24 | `apply_bit_operations` | `verbose=1, cache_enabled=0` → `<< 1` only (incl. bit-31 overflow, negative values) | [x] |
| C25 | `apply_bit_operations` | `verbose=1, cache_enabled=1` → `(<<1) \| 0x0F` | [x] |
| C26 | `apply_bit_operations` | **all 256** `ConfigFlags` byte-0 patterns × randomized values (other bits must be ignored) | [x] |
| C27 | `apply_bit_operations` | dirty padding bytes 1–3 = `0xFF` | [x] |

### `envy` (top-level composed pipeline)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C28 | `envy` | **no** env vars set (all defaults: verbose=0, debug=0, optimize=0, log_level=3, base=64, mult=10); randomized 4-tuples | [x] |
| C29 | `envy` | `PROG_OPTIMIZE` set → `optimize=1` add-path; randomized 4-tuples | [x] |
| C30 | `envy` | `PROG_VERBOSE=1` → 6 extra `printf` lines **and** the `<<1` in `apply_bit_operations`; randomized 4-tuples (stdout compared) | [x] |
| C31 | `envy` | `PROG_DEBUG=1` → 4 extra `printf` lines incl. the `%o` octal and second-colon message (stdout compared) | [x] |
| C32 | `envy` | `PROG_VERBOSE=1` **and** `PROG_DEBUG=1` **and** `PROG_OPTIMIZE=1` (all output paths + add-path at once) | [x] |
| C33 | `envy` | `PROG_VERBOSE`/`PROG_DEBUG` set but **without** `'1'` (flags stay 0 while vars are non-NULL) | [x] |
| C34 | `envy` | `PROG_BASE_OFFSET` set to randomized decimals (overrides 0100) | [x] |
| C35 | `envy` | `PROG_MULTIPLIER` set to randomized decimals (overrides 012; feeds `param3 * multiplier`) | [x] |
| C36 | `envy` | `PROG_BASE_OFFSET` **and** `PROG_MULTIPLIER` both set, randomized | [x] |
| C37 | `envy` | `PROG_BASE_OFFSET` contains `','` → stderr warning + default 64 (stderr compared) | [x] |
| C38 | `envy` | `PROG_MULTIPLIER` contains `';'` → stderr warning + default 10 (stderr compared) | [x] |
| C39 | `envy` | **both** offset and multiplier rejected → two stderr warnings, both defaults | [x] |
| C40 | `envy` | `PROG_BASE_OFFSET=""` / `PROG_MULTIPLIER=""` → 0 / 0 (not defaults) | [x] |
| C41 | `envy` | `param3 == 0` (skip multiplier term) × `param4 == 0` (skip shift term), all 4 combinations, randomized others | [x] |
| C42 | `envy` | `param4 < 0` → arithmetic right shift; randomized negative `param4` | [x] |
| C43 | `envy` | params forced so the final `result < 0` → rollback returns `param1`; verbose on and off (extra "Restored state" line) | [x] |
| C44 | `envy` | params forced so `result == 0` and `result == -1` exactly (rollback boundary) | [x] |
| C45 | `envy` | all four params over the boundary set `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` (overflow in `*`, `+`, `<<`) | [x] |
| C46 | `envy` | `PROG_BASE_OFFSET`/`PROG_MULTIPLIER` = `INT_MIN`/`INT_MAX` (drives wrapping in `result += base_offset` and `param3*multiplier`) | [x] |
| C47 | `envy` | **full randomized cross-product sweep**: verbose(3) × debug(3) × optimize(3) × base_offset(5) × multiplier(5) = 675 env configurations × randomized params, comparing return + stdout + stderr | [x] |

### Cross-cutting

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C48 | `init_config_from_env` → `perform_operation` → `apply_bit_operations` | hand-composed pipeline mirroring `envy`'s call order, driven through the **low-level** exports only, under the 27 env combos × randomized params (catches bugs invisible to per-function tests) | [x] |
| C49 | all five | symbol-parity + `.so` load smoke check (`nm -D` diff must be empty) | [x] |
| C50 | *n/a* | no binary/driver executable is produced by `c_src/CMakeLists.txt` (`add_library(... SHARED ...)` only) and `translation/Cargo.toml` declares only a `cdylib`, so the "compare binary stdout" gate is **not applicable**; stdout is instead compared per-call via fd redirection | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `cargo test --no-default-features` is
nevertheless run in `run_all.sh` to prove the crate builds and passes with no
features enabled (the feature-combination gate is satisfied by the single
existing combination).

## Result

All 50 rows pass. See `tests/configs.rs` (50 tests, 50 passed) and
`VERIFICATION.md` for the full report, including the two harness bugs and the
one real translation bug that this phase and Phase C uncovered.

Note on C50: the "compare the driver binary's stdout" gate is genuinely N/A
(neither build produces an executable), so stdout is compared **per call**
instead — fd 1 and fd 2 are redirected around every single differential
invocation, which is strictly finer-grained than comparing one binary's output.
