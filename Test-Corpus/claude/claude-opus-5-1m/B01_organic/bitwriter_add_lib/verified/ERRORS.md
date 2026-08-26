# ERRORS.md — Error-surface table (Phase A / Phase C)

Derived by mechanically grepping **every** rejection / error construct in the C
source (`c_src/src/lib.c`, `c_src/include/lib.h`).

## Mechanical grep results

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|if *\(|<|>|MIN|MAX' c_src/src/lib.c
5:    const tflac_uint mask = (18446744073709551615UL) << 1;   # constant, not a check
8:    val <<= ((8 * sizeof(tflac_uint)) - bits);                # shift, not a check
11:    while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {   # loop guard
13:        b = b > bits ? bits : b;                              # min(), not a rejection
23:    return 0;                                                 # the ONLY return
```

Findings, stated precisely:

* **`return` statements: exactly one** — `return 0;` at line 23. It is
  unconditional and on the sole exit path.
* **`assert` / `RETURN_ERROR` / `errno` / `exit` / `abort`: none.**
* **`return -1` / `return NULL` / error enums: none.** The header declares no
  error enum and no status type; the return type is a bare `int`.
* **Null checks: none.** `bw` is dereferenced unconditionally at line 9.
* **Range / bounds checks on `bits`: none.** `bits` is used directly as a shift
  count and as a loop-arithmetic operand with no validation.
* **`MIN` / `MAX` constants: none.** The only named constant is `mask`
  (`0xFFFFFFFFFFFFFFFE`), which is data, not a limit.
* **`pos`, `len`, `buffer`: never read and never written** — so there is no
  capacity check and no overflow rejection, even though the struct has the
  fields a bounds check would use.
* **The `i < 100` loop guard is not an error path.** It caps an otherwise
  infinite loop and then *falls through to `return 0`*; it does not report
  failure. It is therefore a **valid-path configuration** row
  (`CONFIGS.md` #16–#18), not an error row.

## The error-surface table

`bitwriter_add` is a **total function with a single unconditional
`return 0`**: there is no input for which the C code reports an error. The
table below therefore enumerates the rejection surface as the C code actually
defines it — one row per distinct *candidate* rejection trigger that a reader
might expect, together with what the C **actually** does — plus the generic FFI
boundaries Phase C mandates.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `bitwriter_add` | any valid `bw`, any `bits`, any `val` (nominal success path) | returns `0`; no error signalling exists |
| 2  | `bitwriter_add` | `bits == 0` (degenerate width; makes `64 - bits == 64`, an out-of-range shift count) | returns `0`; shift count masked to 6 bits ⇒ `val <<= 0`; state updated, no rejection |
| 3  | `bitwriter_add` | `bits == 64` (`bits` exactly equal to `8*sizeof(tflac_uint)`) | returns `0`; `val <<= 0`; loop entered whenever `bw->bits + 64 >= 64` (always) |
| 4  | `bitwriter_add` | `bits == 65` (one step past the documented/implied maximum width of 64) | returns `0`; **no range rejection**; `64u32 - 65 = 0xFFFFFFFF`, masked to shift 63 |
| 5  | `bitwriter_add` | `bits == 0xFFFFFFFF` (`UINT32_MAX`, grossly oversized width) | returns `0`; no rejection; `bw->tot` and `bits` arithmetic wrap mod 2^32 |
| 6  | `bitwriter_add` | `bits` large enough that `bw->bits + bits` **wraps** past 2^32 to a value `< 64` (e.g. `bw->bits = 64`, `bits = 0xFFFFFFC0`) | returns `0`; the 32-bit wrapped sum makes the `while` guard **false**, so the loop is skipped entirely — no rejection |
| 7  | `bitwriter_add` | `bw->bits == 64` (already "full"; `63 - bw->bits` underflows to `0xFFFFFFFF`) | returns `0`; underflow is **not** rejected; `b = min(0xFFFFFFFF, bits) = bits` |
| 8  | `bitwriter_add` | `bw->bits == 0xFFFFFFFF` (maximally out-of-range accumulator width) | returns `0`; no rejection; `bw->bits += b` wraps mod 2^32 |
| 9  | `bitwriter_add` | `bw->bits == 63, bits == 1` ⇒ `b` computes to `0`, loop makes no progress and only the `i < 100` cap stops it | returns `0` after exactly 100 spins; the cap is **not** reported as an error |
| 10 | `bitwriter_add` | `bw->tot` at `0xFFFFFFFF` plus `bits >= 1` ⇒ `tot` counter overflows | returns `0`; overflow silently wraps, no rejection |
| 11 | `bitwriter_add` | `bw->pos > bw->len` / `bw->len == 0` (a capacity violation an ordinary bit-writer *would* reject) | returns `0`; C never reads `pos`/`len`, so there is **no** capacity error |
| 12 | `bitwriter_add` | `bw->buffer == NULL` while `bw` itself is valid (no output buffer at all) | returns `0`; C never touches `buffer`, so **no** null-buffer error |
| 13 | `bitwriter_add` | `bw == NULL` (null struct pointer — generic FFI boundary) | **undefined behaviour**: unconditional `bw->tot += bits` at line 9 dereferences address `0x14` ⇒ process dies with `SIGSEGV`. No graceful error code exists. Rust must fail the same way (also `SIGSEGV`), not panic with a Rust message and not silently succeed |
| 14 | `bitwriter_add` | `bw` non-null but misaligned (odd address) — generic FFI boundary | returns `0`; x86-64 permits unaligned 8-byte access, so both sides behave identically with no rejection |
| 15 | `bitwriter_add` | out-of-range **enum** value crossing the FFI boundary | **not applicable**: the C API declares no `enum` and no mode/flag parameter — `bits` (`tflac_u32`) and `val` (`tflac_u64`) already accept their full integer ranges, which rows 2–8 cover exhaustively at the boundaries |

## Consequence for Phase C

The only *observable* "rejection" in the whole library is row 13's `SIGSEGV`.
Every other row must be shown to **not** be rejected — i.e. both
implementations must return exactly `0` *and* produce byte-identical struct
state. A Rust translation that "helpfully" validated `bits <= 64`, saturated an
overflow, or panicked on the `63 - bw->bits` underflow would diverge on rows
2–10 even though the C reports no error, so each row is checked for
`rc == 0` **and** full 32-byte struct equality.

Row 13 is verified out-of-process (re-exec of the test binary) so that the
expected `SIGSEGV` does not abort the test run; both `.so`s must die with the
same signal.

## Phase C results — every row has a passing differential test

| # | test in `tests/phase_c_errors.rs` | status |
|---|-----------------------------------|--------|
| 1 | `err01_nominal_returns_zero` | [x] |
| 2 | `err02_bits_zero_not_rejected` | [x] |
| 3 | `err03_bits_equal_width_not_rejected` | [x] |
| 4 | `err04_bits_one_past_max_not_rejected` | [x] |
| 5 | `err05_bits_uint32_max_not_rejected` | [x] |
| 6 | `err06_guard_sum_wraparound_not_rejected` | [x] |
| 7 | `err07_bwbits_64_underflow_not_rejected` | [x] |
| 8 | `err08_bwbits_uint32_max_not_rejected` | [x] |
| 9 | `err09_iteration_cap_is_not_an_error` | [x] |
| 10 | `err10_tot_overflow_not_rejected` | [x] |
| 11 | `err11_capacity_violation_not_rejected` | [x] |
| 12 | `err12_null_buffer_not_rejected` | [x] |
| 13 | `err13_null_pointer_same_fatal_signal` (+ `err13_null_pointer_child_worker`) | [x] |
| 14 | `err14_misaligned_struct_pointer` | [x] |
| 15 | `err15_no_enum_params_full_integer_domain` | [x] |
| — | `err_generic_boundaries_one_step_past` (generic ±3 sweep around 0/1/63/64/100/128/2^31/2^32-1) | [x] |

## Divergences these rows actually found (and the fixes)

Phase C caught **two real translation bugs**, both in the *pointer* handling
rather than the arithmetic. The original Rust began with

```rust
let bw: &mut tflac_bitwriter = unsafe { &mut *bw };
```

Forming a Rust reference imposes requirements the C `mov` instructions do not:

| row | input | C behaviour | original Rust behaviour | fix |
|-----|-------|-------------|--------------------------|-----|
| 14 | `bw` at an odd address | returns `0`, updates state | `misaligned pointer dereference` → **`SIGABRT`** | access fields via `addr_of_mut!` + `read_unaligned`/`write_unaligned`, never forming a reference |
| 13 | `bw == NULL` | dies with **`SIGSEGV`** (signal 11) | Rust null check → **`SIGABRT`** (signal 6) | same fix; because the first touch is `bw->tot` at offset 20 the faulting address is `0x14`, non-null, so a real `SIGSEGV` is raised exactly as in C |

Both were invisible to happy-path testing and to symbol-parity checking: the
`.so` exported the right symbol and every arithmetic case matched. They are
exactly the class of bug this phase exists to find.

Note that the `SIGABRT` only appeared in the `dev` profile (Rust's UB checks are
compiled out in `release`), so a release-only test run would have missed them —
which is why `run_all.sh` exercises both profiles.
