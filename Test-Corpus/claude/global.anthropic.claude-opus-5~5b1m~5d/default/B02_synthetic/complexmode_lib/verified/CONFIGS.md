# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Rows are the cross-product of the
axes the C actually branches on, pruned to the combinations `c_src/src/lib.c`
treats differently.

## Axes the C code branches on

**A1 — Cargo/`#ifdef` build options.** None. `translation/Cargo.toml` has no
`[features]` table and `lib.c` contains no `#ifdef`/`#if` around any code, so
there is exactly one build configuration. (`check_features.sh` enumerates the
feature list from `Cargo.toml` and re-runs the suite for every combination it
finds, which is the single default one.)

**A2 — the `mode` selector of `complexmode` (`lib.c:115`).** A 5-way `switch`:
`1` addition, `2` multiplication, `3` array sum, `4` complex, `default` reject.

**A3 — the permission bitmask.** `check_permissions(perms, required)` is
`(perms & required) == required`. The distinguished `required` values in the
source are `READ_PERM|WRITE_PERM == 0600` (`lib.c:52`) and `0100` (`lib.c:154`),
and the distinguished `perms` value is the hard-coded `0644` in `complexmode`
(`lib.c:103`). `perms` reaches `safe_add`/`check_permissions` freely from
outside, so the axis is: `required` = 0 / single bit / multi-bit / negative,
crossed with `perms` = superset / subset / disjoint / partial overlap.
Note `0644 & 0100 == 0 != 0100`, so inside `complexmode` mode 4 the
`value1*value2+value3` branch is **dead** and only `value1+value2+value3` runs;
the multiply branch is reachable only by calling `check_permissions` directly.

**A4 — `count` shape for `copy_and_sum` (`lib.c:67`).** `0` / `1` / `3` (the
value `complexmode` hard-codes) / many; plus the value-dependent `int`
accumulator, which wraps.

**A5 — string shape for `create_result_string` / `compare_operations`.** empty /
short / exactly-fits / truncating (>63 formatted bytes) / bytes `>= 0x80`
(`strcmp` unsigned-char comparison) / common prefix / first-byte difference /
long.

**A6 — integer value shape.** `0` / small / `INT_MAX` / `INT_MIN` / values whose
`a+b` or `a*b` overflows `int` (C UB; GCC and the Rust `wrapping_*` translation
must agree on two's-complement wraparound) / mixed signs.

**A7 — entry-point level.** All 7 exported functions are driven directly, not
just the `complexmode` one-shot wrapper: `check_permissions`, `safe_add`,
`create_result_string`, `multiply_with_log`, `copy_and_sum`,
`compare_operations`, `complexmode`.

**A8 — observable channels.** Return value, `stdout` bytes, the heap buffer
returned through `char*` / `char**` out-params, and (for `complexmode` mode 2)
the ordering of the printed lines. Every row compares **all** applicable
channels byte-for-byte.

**A9 — build profile.** `debug_assertions` changes rustc's *generated code*
(raw-pointer null-check instrumentation, overflow checks), so `cargo test` and
`cargo test --release` are genuinely distinct configurations. Each test binary
loads the cdylib from its OWN profile directory (guarded by `d7_...` in
`tests/phase_d_symbols.rs`), and `check_features.sh` runs every row under both.
This axis is what exposed the one real divergence found -- see `FINDINGS.md`.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `check_permissions` | `required == 0`, randomized `perms` over the full `i32` range (always-accept boundary) | [x] |
| C2 | `check_permissions` | `required` = a single bit (`READ_PERM` 0400, `WRITE_PERM` 0200, `EXEC_PERM` 0100), `perms` randomized — accept and reject both hit | [x] |
| C3 | `check_permissions` | `required` = multi-bit `0600` / `0700` / `0644`, `perms` = exact superset, exact equal, partial overlap, disjoint | [x] |
| C4 | `check_permissions` | `required` and/or `perms` negative (sign bit set, `-1`, `INT_MIN`), randomized | [x] |
| C5 | `check_permissions` | exhaustive sweep of all 512 × 512 low-9-bit `perms`×`required` pairs (the whole permission-bit space the macros describe) | [x] |
| C6 | `safe_add` | `perms` grants both `0400\|0200` → returns `a + b`; `a`,`b` randomized small | [x] |
| C7 | `safe_add` | `perms` grants both; `a`,`b` chosen so `a + b` overflows `int` (positive and negative overflow, `INT_MAX`+1, `INT_MIN`-1) | [x] |
| C8 | `safe_add` | `perms` missing exactly one of the two bits (0400-only, 0200-only) → reject path + message | [x] |
| C9 | `safe_add` | `perms` fully randomized over `i32` (both paths interleaved, message/`stdout` ordering checked) | [x] |
| C10 | `create_result_string` | `op` = `""` (empty), `val` = 0 / ±small — shortest formatted output | [x] |
| C11 | `create_result_string` | `op` = short ASCII, `val` = randomized full `i32` incl. `INT_MIN`/`INT_MAX` (widest `%d` output) | [x] |
| C12 | `create_result_string` | `op` length swept `0..=80` so the formatted string crosses the 63-byte `snprintf` truncation boundary from both sides | [x] |
| C13 | `create_result_string` | `op` containing bytes `>= 0x80` / embedded punctuation / `%` characters (must be treated as data, not format) | [x] |
| C14 | `multiply_with_log` | valid out-param, `a`,`b` randomized small → returns `a*b` and writes `Operation: multiply, Value: <a*b>` | [x] |
| C15 | `multiply_with_log` | valid out-param, `a*b` overflows `int` (incl. `INT_MIN * -1`, `INT_MAX * 2`, two large randoms) — the product is computed twice in the C, both must wrap identically | [x] |
| C16 | `multiply_with_log` | `a` or `b` == 0 → product 0, and negative products (message must carry the `-` sign) | [x] |
| C17 | `copy_and_sum` | `count == 0`, non-NULL `src` (`malloc(0)`, empty loop) | [x] |
| C18 | `copy_and_sum` | `count == 1`, randomized element incl. `INT_MIN`/`INT_MAX` | [x] |
| C19 | `copy_and_sum` | `count == 3` (the shape `complexmode` mode 3 uses), randomized elements | [x] |
| C20 | `copy_and_sum` | `count` = many (2, 4, 5, 8, 17, 64, 255, 1000, 65536, 1048576), randomized elements, plus `count` < buffer length so only a prefix is summed | [x] |
| C21 | `copy_and_sum` | `count` many with elements chosen so the running `int` sum overflows mid-loop and wraps repeatedly | [x] |
| C22 | `compare_operations` | equal strings (incl. `""` vs `""`), so `strcmp == 0` | [x] |
| C23 | `compare_operations` | differ at first byte, both orders (sign of result) | [x] |
| C24 | `compare_operations` | one is a proper prefix of the other, both orders | [x] |
| C25 | `compare_operations` | differ only at a late byte; long (256-byte) strings — exercises the vectorized libc path | [x] |
| C26 | `compare_operations` | bytes `>= 0x80` vs `< 0x80` at the differing position — `strcmp` must compare as *unsigned* char | [x] |
| C27 | `compare_operations` | fully randomized byte strings, randomized lengths `0..=32` | [x] |
| C28 | `complexmode` | `mode == 1`, `value1`/`value2` randomized small; `value3` randomized but unused | [x] |
| C29 | `complexmode` | `mode == 1`, `value1 + value2` overflows (`INT_MAX`/`INT_MIN` corners) | [x] |
| C30 | `complexmode` | `mode == 2`, randomized small values → `Mode 2: Operation: multiply, Value: N` then `Operation performed: multiplication` | [x] |
| C31 | `complexmode` | `mode == 2`, `value1 * value2` overflows; and `value1 * value2 == 0` (the `strcmp(log,"") == 0` gate must still be false because the message is non-empty) | [x] |
| C32 | `complexmode` | `mode == 3`, randomized `value1..value3`; sum-overflow corners | [x] |
| C33 | `complexmode` | `mode == 4`, randomized values — verifies the *dead* multiply branch is not taken (`0644 & 0100 == 0`) so the result is `v1+v2+v3`, incl. overflow corners | [x] |
| C34 | `complexmode` | all four valid modes crossed with the value shapes `{all zero, all INT_MAX, all INT_MIN, mixed sign, randomized}` | [x] |
| C35 | `complexmode` | randomized `mode` over the full `i32` range crossed with randomized values — valid and `default` arms interleaved, checking the `Operation performed:` suffix line appears for 1–4 and is suppressed for `default` | [x] |
| C36 | pipeline: `create_result_string` → `compare_operations` → `copy_and_sum` | compose the low-level entry points the way `complexmode` does, but with caller-chosen data: build two strings with `create_result_string`, compare them with `compare_operations`, and sum a buffer with `copy_and_sum`, asserting every intermediate matches | [x] |

| C37 | `create_result_string` + `compare_operations` + `multiply_with_log`, MIXED across the two `.so`s | cross-library interop: a heap buffer minted by ONE library is read, `strcmp`'d and `free`d by the OTHER. Only passes if the translation forwards to the same libc `malloc`/`free` instead of using Rust's own allocator, i.e. it checks that ownership is genuinely interchangeable across the FFI boundary | [x] |

All 37 rows are checked off — see `tests/phase_b_configs.rs`.
