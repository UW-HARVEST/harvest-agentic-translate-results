# CONFIGS.md — Configuration surface table (Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**Runtime options** — the *only* way a consumer configures this library is the
process environment (there is no options struct, no setter, no `#ifdef`):

| axis | read at | distinct states the C distinguishes |
|------|---------|--------------------------------------|
| `PROG_VERBOSE`     | `lib.c:70,74`  | unset ⇒ 0 · set without `'1'` ⇒ 0 · set containing `'1'` ⇒ 1 |
| `PROG_DEBUG`       | `lib.c:71,75`  | unset ⇒ 0 · set without `'1'` ⇒ 0 · set containing `'1'` ⇒ 1 |
| `PROG_OPTIMIZE`    | `lib.c:72,76`  | unset ⇒ 0 · set to *anything*, incl. `""` ⇒ 1 (presence-only) |
| `PROG_BASE_OFFSET` | `lib.c:123`    | unset ⇒ `0100`=64 · valid decimal · contains `,` ⇒ default+warn · contains `;` ⇒ default+warn · non-numeric ⇒ `atoi`=0 · overflowing |
| `PROG_MULTIPLIER`  | `lib.c:124`    | unset ⇒ `012`=10 · same five other states |

**Hard-wired state** set by `init_config_from_env`: `cache_enabled = 1`,
`log_level = 03`, `reserved = 0`. Because the *low-level* entry points
(`perform_operation`, `apply_bit_operations`) take the `ConfigFlags` unit
**directly**, a real consumer can present any of the 2⁸ bit patterns —
including `cache_enabled = 0` and `log_level ∈ {0..7}`, which the env-driven
path can never produce. Those are separate axes and are exercised directly.

**Input shapes**: `param3 == 0` vs `≠ 0` (guard `lib.c:145`); `param4 == 0` vs
`≠ 0` (guard `lib.c:149`); sign of `param4` (arithmetic vs logical shift);
sign of the accumulated `result` (restore branch `lib.c:171`); `0`, `±1`,
`INT_MIN`, `INT_MAX`, `0x3FFFFFFF`, `0x40000000` boundaries for every `int`
parameter; garbage in bits 8..31 of the flags allocation unit.

**All five public entry points** are driven directly through the `.so` exports
(not just the `envy` one-shot wrapper): `parse_env_numeric`,
`init_config_from_env`, `perform_operation`, `apply_bit_operations`, `envy`.

**Comparison performed for every row**: the `int` return value **and** the
byte-exact `stdout` and `stderr` produced by the call (captured with
`dup2` + `fflush`), **and** — for `init_config_from_env` — all 4 bytes of the
written `ConfigFlags` allocation unit. Each row is driven with many
randomized inputs from a fixed-seed PRNG (seed `0x5EED_1234`), not a single
hand-picked value.

## Rows

### `parse_env_numeric(const char*, int)` — low-level entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `parse_env_numeric` | variable **unset** × randomized `default_val` (incl. `0`, `±1`, `INT_MIN`, `INT_MAX`) | [x] |
| 2 | `parse_env_numeric` | value = randomized valid decimal, **positive** × randomized `default_val` | [x] |
| 3 | `parse_env_numeric` | value = randomized valid decimal, **negative** (`-…`) | [x] |
| 4 | `parse_env_numeric` | value = `"+N"` (explicit plus sign) | [x] |
| 5 | `parse_env_numeric` | value with **leading whitespace** (`"  N"`, `"\tN"`) — `atoi` skips it | [x] |
| 6 | `parse_env_numeric` | value with **trailing garbage** (`"Nabc"`, `"N N"`) — `atoi` stops early | [x] |
| 7 | `parse_env_numeric` | value **leading zeros** (`"0100"`) — `atoi` is **decimal**, not octal ⇒ 100 | [x] |
| 8 | `parse_env_numeric` | value = `"0x1F"` — `atoi` ⇒ 0 | [x] |
| 9 | `parse_env_numeric` | value `INT_MAX` / `INT_MIN` exactly, and one past each | [x] |
| 10 | `parse_env_numeric` | value contains `','` (comma) — warning path × randomized `default_val`, randomized env **name** (the name is `%s`-printed) | [x] |
| 11 | `parse_env_numeric` | value contains `';'` — semicolon warning path × randomized `default_val`/name | [x] |
| 12 | `parse_env_numeric` | value contains **both** separators, in both orders | [x] |
| 13 | `parse_env_numeric` | value = `""` (empty) ⇒ `atoi("")` = 0 | [x] |
| 14 | `parse_env_numeric` | value = very long string (1 KiB of digits) — overflow + no truncation anywhere | [x] |

### `init_config_from_env(struct ConfigFlags*)` — low-level entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 15 | `init_config_from_env` | full **cross product** of `PROG_VERBOSE` ∈ {unset, no-`1`, with-`1`} × `PROG_DEBUG` ∈ {unset, no-`1`, with-`1`} × `PROG_OPTIMIZE` ∈ {unset, `""`, `"0"`} = **27 combinations**, each compared over all 4 bytes of the unit | [x] |
| 16 | `init_config_from_env` | destination buffer pre-filled with `0x00`, `0xFF`, `0xAA`, and randomized garbage — checks which bits the C actually preserves in bits 8..31 | [x] |
| 17 | `init_config_from_env` | `'1'` in first / middle / last position of the value, and repeated (`"11"`), and as part of a longer token (`"v1.0"`, `"310"`) | [x] |
| 18 | `init_config_from_env` | called **twice in a row** on the same buffer (idempotence / no accumulated state) | [x] |

### `perform_operation(int, int, struct ConfigFlags*)` — low-level entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 19 | `perform_operation` | `optimize=1, debug=0` × randomized `val1`,`val2` (uniform + boundary set) ⇒ addition path, no output | [x] |
| 20 | `perform_operation` | `optimize=1, debug=1` ⇒ addition path **+ two debug lines on stdout** (`%o` of `0755`) | [x] |
| 21 | `perform_operation` | `optimize=0, debug=0`, `log_level = 0` ⇒ `0*val1 + val2/2` | [x] |
| 22 | `perform_operation` | `optimize=0, debug=0`, `log_level = 1` | [x] |
| 23 | `perform_operation` | `optimize=0, debug=0`, `log_level = 2` | [x] |
| 24 | `perform_operation` | `optimize=0, debug=0`, `log_level = 3` (the env-driven value) | [x] |
| 25 | `perform_operation` | `optimize=0, debug=0`, `log_level = 4` | [x] |
| 26 | `perform_operation` | `optimize=0, debug=0`, `log_level = 5` | [x] |
| 27 | `perform_operation` | `optimize=0, debug=0`, `log_level = 6` | [x] |
| 28 | `perform_operation` | `optimize=0, debug=0`, `log_level = 7` (max of the 3-bit field) | [x] |
| 29 | `perform_operation` | `optimize=0, debug=1` × `log_level` 0..7 ⇒ multiply path **+ debug output** | [x] |
| 30 | `perform_operation` | all 2⁸ flag byte patterns (verbose/cache_enabled/reserved must be **irrelevant** here) × randomized values | [x] |
| 31 | `perform_operation` | flags unit with garbage in bits 8..31 (`0xDEADBE__`) — must be ignored | [x] |

### `apply_bit_operations(int, struct ConfigFlags*)` — low-level entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 32 | `apply_bit_operations` | `verbose=0, cache_enabled=0` ⇒ identity × randomized + boundary values | [x] |
| 33 | `apply_bit_operations` | `verbose=0, cache_enabled=1` ⇒ `value | 0x0F` | [x] |
| 34 | `apply_bit_operations` | `verbose=1, cache_enabled=0` ⇒ `value << 1` (incl. overflow / negative) | [x] |
| 35 | `apply_bit_operations` | `verbose=1, cache_enabled=1` ⇒ `(value << 1) | 0x0F` | [x] |
| 36 | `apply_bit_operations` | all 2⁸ flag byte patterns (debug/optimize/log_level must be irrelevant) × randomized values | [x] |

### `envy(int, int, int, int)` — public header entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 37 | `envy` | **all 8** verbose × debug × optimize env combinations × randomized `param1..param4`, defaults for offset/multiplier — return value **and** stdout compared | [x] |
| 38 | `envy` | `param3 == 0` (skip multiplier term) × the 8 env combinations | [x] |
| 39 | `envy` | `param4 == 0` (skip shift term) × the 8 env combinations | [x] |
| 40 | `envy` | `param3 == 0 && param4 == 0` × the 8 env combinations | [x] |
| 41 | `envy` | `param4 < 0` (arithmetic shift) and `param4 ∈ {1,2,3,-1,-2,-3}` (shift-to-zero boundary) | [x] |
| 42 | `envy` | inputs forcing `result < 0` ⇒ restore branch, with `param1 > 0`, `param1 == 0`, `param1 < 0` | [x] |
| 43 | `envy` | `PROG_BASE_OFFSET` = randomized valid decimal (incl. negative, `INT_MIN`, `INT_MAX`) × the 8 env combinations | [x] |
| 44 | `envy` | `PROG_MULTIPLIER` = randomized valid decimal (incl. `0`, negative, `INT_MIN`, `INT_MAX`) × the 8 env combinations | [x] |
| 45 | `envy` | both offset **and** multiplier customized, randomized together | [x] |
| 46 | `envy` | `PROG_BASE_OFFSET` **rejected** by comma ⇒ default 64 **+ stderr warning** (interleaving with stdout verbose lines checked) | [x] |
| 47 | `envy` | `PROG_MULTIPLIER` **rejected** by semicolon ⇒ default 10 + stderr warning | [x] |
| 48 | `envy` | both rejected, one by comma and one by semicolon (both warnings, correct `%s` names, correct order) | [x] |
| 49 | `envy` | offset/multiplier non-numeric ⇒ `atoi` = 0 (multiplier 0 ⇒ `param3` term vanishes even though `param3 ≠ 0`) | [x] |
| 50 | `envy` | offset/multiplier overflowing (`"99999999999999"`) | [x] |
| 51 | `envy` | extreme params: every 4-tuple from `{INT_MIN, -1, 0, 1, INT_MAX, 0x3FFFFFFF, 0x40000000}` (7⁴ = 2401 tuples) under the default env | [x] |
| 52 | `envy` | same extreme tuples with `verbose=1` (shift path) and with `optimize=1` (addition path) | [x] |
| 53 | `envy` | pipeline composition check: `envy` result vs. manual `init_config_from_env` → `perform_operation` → `apply_bit_operations` chain on **both** libraries | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** — therefore the
only build configuration is the default one, and
`cargo test --no-default-features` is equivalent to `cargo test`. Both are run
by `run_all.sh`, together with the release-profile build (`panic = "abort"`),
so every code path is verified under every existing configuration.

## How the rows were verified

`./run_all.sh` (in this directory) builds the C `.so` and the Rust `cdylib`,
diffs `nm -D`, and runs the whole suite for every feature combination × both
Rust build profiles:

| suite | file | rows covered |
|-------|------|--------------|
| Phase B, low-level entry points | `tests/phase_b_low_level.rs` | 1–36 (+ hostile env-name variants of 10/11) |
| Phase B, `envy` + composed pipeline | `tests/phase_b_envy.rs` | 37–53 |
| Phase C, error surface | `tests/phase_c_errors.rs` | every `ERRORS.md` row |
| Phase D, symbol parity | `tests/symbols.rs` | `SYMBOLS.md` |

Every call is compared on three axes at once: the `int` return value, the exact
bytes written to `stdout` and to `stderr`, and (where a pointer is passed) the
resulting memory. A `sanity_00_capture_observes_library_output` test asserts the
capture machinery really sees library output, so no row can pass vacuously.

**Result: 84 tests, all passing, in all 4 build configurations** — see the
`ALL CONFIGURATIONS PASSED` summary from `run_all.sh`.
