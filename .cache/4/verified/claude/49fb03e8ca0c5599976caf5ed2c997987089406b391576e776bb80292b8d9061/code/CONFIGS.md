# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/lib.c`

## Axes the C code actually branches on

**Runtime options.** The library has no options struct; every option is an
environment variable read on each call (`getenv`), so the *whole* option surface
is these five variables:

| variable | read in | states the C distinguishes |
|----------|---------|-----------------------------|
| `PROG_VERBOSE`   | `init_config_from_env` | `unset` / `set without '1'` / `set containing '1'` → `flags.verbose` (bit0). Controls 6 `printf`s in `envy` **and** the `<< 1` in `apply_bit_operations`. |
| `PROG_DEBUG`     | `init_config_from_env` | `unset` / `set without '1'` / `set containing '1'` → `flags.debug` (bit1). Controls 5 `printf`s. |
| `PROG_OPTIMIZE`  | `init_config_from_env` | `unset` / `set (value ignored)` → `flags.optimize` (bit2). Selects the arithmetic branch in `perform_operation`. |
| `PROG_BASE_OFFSET` | `envy` → `parse_env_numeric` | `unset`(=`0100`=64) / decimal / negative / `atoi`-garbage(=0) / contains `,` (=default) / contains `;` (=default) / overflow |
| `PROG_MULTIPLIER`  | `envy` → `parse_env_numeric` | same seven shapes; feeds `param3 * multiplier` |

**Derived flag state** (what an *external* caller may pass to the exported
low-level functions, which never validate it): the 4-byte `struct ConfigFlags`
bit-field unit — `bit0 verbose`, `bit1 debug`, `bit2 optimize`,
`bit3 cache_enabled`, `bits4-6 log_level (0..7)`, `bit7 reserved`,
`bits8-31 padding`. `init_config_from_env` only ever produces
`log_level == 3`, so `log_level ∈ {0,1,2,4,5,6,7}` is only reachable through the
low-level entry points — which is exactly why they are driven directly.

**Input shapes.** `param1` (base/backup value), `param2`, `param3`
(`== 0` vs `!= 0` branch), `param4` (`== 0` vs `!= 0` branch), each in
{`0`, `+1`, `-1`, small random, large random, `INT_MAX`, `INT_MIN`,
`0x40000000`, `-0x40000000`}; plus the two global outcomes
`final result >= 0` vs `final result < 0` (backup-restore path).

**Public entry points (all five exported symbols are driven directly):**
`parse_env_numeric`, `init_config_from_env`, `perform_operation`,
`apply_bit_operations`, `envy`.

Every row below is compared C-vs-Rust on **return value + the exact bytes
written to stdout + the exact bytes written to stderr** (and, for the functions
that take a `struct ConfigFlags*`, on the resulting bit pattern of the caller's
buffer, so a stray write would be caught too), over many randomised inputs
(xorshift64*, fixed seed `SEED = 0x2026_0819_C0FF_EE01`).

Both libraries are loaded with `libloading` and every call goes through
`dlsym` — the Rust functions are never called directly, so the `#[no_mangle]`
export wrappers are part of what is being tested.

## Rows

`[x]` = verified by a passing differential test.  The "test" column is the row
label printed by the test's check-off table (`cargo test -- --nocapture`).

| #  | entry point(s) | configuration (options set + input shape) | test (binary / printed row) | [x] |
|----|----------------|--------------------------------------------|------------------------------|-----|
| 1  | `parse_env_numeric` | variable **absent**; `default_val` ∈ {0, ±1, ±random, INT_MAX, INT_MIN} | `phase_b_lowlevel` row 1 | [x] |
| 2  | `parse_env_numeric` | value = plain non-negative decimal (`"0"`, `"7"`, … + 300 random) | `phase_b_lowlevel` row 2 | [x] |
| 3  | `parse_env_numeric` | value = negative decimal (`"-1"`, `"-2147483648"`, + 300 random) | `phase_b_lowlevel` row 3 | [x] |
| 4  | `parse_env_numeric` | value = leading-whitespace / explicit-sign / leading-zero forms (`"  42"`, `"\t-9"`, `"+7"`, `"007"`, `"0100"`) — `atoi` is decimal-only, no octal | `phase_b_lowlevel` row 4 | [x] |
| 5  | `parse_env_numeric` | value = empty string; and `env_name` = `""` / a never-set name | `phase_b_lowlevel` row 5 | [x] |
| 6  | `parse_env_numeric` | value contains `','` (leading / middle / trailing, + random) ⇒ stderr warning + default | `phase_b_lowlevel` row 6 | [x] |
| 7  | `parse_env_numeric` | value contains `';'` only ⇒ stderr warning + default | `phase_b_lowlevel` row 7 | [x] |
| 8  | `parse_env_numeric` | value contains both `','` and `';'` (order of the two checks) | `phase_b_lowlevel` row 8 | [x] |
| 9  | `parse_env_numeric` | value = `atoi` garbage / trailing garbage / overflow (`"abc"`, `"12abc"`, `"9999999999"`, 38-digit) | `phase_b_lowlevel` row 9 | [x] |
| 10 | `init_config_from_env` | all `PROG_VERBOSE` × `PROG_DEBUG` × `PROG_OPTIMIZE` states (6×5×5 values covering the 3×3×2 classes), destination pre-filled with `0x00000000` | `phase_b_lowlevel` row 10 | [x] |
| 11 | `init_config_from_env` | same states, destination pre-filled with `0xFFFFFFFF`, `0xDEADBEEF`, … and random garbage (padding-bit preservation) | `phase_b_lowlevel` row 11 | [x] |
| 12 | `perform_operation` | `optimize = 0`, `debug = 0`, `log_level` = 0..7 (all 8) × `cache` × `verbose`; 11×11 boundary grid + randomised `val1`,`val2` | `phase_b_lowlevel` row 12 | [x] |
| 13 | `perform_operation` | `optimize = 0`, `debug = 1` (stdout: `operation_mode` in octal + result line), `log_level` 0..7 | `phase_b_lowlevel` row 13 | [x] |
| 14 | `perform_operation` | `optimize = 1`, `debug = 0` — `log_level` must be ignored | `phase_b_lowlevel` row 14 | [x] |
| 15 | `perform_operation` | `optimize = 1`, `debug = 1` | `phase_b_lowlevel` row 15 | [x] |
| 16 | `perform_operation` | flags = every one of the 256 low-byte bit patterns (incl. `reserved` and log_level values `init_config_from_env` never produces) × random values | `phase_b_lowlevel` row 16 | [x] |
| 17 | `perform_operation` | flags with garbage padding bits (`0xDEADBE00`, `0xFFFFFF00`, `0x12345600` OR'ed with each low byte) — padding must not affect the result | `phase_b_lowlevel` row 17 | [x] |
| 18 | `apply_bit_operations` | `verbose = 0`, `cache_enabled = 0` (identity) | `phase_b_lowlevel` row 18 | [x] |
| 19 | `apply_bit_operations` | `verbose = 0`, `cache_enabled = 1` (`| 0x0F`) | `phase_b_lowlevel` row 19 | [x] |
| 20 | `apply_bit_operations` | `verbose = 1`, `cache_enabled = 0` (`<< 1`, incl. negative and sign-bit-overflow values) | `phase_b_lowlevel` row 20 | [x] |
| 21 | `apply_bit_operations` | `verbose = 1`, `cache_enabled = 1` (`<< 1` then `| 0x0F`) | `phase_b_lowlevel` row 21 | [x] |
| 22 | `apply_bit_operations` | flags = all 256 low-byte patterns × garbage padding × randomised `value` | `phase_b_lowlevel` row 22 | [x] |
| 23 | `envy` | all `PROG_VERBOSE` × `PROG_DEBUG` × `PROG_OPTIMIZE` states, both numeric variables **unset** (defaults 64 / 10); randomised params | `phase_b_envy` row 23 | [x] |
| 24 | `envy` | `param3 == 0` **and** `param4 == 0` (both optional terms skipped) × the flag states | `phase_b_envy` row 24 | [x] |
| 25 | `envy` | exactly one of `param3` / `param4` non-zero (one term each, incl. `INT_MIN`/`INT_MAX`) × flag states | `phase_b_envy` row 25 | [x] |
| 26 | `envy` | `param3 != 0` **and** `param4 != 0` (both terms) × flag states × random params | `phase_b_envy` row 26 | [x] |
| 27 | `envy` | `PROG_BASE_OFFSET` set to each of the 16 value shapes (absent, empty, valid, 0, leading-zero, negative, INT_MIN/MAX, whitespace, `+`, garbage, prefix, comma, semicolon) × flag states | `phase_b_envy` row 27 | [x] |
| 28 | `envy` | `PROG_MULTIPLIER` set to each of the 16 value shapes × `param3 != 0` (multiplier actually used) | `phase_b_envy` row 28 | [x] |
| 29 | `envy` | both `PROG_BASE_OFFSET` and `PROG_MULTIPLIER` set — full 16×16 cross product of shapes (two stderr warnings in one call) | `phase_b_envy` row 29 | [x] |
| 30 | `envy` | configurations forced onto the `result < 0` backup-restore path (large negative base offset / negative params), `param1` positive, zero and negative | `phase_b_envy` row 30 | [x] |
| 31 | `envy` | boundary params: full `7^4` cross-product of `{0, 1, -1, INT_MAX, INT_MIN, 0x40000000, -0x40000000}` with `optimize` on and off | `phase_b_envy` row 31 | [x] |
| 32 | `envy` | full-random fuzz: random shapes for all five variables × random params, 4000 iterations | `phase_b_envy` row 32 | [x] |
| 33 | pipeline | composed low-level pipeline `init_config_from_env` → `parse_env_numeric` ×2 → `perform_operation` → `apply_bit_operations` reproducing `envy` by hand, over the env/flag matrix; run as an all-C chain vs an all-Rust chain **and** as two cross-chains (C flags+parse ↔ Rust arithmetic) | `phase_b_lowlevel` row 33 | [x] |

## Result

```
CONFIGS.md (low-level) : 23 rows, 29 622 differential comparisons, 0 failures
CONFIGS.md (envy)      : 10 rows, 30 062 differential comparisons, 0 failures
```

Both under the dev-profile cdylib **and** the release-profile cdylib
(`panic = "abort"`, optimised), and additionally against a `-O3` build of the C
library (`CMAKE_BUILD_TYPE=Release`) as a cross-check of the UB-dependent
arithmetic (signed overflow, `<<` into the sign bit, `>>` of negatives).

The suite is validated by `./mutation_check.sh`: 19 seeded mutations of
`src/lib.rs` (wrong `log_level`, wrong octal defaults, wrong bit masks, logical
instead of arithmetic shift, `/2` → `>>1`, skipped `,` check, whole-word instead
of read-modify-write flag store, `<=` instead of `<`, changed message text,
shortened result string, reference-based instead of raw-pointer flag load, …)
— **all 19 are detected** by these tests.
