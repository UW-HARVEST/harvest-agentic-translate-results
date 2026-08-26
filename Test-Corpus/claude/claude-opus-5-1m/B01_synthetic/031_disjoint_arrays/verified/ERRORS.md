# ERRORS.md — error/rejection surface table (Phase A, tested in Phase C)

Derived mechanically from `c_src/src/main.c`. The complete set of
control-flow / rejection constructs in that file is:

```
27:    for (int i = 0; i < len; i++) {          <- fma_array loop guard
33:    if (len == 0) return 0;                  <- call_fma early return
39:    for (int i = 0; i < len; i++) {          <- call_fma fill loop guard
45:    return out[len-1];                       <- call_fma OOB read when len < 0
51:    for (i = 0; i < 100; i++) {              <- main input cap
52:        if (scanf("%d", &data[i]) != 1) {    <- main scan rejection
53:            break;
60:    return 0;                                <- main exit status
```

There are **no** `assert`s, no error enums, no `RETURN_ERROR`-style macros, no
`return -1`, no `return NULL`, no `errno` use and no named min/max constants in
the C source. The library reports nothing through out-params. Every distinct
way the C rejects, short-circuits or mis-handles its input is therefore one of
the rows below.

`sentinel` column meaning: the exact value/observable the differential test
compares (return value, or the bytes written to `stdout`, or "both processes
terminate with the same signal").

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| E1 | `call_fma` | `len == 0` (the one explicit rejection in the file: `if (len == 0) return 0;`) | returns `0`, `data` never dereferenced | `err_e1_call_fma_len_zero` |
| E2 | `call_fma` | `len == 0` **and** `data == NULL` | returns `0`; no fault, because the early return happens before any deref | `err_e2_call_fma_len_zero_null_data` |
| E3 | `call_fma` | `len == 1` (minimum length that reaches the VLA path; `out[0]=0` then overwritten, `return out[0]`) | returns `data[0]` | `err_e3_call_fma_len_one` |
| E4 | `call_fma` | `len < 0` (`-1`, `-2`, `-7`, `-100`, `-65536`, `INT_MIN+1`, `INT_MIN`): declares `int out[len]` with a negative size (which converts to an enormous `size_t`, so the `alloca` moves `rsp` by a wrapped amount), the fill loop runs zero times, then `return out[len-1]` reads outside the frame | **undefined behaviour, measured out of process:**<br>`len` → `libcref.so (-O0)` / `libcref_o2.so (-O2)`<br>`-1` → garbage (`64250770`) / garbage (`889127570`)<br>`-2` → garbage (`32766`) / garbage (`0`)<br>`-7` → SIGSEGV / SIGSEGV<br>`-100` → SIGSEGV / garbage (`32765`)<br>`-65536` → SIGSEGV / SIGSEGV<br>`INT_MIN+1` → SIGSEGV / SIGSEGV<br>`INT_MIN` → SIGSEGV / SIGSEGV<br>The value is stable neither across runs nor across optimisation levels, and whether the process even survives depends on codegen, so **no defined result exists for the Rust to reproduce**. The test asserts the only checkable properties: the Rust is memory-safe here, never faults, and deterministically returns `0`; the C side is still driven out of process so the documented behaviour above stays honest. | documented UB, no value contract | `err_e4_call_fma_negative_len` |
| E5 | `call_fma` | `len > 0` and `data == NULL` (or otherwise invalid) — the C dereferences `data[len-1]` unconditionally | SIGSEGV | `err_e5_call_fma_null_data_positive_len` (out-of-process, asserts both C and Rust die with the same signal) |
| E6 | `call_fma` | `len` so large that `int out[len]; int ones[len]; int zeros[len]` (12·len bytes) overflows the caller's stack, e.g. `len = INT_MAX` ⇒ 24 GiB | SIGSEGV (VLA stack overflow) — UB, asserted only as "the C really does die abnormally", since the Rust heap-allocates and cannot reproduce a *stack* overflow. Differential equality **is** asserted for every `len` that fits: `1, 2, 1024, 100 000, 200 000, 1 000 000, 8 000 000` (the last needing 96 MiB, so the row runs on a 512 MiB thread via `with_big_stack`). | documented UB above the stack limit; exact match below it | `err_e6_call_fma_large_len_documented` |
| E7 | `fma_array` | `len == 0` — loop guard `i < len` fails immediately, so **none** of the four pointers is dereferenced | returns without writing anything; safe even with all-`NULL` pointers | `err_e7_fma_array_len_zero_all_null` |
| E8 | `fma_array` | `len < 0` (`-1`, `INT_MIN`) — same loop guard, still zero iterations | returns without writing anything; safe even with all-`NULL` pointers | `err_e8_fma_array_negative_len_all_null` |
| E9 | `fma_array` | `len > 0` with `out == NULL` | SIGSEGV on the first store | `err_e9_fma_array_null_out` (out-of-process signal comparison) |
| E10 | `fma_array` | `len > 0` with `mul1`/`mul2`/`add == NULL` | SIGSEGV on the first load | `err_e10_fma_array_null_inputs` (out-of-process signal comparison) |
| E11 | `fma_array` | `len` one step past the caller's real buffer (`len = n+1` for an `n`-element buffer) — the C has **no** bounds check, it simply reads/writes one element past the end | reads/writes past the end; with padded buffers the trailing element is written identically by both | `err_e11_fma_array_one_past_end` |
| E12 | `main` | empty stdin — the very first `scanf("%d")` hits end of input and returns `EOF` (≠ 1), so `break` fires with `i == 0`, and `call_fma(data, 0)` takes row E1 | prints `0\n`, exit status 0 | `err_e12_main_empty_input` |
| E13 | `main` | stdin containing only whitespace (`" \t\n\v\f\r"`) — whitespace is consumed by `%d`, then EOF ⇒ input failure, `i == 0` | prints `0\n`, exit status 0 | `err_e13_main_whitespace_only` |
| E14 | `main` | first token is not a number (`"abc"`, `"."`, `","`, `"-"`, `"+"`, `"-x"`, `"e5"`) — `scanf` returns `0` (matching failure) or `EOF`, `i == 0` | prints `0\n`, exit status 0 | `err_e14_main_leading_non_numeric` |
| E15 | `main` | `k` valid integers followed by a non-numeric token (`"7 abc 9"`) — the loop breaks at the bad token, so the trailing input is never read and the answer is the `k`-th integer | prints `data[k-1]` | `err_e15_main_break_mid_stream` |
| E16 | `main` | a number immediately followed by a non-digit (`"0x1f"`, `"3.9"`, `"12abc"`) — `%d` is base-10 only, so it converts the digit prefix, succeeds, then the *next* `%d` fails on the leftover | `"0x1f"`→`0`, `"3.9"`→`3`, `"12abc"`→`12` | `err_e16_main_numeric_prefix` |
| E17 | `main` | a sign with no digits at end of input (`"-"`, `"+"`) — matching/input failure | prints `0\n` | `err_e14_main_leading_non_numeric` |
| E18 | `main` | value one step past `INT_MAX` / `INT_MIN` (`2147483648`, `-2147483649`, `4294967295`) — `%d` converts into a `long` and then narrows; **no** range rejection happens | `2147483648`→`-2147483648`, `-2147483649`→`2147483647`, `4294967295`→`-1` | `err_e18_main_int_range_overflow` |
| E19 | `main` | value one step past `LONG_MAX` / `LONG_MIN`, and far beyond (`9223372036854775808`, `-9223372036854775809`, 29-digit and 400-digit runs) — glibc's `%d` **saturates** at `LONG_MAX`/`LONG_MIN` (sets `ERANGE`, which the C ignores) and then truncates to `int` | saturate-then-truncate: positive overflow → `LONG_MAX as int` = `-1`; negative overflow → `LONG_MIN as int` = `0` | `err_e19_main_long_range_saturation` |
| E20 | `main` | more than 100 integers on stdin — the `for (i = 0; i < 100; i++)` cap stops the loop at `i == 100`; input 101… is never consumed | prints the 100th integer | `err_e20_main_more_than_100` |
| E21 | `main` | exactly 100 integers, then EOF — loop ends on the bound, not on a scan failure | prints the 100th integer | `err_e20_main_more_than_100` |
| E22 | `main` | `int data[100]` is **uninitialised**; when `i == 0` the C calls `call_fma(data, 0)` which returns before touching it | no uninitialised value is ever observable (row E1 guarantees it) | `err_e12_main_empty_input` |
| E23 | `fma_array` / `call_fma` | out-of-range "enum" values across the FFI boundary — the API has no `enum` parameters at all; the only scalar is `int len`, so the analogous inputs are `len ∈ {INT_MIN, -1, 0, 1, INT_MAX}` passed as raw `c_int` | covered by rows E1, E4, E6, E7, E8 | `err_e23_int_len_extremes` |
| E24 | `main` (as `int main(void)`) | any input at all — the function has a single `return 0;` and no error path | exit status / return value always `0` | `err_e24_main_return_value` |

## Divergences found and fixed in the Rust

* **E5 / E9 / E10 (null pointers).** The original translation dereferenced the
  incoming pointers with `*p`, which rustc instruments with a null check
  whenever debug assertions are on. A null pointer therefore produced a Rust
  panic → `SIGABRT` (signal 6) plus a message on stderr, whereas the C dies
  with `SIGSEGV` (signal 11) and prints nothing. `src/fma.rs` now uses
  `std::ptr::read` / `std::ptr::write`, which are not instrumented, so both
  libraries fault identically. Verified by `err_e5_*`, `err_e9_*`, `err_e10_*`,
  which compare the *signal number*, not merely "both failed".
* **E4 (negative `len`).** No fix is possible (the C is UB with no reproducible
  result); the Rust returns a deterministic `0` and the row is documented.

## Notes on surprising-but-correct C behaviour that is replicated

* `%d` accepts a sign with no intervening whitespace, so `"1-2"` is **two**
  successful conversions (`1`, then `-2`) and `main` prints `-2`, not `1`
  (`err_e16_main_numeric_prefix`).
* `%d` is base-10 only, so `"0x1f"` converts as `0` and leaves `"x1f"`
  unconsumed; `main` prints `0`.
* glibc *saturates* out-of-range magnitudes at `LONG_MAX`/`LONG_MIN` before
  narrowing, so `"99999999999999999999999"` → `-1` and
  `"-99999999999999999999999"` → `0`, no matter how many digits follow.
* An arbitrarily long run of leading zeros does **not** saturate:
  `"0"*500 + "123"` → `123`.

## Status

- [x] E1  - [x] E2  - [x] E3  - [x] E4  - [x] E5  - [x] E6
- [x] E7  - [x] E8  - [x] E9  - [x] E10 - [x] E11 - [x] E12
- [x] E13 - [x] E14 - [x] E15 - [x] E16 - [x] E17 - [x] E18
- [x] E19 - [x] E20 - [x] E21 - [x] E22 - [x] E23 - [x] E24
