# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Every distinct way `c_src/src/lib.c` rejects / errors on / special-cases input.
Derived mechanically by grepping the C source for every `return` that is not the
normal success value, every guard condition, every min/max constant, every null
check and every implicit-UB trigger:

```sh
grep -nE 'return (-1|NULL|0\.0|INT_MAX|INT_MIN)|== NULL|!= NULL|>=|<=|[^<>=!]>[^=]|<[^=]|!=|#define MAX' c_src/src/lib.c
```

Raw inventory of guards found (line numbers from `c_src/src/lib.c`):

| line | code | kind |
|---|---|---|
| 30 | `#define MAX_NODES 100` | max constant |
| 31 | `#define MAX_NAME_LEN 50` | max constant |
| 45–47 | `if (node_count >= MAX_NODES) { return -1; }` | error return |
| 56–57 | `strncpy(..., MAX_NAME_LEN - 1); name[MAX_NAME_LEN-1]='\0'` | silent truncation + forced NUL |
| 56 | `strncpy(dst, name, ...)` with `name == NULL` | unchecked null deref |
| 65 | `if (node_storage[i].id == id && node_storage[i].active)` | filter (inactive skipped) |
| 69 | `return NULL;` | not-found sentinel |
| 75 | `if (node_storage[i].parent_id == parent_id && ...active)` | filter |
| 84–86 | `if (node == NULL) { return 0.0; }` | not-found sentinel |
| 92 | `sum += calculate_subtree_sum(...)` (unbounded recursion) | no cycle guard |
| 102 | `if (*str)` with `str == NULL` | unchecked null deref |
| 102 | `if (*str)` false | empty-string early exit |
| 113–115 | `if (d > (double)INT_MAX) return INT_MAX;` | saturation |
| 116–118 | `if (d < (double)INT_MIN) return INT_MIN;` | saturation |
| 120–122 | `if (d != d) return 0;` | NaN sentinel |
| 124 | `return (int)d;` | truncation toward zero |
| 139 | `(param1 % 6) + 1` | can yield ids ≤ 0 → miss |
| 142 | `if (selected_node != NULL)` false | whole block skipped |
| 145 | `if (*name_ptr)` false | unreachable for the 6 seeded nodes |
| 155–158 | `(param2 % 6) + 1`, `if (second_node != NULL)` false | whole block skipped |
| 159 | `second_node->value * param3` | double overflow → ±inf |
| 165 | `(param4 % 3) + 1` | can yield `-1`/`0` |
| 169 | `(double)(param3 + 1)` | signed-int overflow (UB) + division by 0.0 |
| 169 | `(double)(param1 + param2)` | signed-int overflow (UB) |
| 152/162/167/173 | `result += ...` | signed-int overflow (UB) |

Note: this API contains **no `enum` types and no error-code enum**, so there is no
"out-of-range enum variant" to pass across FFI. The *equivalent* class of "any int
is a legal FFI input with no valid meaning" is covered instead by rows E13–E16
(`active` set to values other than 0/1, and the full `int` domain incl.
`INT_MIN`/`INT_MAX` for every `int` parameter) — see rows below.

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|----------------------------------------------|-------------------|---|
| E1 | `add_node` | called when `node_count == MAX_NODES` (i.e. the 101st call since the last `maxnmin`) | returns `-1`; `node_count` unchanged, storage untouched | [x] |
| E2 | `add_node` | called at `node_count == MAX_NODES - 1` (100th call, last legal slot) | returns `99` (boundary one step *inside* the range) | [x] |
| E3 | `add_node` | `name` longer than `MAX_NAME_LEN - 1` (≥ 50 bytes) | no error; name silently truncated to 49 bytes, `name[49] == '\0'` | [x] |
| E4 | `add_node` | `name` exactly 49 bytes + NUL (boundary) | stored verbatim, `name[49] == '\0'` | [x] |
| E5 | `add_node` | `name == ""` (zero length) | stored as all-NUL name; returns the new index | [x] |
| E6 | `add_node` | `name == NULL` | unchecked → `strncpy(dst, NULL, 49)` → SIGSEGV (crash, no return) | [x] |
| E7 | `find_node_by_id` | `id` that was never added (incl. `0`, negative, `INT_MIN`, `INT_MAX`) | returns `NULL` | [x] |
| E8 | `find_node_by_id` | called while `node_count == 0` (empty storage), any `id` | returns `NULL` | [x] |
| E9 | `find_node_by_id` | matching node exists but its `active` field is `0` | returns `NULL` (inactive is skipped) | [x] |
| E10 | `calculate_subtree_sum` | `node_id` not found (missing / `0` / negative / `INT_MIN` / `INT_MAX`) | returns `0.0` (positive zero, bits `0x0000000000000000`) | [x] |
| E11 | `calculate_subtree_sum` | node exists but is inactive (`active == 0`) | returns `0.0` (lookup fails) | [x] |
| E12 | `calculate_subtree_sum` | parent/child cycle (e.g. a node whose `parent_id == its own id`) — no cycle guard | infinite recursion → stack exhaustion → abnormal termination | [x] |
| E13 | `find_node_by_id` / `get_children_count` / `calculate_subtree_sum` | `active` set through the returned `Node*` to a non-0/1 int (`2`, `-1`, `INT_MIN`, `0x100`) — "out-of-range boolean" across FFI | any non-zero is truthy → node is visible; only exactly `0` hides it | [x] |
| E14 | `get_children_count` | `parent_id` that no node references (incl. `0`, `INT_MIN`, `INT_MAX`) | returns `0` | [x] |
| E15 | `get_children_count` | called while `node_count == 0` | returns `0` | [x] |
| E16 | `get_children_count` | `parent_id == -1` (the sentinel the seeded root uses) — *not* an error but the value one step below the smallest real id | returns the count of `-1`-parented nodes (`1` for `maxnmin`'s seed set) | [x] |
| E17 | `process_string` | `str == NULL` | unchecked `*str` → SIGSEGV (crash, no return) | [x] |
| E18 | `process_string` | `str == ""` (zero length) | returns `0` | [x] |
| E19 | `process_string` | bytes with the high bit set (`0x80`–`0xFF`) | `char` is signed on x86-64 → each contributes a **negative** value (`0x80` → `-128`) | [x] |
| E20 | `process_string` | sum of bytes exceeds `INT_MAX` (oversized length, ~17 M × `0x7F`) | signed overflow (UB) → wraps (two's complement `addl`) | [x] |
| E21 | `safe_double_to_int` | `d > (double)INT_MAX`, e.g. `2147483648.0`, `1e300`, `+INFINITY` | returns `INT_MAX` (`2147483647`) | [x] |
| E22 | `safe_double_to_int` | `d == (double)INT_MAX` exactly (one step *inside* the range) | falls through → returns `2147483647` | [x] |
| E23 | `safe_double_to_int` | `d < (double)INT_MIN`, e.g. `-2147483648.5`, `-1e300`, `-INFINITY` | returns `INT_MIN` (`-2147483648`) | [x] |
| E24 | `safe_double_to_int` | `d == (double)INT_MIN` exactly (one step *inside* the range) | falls through → returns `-2147483648` | [x] |
| E25 | `safe_double_to_int` | `d` is NaN (quiet, signalling, negative, custom payload) | the two range compares are false, then `d != d` → returns `0` | [x] |
| E26 | `safe_double_to_int` | `d == -0.0` / subnormal / fractional (`1.9`, `-1.9`) | truncation **toward zero** → `0`, `0`, `1`, `-1` | [x] |
| E27 | `maxnmin` | `param1` such that `(param1 % 6) + 1 <= 0` (any `param1 < 0` not ≡ 0 mod 6, e.g. `-1`) | `find_node_by_id` → `NULL` → the whole first block (name sum + subtree sum) is **skipped** | [x] |
| E28 | `maxnmin` | `param2` such that `(param2 % 6) + 1 <= 0` (e.g. `-1`) | `second_node == NULL` → the multiply block is **skipped** | [x] |
| E29 | `maxnmin` | `param3 == -1` → `(double)(param3 + 1) == 0.0`, i.e. division by zero | `x/0.0` → `±INFINITY` (or NaN when `param1+param2 == 0`); then `*= param4`; `safe_double_to_int` maps it to `INT_MAX` / `INT_MIN` / `0` | [x] |
| E30 | `maxnmin` | `param3 == INT_MAX` → `param3 + 1` overflows (UB) | wraps to `INT_MIN`, so the divisor becomes `-2147483648.0` | [x] |
| E31 | `maxnmin` | `param1 + param2` overflows (e.g. both `INT_MAX`, or both `INT_MIN`) (UB) | wraps two's complement before the `(double)` conversion | [x] |
| E32 | `maxnmin` | `param1 == INT_MIN` / `param2 == INT_MIN` → `INT_MIN % 6 == -2`; `param4 == INT_MIN` → `INT_MIN % 3 == -2` | C `%` truncates toward zero; ids become `-1` and `-1` | [x] |
| E33 | `maxnmin` | `param4 % 3 + 1 == 0` (e.g. `param4 == -1`) → `get_children_count(0)` | `0` children → contributes `0` | [x] |
| E34 | `maxnmin` | `param4 % 3 + 1 == -1` (e.g. `param4 == -2`) → `get_children_count(-1)` | matches the seeded root → contributes `1 * 10 == 10` | [x] |
| E35 | `maxnmin` | accumulated `result` overflows `int` (e.g. `param3` huge → two `INT_MAX` contributions) (UB) | wraps two's complement | [x] |
| E36 | `maxnmin` | `param3` huge → `value * param3` exceeds `INT_MAX` | `safe_double_to_int` saturates that term to `INT_MAX` / `INT_MIN` | [x] |
| E37 | `add_node` | `id`/`parent_id` at the extremes (`INT_MIN`, `INT_MAX`, `0`, `-1`) and `value` = NaN / ±inf | accepted verbatim (no validation at all) — NaN `value` then poisons `calculate_subtree_sum` | [x] |
| E38 | `maxnmin` | called twice in a row (state carry-over) | `node_count = 0` reset at entry ⇒ second call returns the **same** value; storage always ends with exactly 6 nodes | [x] |

## Row → test mapping

All 38 rows have a passing differential test.

| rows | test file | notes |
|---|---|---|
| E1–E5, E7–E11, E13–E16, E18–E38 | `tests/errors.rs` | plain in-process differential calls; also cross-checked against an independent third implementation of the C semantics (`model_maxnmin`, `model_safe_double_to_int`) over 110 000+ randomized inputs |
| E6, E17 (SIGSEGV) and E12 (stack exhaustion) | `tests/crash.rs` | each trigger runs in a forked child (this test binary re-executed with `DIFF_CRASH_CASE=…`) so the parent survives and can compare the **exact termination signal**, not just "both failed" |

## Divergences found and fixed

Both were found by these tests and fixed in `src/lib.rs` (never in `c_src/`):

1. **E6 / E17 — null-pointer death signal.** The original Rust dereferenced the
   caller's `name` / `str` pointer directly. With `debug_assertions` on, rustc
   instruments raw-pointer derefs, so a NULL argument produced a non-unwinding
   Rust panic → `abort()` → **SIGABRT**, whereas the C faults → **SIGSEGV**.
   Fixed by delegating to libc `strncpy` / `strlen` — which is literally what the
   C source calls — so the fault occurs inside libc exactly as in C. Now SIGSEGV
   in both, under both the `dev` and `release` profile.

2. **NaN payload of `calculate_subtree_sum`.** `sum += child` is compiled to a
   single `addsd`, and on x86 the *operand position* decides which NaN survives
   when both operands are NaN (SRC1 wins, quieted if signalling). gcc emits
   `addsd xmm0, xmm1` with the recursive call's result as SRC1, but LLVM treats
   `fadd` as commutative and the optimising build swapped the operands — so a
   release build returned a different NaN bit pattern (e.g.
   `0xFFF80000000000FF` instead of `0xFFF8000000000000`). Fixed by resolving the
   tie explicitly in `add_child_into_sum` instead of relying on codegen, so the
   result no longer depends on the optimisation level. Covered by CONFIGS.md row
   C23b.

## Notes on rows that are UB in the C

E6, E12, E17, E20, E30, E31 and E35 are undefined behaviour in C (null deref,
unbounded recursion, signed overflow). The reference `.so` is the ground truth:
it is built by `c_src/CMakeLists.txt` with no optimisation flags, where gcc emits
plain `addl` (two's-complement wrap) and an ordinary faulting load. The Rust
matches that observed behaviour exactly (`wrapping_*`, libc string calls), and the
tests assert it in both profiles.
