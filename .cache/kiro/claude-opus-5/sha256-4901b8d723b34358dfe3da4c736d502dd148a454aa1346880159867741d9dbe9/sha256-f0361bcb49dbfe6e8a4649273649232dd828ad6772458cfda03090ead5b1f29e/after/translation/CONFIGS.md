# CONFIGS.md — Phase A configuration surface (valid inputs)

Derived mechanically from the branches in `c_src/src/lib.c`. The public surface
is the 5 exported symbols in `SYMBOLS.md`; `include/lib.h` declares only `envy`,
but the other four are dynamically exported and are the **low-level entry
points**, so they are driven directly here as well as through `envy`.

## Axes the C actually branches on

**A. Runtime options — all configuration enters through `getenv`, there are no
setter functions.**

| axis | read at | states the C distinguishes | state it toggles |
|------|---------|----------------------------|------------------|
| `PROG_VERBOSE` | `init_config_from_env` | unset / set-and-contains-`'1'` / set-and-lacks-`'1'` (incl. `""`) | `flags.verbose` (bit 0) → `adjusted << 1` in `apply_bit_operations`, plus 5 `printf` sites |
| `PROG_DEBUG` | `init_config_from_env` | unset / set-and-contains-`'1'` / set-and-lacks-`'1'` | `flags.debug` (bit 1) → 2 `printf` in `perform_operation`, 3 `printf` in `envy` |
| `PROG_OPTIMIZE` | `init_config_from_env` | unset / set-to-anything (**value ignored**, even `""` and `"0"` enable it) | `flags.optimize` (bit 2) → `val1+val2` vs `val1*log_level + val2/2` |
| `PROG_BASE_OFFSET` | `parse_env_numeric` | unset → `0100`=64 / numeric / negative / `,`-rejected / `;`-rejected / non-numeric → 0 / overflow | additive term at the end of `envy` |
| `PROG_MULTIPLIER` | `parse_env_numeric` | unset → `012`=10 / numeric / negative / `,`-rejected / `;`-rejected / non-numeric → 0 / overflow | `state.multiplier`, scales `param3` |

`flags.cache_enabled` (bit 3) is hard-wired to 1 and `flags.log_level` (bits
4–6) to `03` by `init_config_from_env`, so **through `envy` they are constants**.
Both are freely settable through the low-level entry points, which is why rows
below drive `perform_operation` across all 8 `log_level` values and
`apply_bit_operations` across both `cache_enabled` values. `flags.reserved`
(bit 7) and the 24 padding bits of the storage word are never read.

**B. Input shapes the code special-cases.**

| axis | states |
|------|--------|
| `envy` `param3` | `== 0` (term skipped) / `!= 0` (term added) |
| `envy` `param4` | `== 0` (term skipped) / `> 0` / `< 0` (arithmetic `>> 2`) |
| `envy` final `result` | `>= 0` (returned) / `< 0` (backup restored, returns `param1`) |
| scalar magnitudes | `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`, random — exercises signed wrap, `<< 1` on negatives, `>> 2` on negatives, `INT_MIN/2` |
| `parse_env_numeric` name | absent / present / `""` / 4096-byte |
| `ConfigFlags` storage | all 256 low-byte patterns × padding-bits zero/garbage |

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_5678_9ABC`, xorshift64\* generator in `tests/common/mod.rs`), not a
single hand-picked value, and additionally with the boundary vector above.

## Rows

`entry point` column: `PE` = `parse_env_numeric`, `IC` = `init_config_from_env`,
`PO` = `perform_operation`, `AB` = `apply_bit_operations`, `EN` = `envy`.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `PE` | variable absent; `default_val` randomized over full `i32` range | [x] |
| 2 | `PE` | variable present, plain decimal digits, randomized 0..=INT_MAX as text | [x] |
| 3 | `PE` | variable present, leading `-`, randomized negative values as text | [x] |
| 4 | `PE` | variable present, leading `+`, leading/trailing whitespace, leading zeros (`"  007"`, `"+42"`) | [x] |
| 5 | `PE` | variable present, digits followed by trailing junk (`"42abc"`) — `atoi` stops at junk | [x] |
| 6 | `PE` | variable present, value at/over `int` bounds (`"2147483647"`, `"2147483648"`, `"-2147483648"`, `"-2147483649"`, 20-digit) | [x] |
| 7 | `PE` | variable present, contains `,` (comma-rejection path) with randomized surrounding digits | [x] |
| 8 | `PE` | variable present, contains `;` but no `,` (semicolon-rejection path) | [x] |
| 9 | `PE` | variable present, contains both `,` and `;` in both orders | [x] |
| 10 | `IC` | all 3×3×2 = 18 combinations of `PROG_VERBOSE` ∈ {unset, contains-`1`, lacks-`1`} × `PROG_DEBUG` ∈ {unset, contains-`1`, lacks-`1`} × `PROG_OPTIMIZE` ∈ {unset, set}; compare the full 4-byte storage word written by both | [x] |
| 11 | `IC` | pre-existing garbage in the caller's `ConfigFlags` word (0x00000000, 0xFFFFFFFF, random) — verifies both libraries overwrite the same bits and leave the same padding bits | [x] |
| 12 | `IC` | `PROG_OPTIMIZE=""` (empty but present) — enables `optimize`; and `PROG_VERBOSE`/`PROG_DEBUG` containing `'1'` embedded mid-string (`"x1x"`) | [x] |
| 13 | `PO` | `optimize=1`, `debug=0`; `val1`/`val2` randomized over full `i32` | [x] |
| 14 | `PO` | `optimize=1`, `debug=1` (adds the two debug `printf`s) | [x] |
| 15 | `PO` | `optimize=0`, `log_level=0`, `debug=0` — product term vanishes, only `val2/2` | [x] |
| 16 | `PO` | `optimize=0`, `log_level=1`, `debug=0` | [x] |
| 17 | `PO` | `optimize=0`, `log_level=2`, `debug=0` | [x] |
| 18 | `PO` | `optimize=0`, `log_level=3` (the value `envy` always produces), `debug=0` | [x] |
| 19 | `PO` | `optimize=0`, `log_level=4`, `debug=0` | [x] |
| 20 | `PO` | `optimize=0`, `log_level=5`, `debug=0` | [x] |
| 21 | `PO` | `optimize=0`, `log_level=6`, `debug=0` | [x] |
| 22 | `PO` | `optimize=0`, `log_level=7`, `debug=0` — maximum 3-bit multiplier, max signed-overflow pressure | [x] |
| 23 | `PO` | `optimize=0`, `debug=1`, `log_level` randomized 0..7 | [x] |
| 24 | `PO` | boundary `val1`/`val2` vector (`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`) × all 8 `log_level` × `optimize` ∈ {0,1} — covers `INT_MIN/2` and signed multiply wrap | [x] |
| 25 | `PO` | exhaustive over all 256 flag low-byte patterns, randomized `val1`/`val2` | [x] |
| 26 | `AB` | `verbose=0`, `cache_enabled=0` — identity | [x] |
| 27 | `AB` | `verbose=0`, `cache_enabled=1` — `\| 0x0F` only | [x] |
| 28 | `AB` | `verbose=1`, `cache_enabled=0` — `<< 1` only, incl. negative and `INT_MIN`/`INT_MAX` inputs (signed shift wrap) | [x] |
| 29 | `AB` | `verbose=1`, `cache_enabled=1` — `<< 1` then `\| 0x0F`, order matters | [x] |
| 30 | `AB` | exhaustive over all 256 flag low-byte patterns × boundary + randomized `value` | [x] |
| 31 | `EN` | no `PROG_*` variables set at all (pure default config: verbose=0 debug=0 optimize=0 log_level=3 cache=1, base_offset=64, multiplier=10); randomized params | [x] |
| 32 | `EN` | `PROG_OPTIMIZE` set only → `optimize` branch; randomized params | [x] |
| 33 | `EN` | `PROG_VERBOSE=1` only → `verbose` printf block **and** the `<< 1` in `apply_bit_operations` (changes the return value, not just output) | [x] |
| 34 | `EN` | `PROG_DEBUG=1` only → debug printf blocks, return value unchanged | [x] |
| 35 | `EN` | `PROG_VERBOSE=1` + `PROG_DEBUG=1` + `PROG_OPTIMIZE=1` (all bits on) | [x] |
| 36 | `EN` | full cross-product of `PROG_VERBOSE`/`PROG_DEBUG`/`PROG_OPTIMIZE` ∈ {unset, "1", "0"} (27 combos) with randomized params | [x] |
| 37 | `EN` | `PROG_BASE_OFFSET` set to randomized decimal values (positive, negative, 0) | [x] |
| 38 | `EN` | `PROG_MULTIPLIER` set to randomized decimal values (positive, negative, 0) | [x] |
| 39 | `EN` | both `PROG_BASE_OFFSET` and `PROG_MULTIPLIER` set, randomized, × verbose/optimize on/off | [x] |
| 40 | `EN` | `PROG_BASE_OFFSET`/`PROG_MULTIPLIER` rejected by `,` or `;` → octal defaults 64 / 10 used | [x] |
| 41 | `EN` | `PROG_BASE_OFFSET`/`PROG_MULTIPLIER` non-numeric → 0 used (distinct from the default!) | [x] |
| 42 | `EN` | `param3 == 0` (term skipped) with `param4 != 0`; randomized | [x] |
| 43 | `EN` | `param4 == 0` (term skipped) with `param3 != 0`; randomized | [x] |
| 44 | `EN` | `param3 == 0` **and** `param4 == 0` | [x] |
| 45 | `EN` | `param4 < 0` → arithmetic `>> 2` (rounds toward −∞, unlike division) | [x] |
| 46 | `EN` | inputs engineered so the final `result < 0` → backup-restore path returns `param1`; × verbose on/off | [x] |
| 47 | `EN` | inputs engineered so final `result` is exactly 0 / exactly −1 (boundary of the `result < 0` test) | [x] |
| 48 | `EN` | boundary vector `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` on all four params (7^4 = 2401 combos) under default env | [x] |
| 49 | `EN` | same 7^4 boundary vector under `optimize=1` | [x] |
| 50 | `EN` | same 7^4 boundary vector under `verbose=1` (exercises `<< 1` overflow together with the recovery path) | [x] |
| 51 | `EN` | large randomized sweep (100k iterations) over all four params × randomly chosen env configuration — the composed pipeline end-to-end | [x] |
| 52 | `EN` + `PE` + `IC` + `PO` + `AB` | **composed pipeline driven at the low level**: call `IC` to build flags, then `PE` for both octal defaults, then `PO`, then the `param3`/`param4` terms, then `AB`, then `+ base_offset`, and assert the manually composed value equals what `EN` returns — for both libraries, cross-checked C↔Rust | [x] |
| 53 | stdout/stderr | byte-for-byte comparison of everything each `.so` writes to fd 1 and fd 2, captured via `dup2` to a temp file, for the verbose/debug-enabled rows (33, 34, 35, 46) and the stderr warning rows (7, 8, 9) | [x] |

## Notes on what is *not* a separate row

* `state.operation = '+'` is written and never read.
* `snprintf` into `buffer` and the two `strchr` scans have no effect on the
  return value; their only observable effect is the verbose/debug `printf`s,
  which row 53 compares byte-for-byte.
* `flags.reserved` and the upper 24 padding bits are never read — covered as a
  negative assertion by rows 11, 25, 30.
* `operation_mode = 0755` in `perform_operation` is only printed under `debug`.

## Gate

- [x] Every row passes across its randomized inputs, C vs Rust, through the
      `.so` exports only.

## Result

All 53 rows pass. Tests live in `tests/phase_b_valid.rs` (rows 1–52) and
`tests/phase_d_output.rs` (row 53). No divergence was found on any valid path.

### Suite validation (mutation testing)

Passing tests only prove something if they can fail. `mutation_check.sh`
injects 26 deliberate defects into `src/lib.rs`, rebuilds, and runs the suite,
expecting each to be caught. Results: **23 caught**, 3 survivors, of which two
are provably unobservable and one was a script artifact:

| survivor | verdict |
|----------|---------|
| `RESERVED_SHIFT: 7 -> 6` | Semantically equivalent. `reserved` is assigned `0` unconditionally by `init_config_from_env`, so `0 << 6` and `0 << 7` are both 0, and nothing ever reads the field. Rows 10–12 compare the whole 4-byte storage word and confirm bit 7 is cleared either way. |
| `printf("… %ld") -> "… %d"` for the colon offset | Semantically equivalent on this ABI. `colon_pos - buffer` is always exactly 6 (`"Result:"`), and a varargs `long` of 6 printed with `%d` reads the low 32 bits of the same register, producing identical bytes. |
| `"Verbose mode enabled"` text change | Script escaping bug, re-run separately: **caught** by rows 33/35/36. |

Caught mutations include every constant (`0o100`, `0o12`, `0o3`, `0o755`,
`0x0F`, `LOG_LEVEL_MASK`, `LOG_LEVEL_SHIFT`, `cache_enabled`), every operator
(`<< 1`, `>> 2`, `/ 2`), every branch condition (`result < 0` → `<= 0`, the
`,`/`;`/`'1'` `strchr` probes, the `optimize` null test), the flag-word byte
offset, the recovery path's source value, the `param3`/`param4` term mix-up,
every `printf`/`fprintf` format string and argument order, and removal of the
`#[no_mangle]` export.

### Configuration matrix

`run_all_configs.sh` runs the whole suite across every axis that exists:

```
declared features: [none]
  ok : cargo check <default> / --no-default-features
  ok : rust:{debug,release} x {<default>,--no-default-features}
       vs c:{sanctioned(no -O), -O2, -O3}      12 configurations, 74 tests each
ALL CONFIGURATIONS PASS
```

* The crate declares **no** `[features]`, the Rust has no `cfg(...)` branches
  and the C has no `#if`/`#ifdef`, so `<default>` and `--no-default-features`
  are the complete feature space; both are run anyway.
* Both Rust profiles are covered because they are materially different
  artifacts (opt-level 0 vs 3, `panic=unwind` vs `abort`, `debug_assertions`
  on vs off) — and that axis is what surfaced the null-pointer divergence
  recorded in `ERRORS.md`.
* The `-O2`/`-O3` C builds are extra (non-gating) configurations: the C relies
  on signed-overflow and signed-shift UB, so an optimizing compiler could in
  principle change its behaviour. It does not here — the Rust matches all three
  C builds. `DIFFTEST_C_SO` / `DIFFTEST_RUST_SO` select which `.so` pair the
  suite loads; the optimized C builds are made out of tree in `/tmp` so nothing
  under `c_src/` is modified.

### No binary to compare

`c_src/CMakeLists.txt` contains only `add_library(... SHARED src/lib.c)` — no
`add_executable` — and the crate has no `[[bin]]` or `src/main.rs`. There is no
driver stdout to diff. The equivalent surface is the library's own `printf`
output, which row 53 compares per call, byte for byte, on both fd 1 and fd 2.
