# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branches the C actually takes in
`c_src/src/lib.c`. Every row is exercised through the `.so` exports of **both**
libraries with many randomized inputs (fixed seed, see
`tests/differential.rs::SEED`), and every row compares **three** observables:
the `int` return value, the exact bytes written to `stdout`, and the exact bytes
written to `stderr`.

## The axes the C branches on

**A. Runtime options — there is no options struct; the configuration is the
process environment plus the caller-supplied `struct ConfigFlags` byte.**

| axis | source | states the C distinguishes |
|------|--------|-----------------------------|
| `PROG_VERBOSE`  | `lib.c:70,74` | unset · set **containing** `'1'` · set **not** containing `'1'` (incl. `""`) |
| `PROG_DEBUG`    | `lib.c:71,75` | unset · set containing `'1'` · set not containing `'1'` |
| `PROG_OPTIMIZE` | `lib.c:72,76` | unset · set to **anything at all, including `""`** (only `!= NULL` is tested) |
| `PROG_BASE_OFFSET` | `lib.c:123` → `parse_env_numeric` | unset (⇒ `0100` = 64) · numeric · negative · non-numeric · overflowing · contains `,` · contains `;` |
| `PROG_MULTIPLIER`  | `lib.c:124` → `parse_env_numeric` | unset (⇒ `012` = 10) · numeric · negative · non-numeric · overflowing · contains `,` · contains `;` |
| `flags.verbose`  | `lib.c:104` (`apply_bit_operations`) | 0 ⇒ no shift · 1 ⇒ `adjusted <<= 1` |
| `flags.debug`    | `lib.c:93` (`perform_operation`) | 0 ⇒ silent · 1 ⇒ two `printf` lines |
| `flags.optimize` | `lib.c:87` (`perform_operation`) | 0 ⇒ `val1*log_level + val2/2` · 1 ⇒ `val1+val2` |
| `flags.cache_enabled` | `lib.c:108` | 0 ⇒ no mask · 1 ⇒ `adjusted \|= 0x0F` |
| `flags.log_level` | `lib.c:90` | 3-bit field, all 8 values `0..7` reachable when the caller supplies the struct (`init_config_from_env` only ever writes `03`) |
| `flags.reserved` | written at `lib.c:79`, never read | must still round-trip |
| `flags` padding bytes 1..3 | never touched by the C | must be preserved by writes, ignored by reads |

**B. Input shapes the C special-cases**

| axis | source | shapes |
|------|--------|--------|
| `param3` | `lib.c:145` | `== 0` (block skipped) vs `!= 0` |
| `param4` | `lib.c:149` | `== 0` (block skipped) vs `!= 0`; negative ⇒ arithmetic `>>` |
| final `result` sign | `lib.c:171` | `>= 0` (returned as-is) vs `< 0` (roll-back to `param1`) |
| integer magnitude | every arithmetic op | small · `INT_MAX`/`INT_MIN` · values chosen to overflow each of the 5 overflow-capable expressions |
| `env_name` string | `lib.c:48` | present · absent · empty name · long name |
| `default_val` | `lib.c:51,57,63` | `0` · negative · `INT_MIN` · `INT_MAX` |

## Rows (pruned cross-product — only combinations the C treats differently)

### `parse_env_numeric` (lowest level, called directly via the `.so`)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `parse_env_numeric` | name absent from environment × `default_val` ∈ {0, ±small, `INT_MIN`, `INT_MAX`} (randomized) | [x] |
| 2 | `parse_env_numeric` | value = randomized pure decimal digits, no sign, no `,`/`;` | [x] |
| 3 | `parse_env_numeric` | value = randomized signed decimal (`+`/`-` prefix) | [x] |
| 4 | `parse_env_numeric` | value = randomized decimal with leading whitespace / trailing garbage (`atoi` prefix parse) | [x] |
| 5 | `parse_env_numeric` | value = randomized text containing `,` at a random position ⇒ `default_val` + stderr warning | [x] |
| 6 | `parse_env_numeric` | value = randomized text containing `;` (no `,`) ⇒ `default_val` + stderr warning | [x] |
| 7 | `parse_env_numeric` | value = randomized text containing both `,` and `;` in random order ⇒ comma branch only | [x] |
| 8 | `parse_env_numeric` | value = `""` (present but empty) ⇒ `atoi("") == 0` | [x] |
| 9 | `parse_env_numeric` | value = randomized octal-looking (`0100`, `012`, `0755`) — `atoi` is **decimal**, so `0100` is 100, not 64 | [x] |
| 10 | `parse_env_numeric` | value = randomized `INT_MAX`/`INT_MIN`-adjacent and beyond-64-bit digit strings | [x] |
| 11 | `parse_env_numeric` | randomized variable *names* (incl. empty name, 200-char name) — the name is echoed by the warning `%s` | [x] |

### `init_config_from_env` (lowest level, direct)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 12 | `init_config_from_env` | all 3×3×2 = 18 combinations of `PROG_VERBOSE` ∈ {unset, with `'1'`, without `'1'`} × `PROG_DEBUG` ∈ {same} × `PROG_OPTIMIZE` ∈ {unset, set} — compare all 4 struct bytes | [x] |
| 13 | `init_config_from_env` | same 18 combinations, but the 4-byte struct pre-filled with a randomized garbage pattern ⇒ verifies bit-field RMW and padding preservation | [x] |
| 14 | `init_config_from_env` | `PROG_OPTIMIZE=""` (empty ⇒ still enables) and `PROG_VERBOSE`/`PROG_DEBUG` with `'1'` in random positions of a random string | [x] |
| 15 | `init_config_from_env` | called repeatedly on the *same* struct (idempotence) and on a struct whose bits are already set | [x] |

### `perform_operation` (lowest level, direct)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 16 | `perform_operation` | `optimize = 1` × randomized `val1`,`val2` incl. `INT_MAX`/`INT_MIN` ⇒ wrapping `val1+val2` | [x] |
| 17 | `perform_operation` | `optimize = 0` × `log_level` = each of `0..7` × randomized `val1`,`val2` ⇒ `val1*log_level + val2/2` | [x] |
| 18 | `perform_operation` | `optimize = 0` × `val2` ∈ {odd negative, `INT_MIN`, ±1} ⇒ truncate-toward-zero division | [x] |
| 19 | `perform_operation` | `debug = 1` × `optimize` ∈ {0,1} ⇒ the two debug `printf` lines (`%o` of `0755` and `%d` of result) must match byte-for-byte, incl. for negative results | [x] |
| 20 | `perform_operation` | **all 256 byte-0 bit patterns** × randomized `val1`,`val2` (covers `reserved=1`, `log_level=4..7`, every flag cross-product) | [x] |
| 21 | `perform_operation` | struct with randomized garbage in bytes 1..3 ⇒ must not affect the result | [x] |

### `apply_bit_operations` (lowest level, direct)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 22 | `apply_bit_operations` | `verbose=0, cache_enabled=0` ⇒ identity × randomized `value` | [x] |
| 23 | `apply_bit_operations` | `verbose=0, cache_enabled=1` ⇒ `value \| 0x0F` × randomized `value` incl. negatives | [x] |
| 24 | `apply_bit_operations` | `verbose=1, cache_enabled=0` ⇒ `value << 1`, including values whose shift overflows `int` (`INT_MAX`, `INT_MIN`, `0x40000000`) | [x] |
| 25 | `apply_bit_operations` | `verbose=1, cache_enabled=1` ⇒ shift **then** mask (order matters) × randomized `value` | [x] |
| 26 | `apply_bit_operations` | all 256 byte-0 bit patterns × randomized `value` | [x] |
| 27 | `apply_bit_operations` | struct with randomized garbage in bytes 1..3 | [x] |

### `envy` (top-level composed pipeline — the full end-to-end operation)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 28 | `envy` | environment completely clean (all 5 vars unset) × randomized `param1..4` ⇒ non-optimize path, `log_level=3`, defaults `64`/`10`, silent | [x] |
| 29 | `envy` | `PROG_OPTIMIZE` set × randomized params ⇒ `val1+val2` path, silent | [x] |
| 30 | `envy` | `PROG_VERBOSE=1` (only) ⇒ 5 stdout lines incl. `Found colon at position: 6` and the `Configuration - …` line; `apply_bit_operations` also shifts | [x] |
| 31 | `envy` | `PROG_DEBUG=1` (only) ⇒ 4 debug stdout lines incl. `Debug: Result string format validated` | [x] |
| 32 | `envy` | `PROG_VERBOSE=1` **and** `PROG_DEBUG=1` ⇒ full 9-line interleaving in the exact C order | [x] |
| 33 | `envy` | all 8 combinations of {`PROG_VERBOSE`,`PROG_DEBUG`,`PROG_OPTIMIZE`} × randomized `param1..4` | [x] |
| 34 | `envy` | `PROG_BASE_OFFSET` numeric/negative/huge × `PROG_MULTIPLIER` numeric/negative/huge (randomized pairs) | [x] |
| 35 | `envy` | `PROG_BASE_OFFSET` with `,` and/or `;` (⇒ falls back to `0100`=64 + stderr warning) × verbose on/off | [x] |
| 36 | `envy` | `PROG_MULTIPLIER` with `,` and/or `;` (⇒ falls back to `012`=10 + stderr warning) × `param3` ∈ {0, non-zero} | [x] |
| 37 | `envy` | `param3 == 0` (block skipped) × `param4 == 0` (block skipped) — all 4 zero/non-zero combinations × randomized values | [x] |
| 38 | `envy` | `param4 < 0` ⇒ arithmetic `>> 2`; randomized negatives incl. `INT_MIN`, `-1`, `-3` | [x] |
| 39 | `envy` | inputs engineered so the pre-`base_offset` result is negative but the post-`base_offset` result is ≥ 0 (and vice versa) ⇒ exercises the `result < 0` roll-back boundary from both sides | [x] |
| 40 | `envy` | roll-back path taken (`result < 0`) × `param1` ∈ {positive, negative, 0, `INT_MIN`, `INT_MAX`} × verbose on/off (`Restored state from backup` line) | [x] |
| 41 | `envy` | `param1..4` all at `INT_MAX`/`INT_MIN` extremes (every overflow-capable expression made to wrap) × all 8 env-flag combinations | [x] |
| 42 | `envy` | full randomized fuzz: 5 env vars randomized (present/absent, random values incl. `,`/`;`/`'1'`) × 4 randomized `int` params — 4000 iterations | [x] |
| 43 | `envy` | called twice in a row with the same environment (no hidden per-call state; `state`/`state_backup`/`buffer` are locals) | [x] |
| 44 | `envy` + `parse_env_numeric` + `init_config_from_env` + `perform_operation` + `apply_bit_operations` | **composed pipeline check**: reproduce `envy` by driving the four low-level exports by hand in the same order and assert the hand-composed value equals `envy`'s, for randomized env × params — catches divergence that per-function tests hide | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table ⇒ the single configuration
`(default)` is also `--no-default-features` and `--all-features`.
`tests/feature_matrix.sh` extracts the feature list from `Cargo.toml`, asserts it
is empty, and re-runs the whole suite under `--no-default-features`,
`--all-features` and `--release` so that the "every feature combination" gate is
verified mechanically.

## Status

**44/44 rows pass across randomized inputs.** Row *N* is covered by the test
named `cfg_NN_…` in `tests/differential.rs`. The row↔test mapping is checked
mechanically:

```
CONFIGS.md rows: 44  ->  cfg_ tests in tests/differential.rs: 44   MATCH
```

Every row runs many randomized inputs (hundreds to thousands per row) from the
fixed-seed splitmix64 generator in `tests/common/mod.rs`, biased towards the
values the C actually branches on — `0`, `±1`, `INT_MIN`/`INT_MAX`, the powers of
two that make each expression overflow, and env-var strings drawn from an
alphabet rich in `,` `;` `1` and digits. Row 20/26 sweep **all 256**
`struct ConfigFlags` byte-0 bit patterns; row 13 crosses those with all 18
environment states. Row 44 additionally rebuilds `envy` out of the four
lower-level exports and requires the hand-composed pipeline to agree with the
one-shot entry point on both libraries — the composed-pipeline check that
per-function tests cannot see.
