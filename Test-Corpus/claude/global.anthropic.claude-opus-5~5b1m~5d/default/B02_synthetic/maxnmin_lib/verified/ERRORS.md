# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. Every early `return`, sentinel
value, clamp, range check, implicit limit (`MAX_NODES`, `MAX_NAME_LEN`,
`INT_MAX`, `INT_MIN`), missing null check and unchecked-overflow site in the C
source gets exactly one row. There are no `assert`s, no error enums and no
`errno` use in this library — the whole error surface is sentinel returns
(`-1`, `NULL`, `0`, `0.0`, `INT_MAX`, `INT_MIN`) plus un-guarded UB sites.

Test files: `translation/tests/error_paths.rs` (row `En` -> test `en_...`) and
`translation/tests/null_pointer.rs` (rows E6 and E21).
Column `[x]` = differential test written **and passing** against both `.so`s.

| # | function | trigger (exact invalid input / condition) | expected C result | [x] |
|---|----------|-------------------------------------------|-------------------|-----|
| E1 | `add_node` | `node_count >= MAX_NODES` (line 45): the 101st `add_node` on a pristine library | returns `-1`, storage untouched, `node_count` stays 100 | [x] |
| E2 | `add_node` | every subsequent call once full (102nd … 150th) | keeps returning `-1` (no wrap, no partial write) | [x] |
| E3 | `add_node` | `name` longer than `MAX_NAME_LEN-1` = 49 bytes (`strncpy` copies 49, then `name[49]='\0'`) | name truncated to first 49 bytes + NUL; return value is the new index | [x] |
| E4 | `add_node` | `name` exactly 49 bytes (boundary, one below the limit) | full 49 bytes stored + NUL at `[49]` | [x] |
| E5 | `add_node` | `name = ""` (zero length) | `name` all zero bytes; success index returned | [x] |
| E6 | `add_node` | `name = NULL` (missing null check, line 56 `strncpy`) | dereferences NULL → `SIGSEGV` (11) — identical fatal signal required | [x] |
| E7 | `add_node` | `id`/`parent_id` at `INT_MIN` / `INT_MAX` (no range check at all) | stored verbatim, success index returned | [x] |
| E8 | `add_node` | `value` = NaN / ±inf (no validation) | stored verbatim (bit pattern preserved), success index | [x] |
| E9 | `find_node_by_id` | `node_count == 0` (loop never entered, line 69) | returns `NULL` | [x] |
| E10 | `find_node_by_id` | `id` not present among the stored nodes | returns `NULL` | [x] |
| E11 | `find_node_by_id` | `id` present but `active == 0` (line 65 requires `active`) | returns `NULL` | [x] |
| E12 | `find_node_by_id` | `id` present twice, first copy `active == 0` | returns pointer to the **second** (first *active*) match | [x] |
| E13 | `find_node_by_id` | `id` = `INT_MIN` / `INT_MAX` / `0` never inserted | returns `NULL` | [x] |
| E14 | `get_children_count` | `parent_id` matches nothing (or store empty) | returns `0` (never negative, never an error code) | [x] |
| E15 | `get_children_count` | `parent_id` matches only *inactive* nodes | returns `0` | [x] |
| E16 | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` (line 84) | returns exactly `+0.0` (bitwise `0x0000000000000000`) | [x] |
| E17 | `calculate_subtree_sum` | node exists but `value` is NaN | returns NaN (propagates; no check) | [x] |
| E18 | `calculate_subtree_sum` | children values overflow the `double` accumulator (`1e308 + 1e308`) | returns `+inf` (no check) | [x] |
| E19 | `calculate_subtree_sum` | `+inf` child and `-inf` child in the same subtree | returns NaN | [x] |
| E20 | `process_string` | `*str == '\0'` — the `if (*str)` guard on line 102 is false | returns `0` | [x] |
| E21 | `process_string` | `str = NULL` (missing null check, line 102) | dereferences NULL → `SIGSEGV` (11) | [x] |
| E22 | `process_string` | bytes with the high bit set (`0x80`–`0xFF`); `char` is signed on x86-64 so `(int)(*str)` sign-extends | negative addends → possibly negative result | [x] |
| E23 | `process_string` | `result += (int)(*str)` overflows `int` (≈17 M × `0x7F`) — unchecked signed overflow | wraps (two's complement, `-O0` gcc) | [x] |
| E24 | `safe_double_to_int` | `d > (double)INT_MAX` (line 113) — incl. `2147483647.5`, `2147483648.0`, `1e300`, `+inf` | returns `INT_MAX` = `2147483647` | [x] |
| E25 | `safe_double_to_int` | `d < (double)INT_MIN` (line 116) — incl. `-2147483648.5`, `-1e300`, `-inf` | returns `INT_MIN` = `-2147483648` | [x] |
| E26 | `safe_double_to_int` | `d != d`, i.e. NaN — reached only *after* both comparisons fail (line 120) | returns `0`; both quiet and signalling NaN payloads, both signs | [x] |
| E27 | `safe_double_to_int` | `d` exactly `(double)INT_MAX` / `(double)INT_MIN` (boundary: comparison is strict `>` / `<`) | falls through to `(int)d` → `INT_MAX` / `INT_MIN` | [x] |
| E28 | `maxnmin` | `selected_node == NULL` (line 142): `(param1 % 6) + 1 <= 0`, e.g. `param1 = -1 … -5`, `INT_MIN` | first block skipped entirely (no name sum, no subtree sum) | [x] |
| E29 | `maxnmin` | `*name_ptr == 0` (line 145) — dead branch for the six built-in names, still must not diverge | `process_string` not called | [x] |
| E30 | `maxnmin` | `second_node == NULL` (line 158): `(param2 % 6) + 1 <= 0` | second block skipped (no `value * param3` term) | [x] |
| E31 | `maxnmin` | `param3 == -1` → `(double)(param1+param2) / 0.0` — unchecked division by zero | `±inf` (or NaN when `param1+param2 == 0`), then `* param4` → `safe_double_to_int` clamps to `INT_MAX`/`INT_MIN`/`0` | [x] |
| E32 | `maxnmin` | `param3 == INT_MAX` → `param3 + 1` signed overflow (UB; `-O0` wraps to `INT_MIN`) | divisor `-2147483648.0` | [x] |
| E33 | `maxnmin` | `param1 + param2` signed overflow (`INT_MAX + INT_MAX`, `INT_MIN + INT_MIN`) | wraps before the `(double)` cast | [x] |
| E34 | `maxnmin` | `param1`/`param2`/`param4` = `INT_MIN` → `INT_MIN % 6 == -2`, `INT_MIN % 3 == -2` (C truncating `%`) | `node_id = -1`, `parent_id = -1` (matches the root's `parent_id`!) | [x] |
| E35 | `maxnmin` | `(param4 % 3) + 1` lands on a non-existent parent (`0`, `-1`) | `get_children_count` returns `0` or `1` (`-1` is the root's parent) | [x] |
| E36 | `maxnmin` | `result` accumulation overflows `int` (unchecked signed overflow) | wraps | [x] |
| E37 | out-of-range "enum"-like int across FFI | `Node.active` written through the pointer from `find_node_by_id` with a value that is neither 0 nor 1 (`2`, `-1`, `INT_MIN`, `0x80000000`) — C treats *any* non-zero as true | node stays visible to `find_node_by_id` / `get_children_count` / `calculate_subtree_sum` | [x] |
| E38 | `add_node` after `maxnmin` | `maxnmin` resets `node_count = 0` (line 130) mid-life, silently discarding all previously added nodes | subsequent `add_node` returns index `6`; old nodes 7.. unreachable | [x] |
| E39 | `calculate_subtree_sum` | parent cycle / self-parent (`id == parent_id`) — line 91 has no visited-set, so recursion never terminates | unbounded recursion → stack exhaustion (fatal signal). **Documented, not executed**: identical UB in both, and the fatal signal observed depends on the *test harness'* SIGSEGV handler, not on the library. | n/a |
| E40 | `process_string` | non-NUL-terminated buffer (reads past the end, line 103) | out-of-bounds read (UB). **Documented, not executed** — same UB in both, unobservable deterministically. | n/a |

## Results

All 38 executable rows pass; E39/E40 are documented-only (see below).

* `tests/error_paths.rs` — 36 tests, all green.
* `tests/null_pointer.rs` — 2 tests (E6, E21), all green.

### How E6 / E21 are compared

Both calls kill the process, so "the same error" means "the same death". The test
binary re-executes itself once per library (an `#[ignore]`d payload test selected
by `HARVEST_CRASH_TARGET`) and compares the full `ExitStatus`: C and Rust must
both terminate with **signal 11 (SIGSEGV)** and no exit code. A child that
*survives* the null dereference also fails the test.

This is why `Cargo.toml` sets `[profile.dev] debug-assertions = false`: with
debug assertions on, rustc inserts a "null pointer dereference" UB check that
converts these two crashes into a panic → `SIGABRT` (6), a *different* observable
failure mode from the C's `SIGSEGV` (11). With the setting, the dev-profile and
release-profile `cdylib`s are both byte-identical in behaviour to the C — the
whole suite passes with `HARVEST_RUST_SO` pointed at either one.

## Notes on the two "n/a" rows

E39 and E40 are genuine holes in the C API (it has no way to reject them), and
the Rust translation reproduces the same unchecked loops/recursion — verified by
reading `translation/src/lib.rs` (`calculate_subtree_sum` recurses with no
visited set; `process_string` walks until it reads a `0`). They are excluded
from execution because their observable result is "the process dies somewhere",
which is not a value that can be compared byte-for-byte, and because running
them would take the differential harness down with them.
