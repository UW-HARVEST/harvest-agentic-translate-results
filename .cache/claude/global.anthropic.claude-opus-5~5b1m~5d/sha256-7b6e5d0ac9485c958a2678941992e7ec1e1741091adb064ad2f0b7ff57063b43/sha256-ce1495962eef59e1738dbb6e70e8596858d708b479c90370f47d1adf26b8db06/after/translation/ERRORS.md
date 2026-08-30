# ERRORS.md — Phase C error / rejection surface

Mechanically derived from `c_src/src/driver.c` (63 lines, the only C source).

Exhaustive grep of every rejection-ish construct in the C source:

```
$ grep -n 'return\|assert\|break\|if (\|for (\|NULL\|-1\|== 0\|!= 1' c_src/src/driver.c
29:    for (int i = 0; i < len; i++) {          # implicit range guard
35:    if (len == 0) return 0;                  # explicit early-out
41:    for (int i = 0; i < len; i++) {          # implicit range guard
47:    return out[len-1];
53:    for (i = 0; i < 100; i++) {              # hard cap on item count
55:        if (sscanf(in, "%d%zn", &data[i], &nb) != 1) {
56:            break;                           # parse-failure rejection
57:        }
61:    int result = call_fma(data, i);
```

There are **no** `assert()`s, **no** error enums, **no** `RETURN_ERROR` macros,
**no** `return -1` / `return NULL` statements and **no** explicit NULL checks in
this library. Its entire rejection surface consists of the early-out in
`call_fma`, the loop guards in `fma_array`/`call_fma`, and the `sscanf != 1`
parse-failure `break` in `driver` — plus the generic C-API boundaries
(NULL pointers, zero / oversized lengths, one-past-range values) that the task
requires be covered anyway.

`driver`/`call_fma`/`fma_array` take **no enum parameters**, so the
"out-of-range enum value across FFI" class has no instance here; the analogous
"integer parameter with no valid meaning" case is a negative `len`, covered by
rows 4, 12 and 13.

## Error-surface table

Legend for *expected C result*: `ret` = returned value, `out` = printed bytes.
`UB` = C undefined behaviour (documented, see notes below the table).

| #  | function    | trigger (the exact invalid input/condition)                                              | expected C result | test | status |
|----|-------------|------------------------------------------------------------------------------------------|-------------------|------|--------|
| 1  | `call_fma`  | `len == 0` (explicit early-out at driver.c:35) — `data` never dereferenced                | `ret == 0`        | `err01_call_fma_len_zero` | [x] |
| 2  | `call_fma`  | `len == 0` **and** `data == NULL` (early-out fires before any deref)                      | `ret == 0`, no crash | `err02_call_fma_len_zero_null_data` | [x] |
| 3  | `call_fma`  | `len == 1` (boundary: one *past* the rejected `len == 0`) — reads `data[0]` only          | `ret == data[0]`  | `err03_call_fma_len_one_boundary` | [x] |
| 4  | `call_fma`  | `len < 0` (`-1`, `-2`, `INT_MIN`): VLA declared with negative size                        | UB — see note A   | `err04_call_fma_negative_len_ub` | [x] |
| 5  | `fma_array` | `len == 0`: loop guard `i < len` never taken → zero stores, no deref of any pointer        | returns, `out` untouched | `err05_fma_array_len_zero_no_writes` | [x] |
| 6  | `fma_array` | `len == 0` **and** all four pointers `NULL`                                               | returns, no crash | `err06_fma_array_len_zero_all_null` | [x] |
| 7  | `fma_array` | `len < 0` (`-1`, `INT_MIN`): loop guard rejects immediately → zero stores                  | returns, `out` untouched | `err07_fma_array_negative_len_no_writes` | [x] |
| 8  | `fma_array` | `len < 0` **and** all four pointers `NULL`                                                | returns, no crash | `err08_fma_array_negative_len_all_null` | [x] |
| 9  | `driver`    | empty input `""` → `sscanf` returns `EOF` (`!= 1`) on the *first* iteration, `i == 0`, so `call_fma(data,0)` early-outs | prints `"0\n"` | `err09_driver_empty_input` | [x] |
| 10 | `driver`    | input with no numeric prefix at all (`"abc"`, `"+"`, `"-"`, `"x"`, `" "`, `"\n"`, `"0x"` after first token, …) → `sscanf != 1` at `i == 0` | prints `"0\n"` | `err10_driver_no_parseable_number` | [x] |
| 11 | `driver`    | input whose *k*-th token is unparseable (`"1 2 x 4"`, `"1,2"`, `"7 -"`) → `break` at `i == k`, result is the last **successfully** parsed value | prints that value | `err11_driver_partial_parse_break` | [x] |
| 12 | `driver`    | input containing **more than 100** integers → hard cap `i < 100`, only the first 100 consumed, prints the 100th | prints 100th value | `err12_driver_over_100_items_cap` | [x] |
| 13 | `driver`    | integer literal outside `int` range (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) → glibc `%d` range clamping | prints glibc-clamped value | `err13_driver_int_range_overflow` | [x] |
| 14 | `driver`    | `INT_MIN`/`INT_MAX` exactly (`"-2147483648"`, `"2147483647"`) — last in-range values      | prints value verbatim | `err14_driver_int_extremes` | [x] |
| 15 | `fma_array` | signed `int` overflow in `mul1[i]*mul2[i] + add[i]` (e.g. `INT_MAX * 2`, `INT_MIN * -1`)  | UB — see note B   | `err15_fma_array_signed_overflow` | [x] |
| 16 | `call_fma`  | oversized `len` (`100_000`): three VLAs of `len` ints on the stack                         | works, `ret == data[len-1]` | `err16_call_fma_oversized_len` | [x] |
| 17 | `driver`    | oversized input string (100 000 chars of digits/whitespace) — length is not checked anywhere | prints 100th value | `err17_driver_oversized_input` | [x] |
| 18 | `driver`    | exactly 100 integers (boundary: the largest count that is *not* capped)                    | prints 100th value | `err18_driver_exactly_100_items` | [x] |
| 19 | `driver`    | `in == NULL`                                                                              | UB — see note C   | `err19_driver_null_input_ub` | [x] |
| 20 | `call_fma`  | `data == NULL` with `len > 0`; and `fma_array` with `len > 0` and (a) each of `mul1`/`mul2`/`add` individually `NULL` — a different *load* faults in each case — and (b) `out == NULL` with valid inputs, where all loads succeed and the *store* faults | UB — see note C   | `err20_call_fma_null_data_positive_len_ub` | [x] |
| 21 | `call_fma`  | `len` larger than the caller's stack can hold three `int[len]` VLAs (~`RLIMIT_STACK / 12`) | stack exhaustion — see note D | `err21_call_fma_len_beyond_stack_budget` | [x] |

### Generic C-API boundaries (required even though not in the table above)

| #  | area | coverage | test | status |
|----|------|----------|------|--------|
| G1 | out-of-domain integer parameter (the API has **no enum parameters**, so the analogous "value with no valid variant" is an out-of-domain `int len`) — full boundary sweep of `len` around `0`, `INT_MIN`, `INT_MAX` and the buffer size | both libraries agree for every `len` | `err22_no_enum_parameters_int_boundary_sweep` | [x] |
| G2 | exhaustive short inputs to `driver`: **every** 1-byte input (all 255 non-NUL bytes), **every** 2-byte ASCII input (16 129 combinations) and every 3-byte combination over the 16 bytes `%d` actually branches on (4 096) | byte-identical stdout for all 20 480 inputs | `err23_driver_exhaustive_short_inputs` | [x] |

### Note A — `call_fma` with negative `len`

`int out[len]` with `len < 0` is C undefined behaviour (C11 6.7.6.2p5 requires a
VLA size > 0). At `-O0` gcc sign-extends `len`, computes a wild stack
adjustment, then `out[0] = 0` and `return out[len-1]` touch memory outside any
object, so the returned value is *indeterminate garbage* and the call may fault.
There is therefore **no observable C behaviour to reproduce**. The Rust
translation deliberately returns `0` for `len < 0` (no memory touched). The test
`err04_call_fma_negative_len_ub` asserts the *contract we can assert*: the Rust
export is total (returns `0`, never faults, never panics) for `len ∈
{-1, -2, -3, INT_MIN}`, and documents that the C side is UB; it does **not**
compare against the C garbage value. Rows 5–8 (`fma_array`, where negative `len`
is *well defined* because the loop guard simply rejects it) *are* compared
against C.

### Note B — signed integer overflow in `fma_array`

`mul1[i] * mul2[i] + add[i]` on `int` can overflow, which is UB in C. The C
library is compiled by CMake with no `-O`/`-ftrapv`/`-fwrapv` flags, so gcc
emits plain `imul`/`add`, i.e. two's-complement wrapping. The Rust translation
uses `wrapping_mul`/`wrapping_add`, and the test drives thousands of randomized
overflowing triples (including `INT_MIN`/`INT_MAX` corners) through both `.so`s
and requires byte-identical results.

### Note C — NULL pointer dereference

`driver(NULL)` reaches `__isoc99_sscanf(NULL, ...)` and `call_fma(NULL, len>0)`
reaches `mul2[i]`; both dereference a null pointer, i.e. UB that faults with
`SIGSEGV` on this platform *in both implementations*. The tests for rows 19–20
run each library **in a forked child process** and assert that C and Rust agree
on the *observable* outcome (the same termination signal, `SIGSEGV`/11 — not
merely "both failed"), rather than crashing the test harness.

Getting this to hold in **both** cargo profiles required a fix to the
translation. With `-C debug-assertions=on` (the dev profile) rustc inserts a
null-pointer check around a raw-pointer *place* expression, so `*mul1.offset(i)`
aborted with `SIGABRT` where the C build faults with `SIGSEGV`; and
`<*const T>::offset` additionally carries a debug-checked in-bounds
precondition. `fma_array` therefore uses `wrapping_offset` plus
`core::ptr::read`/`core::ptr::write`, which lower to the same instructions and
carry no such checks, so the NULL-pointer UB now dies identically to C under the
dev **and** the release profile. `driver` uses `wrapping_add` on the cursor for
the same reason.

### Note D — `len` beyond the stack budget

`call_fma` declares three `int[len]` VLAs (~`12 * len` bytes of the *caller's*
stack) and never checks `len` against the available stack. Measured against the
C build with an 8 MiB stack: `len = 690_000` still returns the right answer,
`len = 700_000` dies with `SIGSEGV`. This is resource exhaustion rather than a
computed result, so there is no value for the Rust side to reproduce; the Rust
translation allocates the three arrays on the **heap** and keeps working. This is
a documented, deliberate deviation, and `err21_call_fma_len_beyond_stack_budget`
pins down both halves of it: the libraries agree for every `len` that fits the
stack (tested at 600 000 and 650 000 on an explicitly 8 MiB stack), and past the
boundary the C side really is a stack fault while the Rust side returns
`data[len-1]`.

Because `libtest` runs each test on a thread with a 2 MiB stack, all large-`len`
rows are executed on a thread with an explicitly sized stack
(`common::on_big_stack` / `common::on_stack_of`); otherwise the tests would be
measuring the harness's thread stack instead of the translation.
