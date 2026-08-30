# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/slicing.c`. Every `return` other than the
final success `return 0`, every diagnostic `printf`, every comparison that can
reject an input, and every implicit pointer requirement is listed.

`grep -n 'return\|printf\|assert\|if (' c_src/src/slicing.c` yields exactly the
three rejection branches below (lines 45–48, 55–58, 59–62); there are no
`assert`s, no error enums, and no named min/max constants in the library.

A "rejection" is defined as: `slice` returns `1` after printing a diagnostic to
`stdout`. The differential tests compare **both** the return code **and** the
exact bytes written to `stdout`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `slice` | `start_ptr != NULL` and `(size_t)*start_ptr > strlen(mystr)` — i.e. `*start_ptr` strictly greater than `len` (e.g. `len=5`, `start=6`) | prints `Error: start is off the end of the string!\n`, returns `1` | `e1_start_past_end` | [x] |
| E2 | `slice` | `start_ptr != NULL` and `*start_ptr < 0`. C compares `int > size_t`, so the negative `int` is converted to `size_t` and becomes a huge value ⇒ same branch as E1. Covers `-1`, `INT_MIN`, random negatives | prints `Error: start is off the end of the string!\n`, returns `1` | `e2_start_negative` | [x] |
| E3 | `slice` | `stop_ptr != NULL` and `(size_t)*stop_ptr > strlen(mystr)` (e.g. `len=5`, `stop=6`); checked **before** the `stop <= start` check | prints `Error: stop is off the end of the string!\n`, returns `1` | `e3_stop_past_end` | [x] |
| E4 | `slice` | `stop_ptr != NULL` and `*stop_ptr < 0` — same `int`→`size_t` conversion as E2, so a negative `stop` reports "off the end", **not** "must come after start" | prints `Error: stop is off the end of the string!\n`, returns `1` | `e4_stop_negative` | [x] |
| E5 | `slice` | `stop_ptr != NULL`, `*stop_ptr` in range, and `*stop_ptr < *start_ptr` | prints `Error: stop must come after start!\n`, returns `1` | `e5_stop_before_start` | [x] |
| E6 | `slice` | `stop_ptr != NULL`, `*stop_ptr` in range, and `*stop_ptr == *start_ptr` (the check is `<=`, so equal bounds — a legitimate empty Python-style slice — are **rejected**) | prints `Error: stop must come after start!\n`, returns `1` | `e6_stop_equals_start` | [x] |
| E7 | `slice` | `start_ptr == NULL` (so `start = 0`) and `stop_ptr != NULL` with `*stop_ptr == 0`: `0 > len` is false, then `0 <= 0` ⇒ rejected | prints `Error: stop must come after start!\n`, returns `1` | `e7_null_start_zero_stop` | [x] |
| E8 | `slice` | empty string (`len == 0`) with `stop_ptr != NULL`, `*stop_ptr == 0`: passes the range check, fails `stop <= start` | prints `Error: stop must come after start!\n`, returns `1` | `e8_empty_string_zero_stop` | [x] |
| E9 | `slice` | empty string (`len == 0`) with `*start_ptr == 1` (one past the only valid value `0`) | prints `Error: start is off the end of the string!\n`, returns `1` | `e9_empty_string_start_one` | [x] |
| E10 | `slice` | `start_ptr != NULL` with `*start_ptr == len` (boundary: `>` not `>=`, so this is **accepted**) and `stop_ptr != NULL` with any in-range `*stop_ptr`: no in-range `stop` can exceed `start == len`, so this always hits `stop <= start` | prints `Error: stop must come after start!\n`, returns `1` | `e10_start_at_len_with_stop` | [x] |
| E11 | `slice` | check *ordering*: both `*start_ptr` and `*stop_ptr` invalid (both out of range) — the `start` check runs first, so the **start** message wins | prints `Error: start is off the end of the string!\n`, returns `1` | `e11_both_invalid_start_wins` | [x] |
| E12 | `slice` | check *ordering*: `*stop_ptr` out of range **and** `*stop_ptr <= *start_ptr` (e.g. `stop` negative and `start` positive) — the range check runs first, so the **"off the end"** message wins | prints `Error: stop is off the end of the string!\n`, returns `1` | `e12_stop_range_before_order` | [x] |
| E13 | `slice` | `mystr == NULL`: the C code dereferences it unconditionally via `strlen(mystr)` with **no null check**. This is UB; on Linux/glibc it faults | process terminates with `SIGSEGV` (no return, nothing printed) | `e13_null_string_faults` (forked subprocess, compares termination signal) | [x] |

## Generic boundary cases also covered (not distinct C branches)

| # | condition | expected | test | [x] |
|---|-----------|----------|------|-----|
| G1 | `start_ptr == NULL && stop_ptr == NULL` (both "absent" sentinels) | success, whole string | `phase_b` row C1 | [x] |
| G2 | `*start_ptr == INT_MAX` / `*stop_ptr == INT_MAX` (oversized length) | E1 / E3 respectively (`(size_t)INT_MAX > len`) | `g2_int_max_bounds` | [x] |
| G3 | `*start_ptr == INT_MIN` / `*stop_ptr == INT_MIN` (extreme negative) | E2 / E4 | `g3_int_min_bounds` | [x] |
| G4 | zero length: `""` with every combination of null/`0`/`1` bounds | see E7–E9 and `phase_b` row C2 | `g4_empty_string_matrix` | [x] |
| G5 | one step past the valid range on both sides for many random lengths: `start ∈ {-1, len, len+1}`, `stop ∈ {-1, len, len+1}` | exhaustive cross-product, C vs Rust | `g5_off_by_one_matrix` | [x] |
| G6 | out-of-range "enum" values across the FFI boundary: `slice` declares no `enum` parameters, so the analogous input is an arbitrary `int` bit pattern in `*start_ptr`/`*stop_ptr` with no meaningful interpretation. Fuzzed over the full `i32` range | identical return code + stdout | `g6_full_int_range_fuzz` | [x] |
| G7 | the bound pointers are **not** written by the C code | `*start_ptr` / `*stop_ptr` unchanged after both calls; `mystr` buffer unchanged | asserted inside the shared harness on every call | [x] |
| G8 | aliased bound pointers: `slice(s, &n, &n)` — a legal C call, well defined here because the code only *reads* through the pointers. `start == stop` always, so the ordering check rejects it unless `n` is out of range (start check fires first) | `1` + `Error: stop must come after start!\n`, or `1` + `Error: start is off the end of the string!\n` when `(size_t)n > len` | `g8_aliased_bound_pointers` | [x] |

All 13 + 8 rows are checked off in `tests/phase_c_errors.rs` (E13 and the
matrices included).

## Not mechanically testable

| condition | why |
|-----------|-----|
| `strlen(mystr) > INT_MAX` with `stop_ptr == NULL` | `stop = len` narrows `size_t`→`int` (implementation-defined; gcc truncates modulo 2³²). The Rust `len as c_int` truncates identically, but reproducing it needs a >2 GiB string, so it is reviewed by inspection rather than executed. |
| `start_ptr` / `stop_ptr` pointing at unreadable memory | Plain UB with no defined C behaviour to match; `mystr == NULL` (E13) is covered because it is the one null case the C code reaches unconditionally. |
