# ERRORS.md — Error-surface table (Phase A) / error-path tests (Phase C)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`,
not from docs or assumptions.

## Mechanical grep of the whole C source for rejection machinery

Counts over every non-license line of every file in `c_src` (`driver.c`, `driver.h`):

| construct grepped | hits |
|---|---|
| `return` (incl. `return -1`, `return NULL`, `RETURN_ERROR`) | 0 |
| `assert` | 0 |
| `NULL` | 0 |
| `errno` | 0 |
| `exit` | 0 |
| `abort` | 0 |
| `if` | 0 |
| `switch` | 0 |
| `goto` | 0 |
| `#if` / `#ifdef` | 0 |
| `malloc` / `free` | 0 |

The entire library is:

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) {
        printf("%d %d\n", i, j);
    }
}
```

Consequences for the error surface, all read straight off that body:

* the return type is `void` → there is **no error code, no sentinel and no
  out-parameter** through which failure could ever be reported;
* the only parameter is a by-value `int` → there is **no pointer to null-check**
  and **no length/size/count to range-check**;
* there is **no enum parameter** → the "out-of-range enum value across FFI"
  class does not exist for this ABI (documented here so the omission is
  deliberate, not an oversight);
* there is **no explicit validation of `x` at all**. The *only* thing the code
  does with `x` is the loop guard `i < x`. So the library's complete
  "rejection" behaviour is: **`x <= 0` produces zero output and returns
  normally** — the loop body is never entered.

Because the rejection surface is defined purely by the guard `i < x`, the rows
below enumerate every distinct way that guard can reject, plus the generic
C-API boundary values that Phase C must cover anyway.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | `x == 0` — guard `0 < 0` false on the first evaluation (the empty / "zero length" input) | returns normally, writes **0 bytes**, `stdout` untouched | `e1_zero_is_rejected_with_empty_output` | [x] |
| E2 | `driver` | `x == -1` — one step past the low end of the "produces output" range | returns normally, writes **0 bytes** | `e2_minus_one_empty_output` | [x] |
| E3 | `driver` | `x < 0`, randomized negative values (property test, fixed seed) | returns normally, writes **0 bytes**, for every value | `e3_random_negative_empty_output` | [x] |
| E4 | `driver` | `x == INT_MIN` (`-2147483648`) — extreme low boundary of the parameter's domain; also the value whose negation overflows | returns normally, writes **0 bytes** | `e4_int_min_empty_output` | [x] |
| E5 | `driver` | `x == INT_MIN + 1` — one step in from the extreme low boundary | returns normally, writes **0 bytes** | `e5_int_min_plus_one_empty_output` | [x] |
| E6 | `driver` | `x == 1` — one step past the rejection boundary on the accepting side (first value the guard admits) | returns normally, writes exactly `"0 0\n"` | `e6_one_is_the_accept_boundary` | [x] |
| E7 | `driver` | out-of-range / garbage bit pattern passed where a C `enum`-like small int is expected: `x` = `0x7FFFFFFF`-family values reinterpreted from `u32` (e.g. `0x80000000u32 as i32`, `0xFFFFFFFFu32 as i32`) — a C API accepts any `int` bit pattern | `0x80000000 -> INT_MIN` (0 bytes), `0xFFFFFFFF -> -1` (0 bytes); no trap, no diagnostic | `e7_raw_bit_patterns_across_ffi` | [x] |
| E8 | `driver` | repeated rejection: `driver(0)` / `driver(-5)` called many times in a row, interleaved with an accepting call | no residual state; the accepting call's output is unaffected | `e8_rejections_leave_no_residual_state` | [x] |
| E9 | `driver` | rejecting call while `fd 1` is a **pipe** instead of a regular file (different glibc buffering mode) | still 0 bytes, no partial/stale flush | `e9_zero_output_on_pipe` | [x] |
| E10 | `driver` | ABI edge: caller puts a **64-bit** value in the argument register (mismatched prototype / widened out-of-range "enum"), e.g. `0x1234_5678_0000_0005`, `0x0000_0007_FFFF_FFFF` | only the low 32 bits are read (`5`, `-1`, ...); identical byte stream to passing that `int` directly | `e10_high_register_bits_ignored_identically` | [x] |

### Explicitly-not-applicable generic boundaries (recorded so they are not blind spots)

| generic boundary | applicable? | why |
|---|---|---|
| null pointer argument | **no** | `driver` takes no pointer |
| zero length / oversized length | **covered by E1/E3/E4** | there is no separate length parameter; `x` *is* the count, and `x<=0` is the "zero/negative length" case |
| out-of-range enum across FFI | **no valid enum exists** | no enum in the ABI; the equivalent "any int bit pattern" case is E7 |
| error code / sentinel comparison | **no** | `void` return; the observable is the byte stream, which E1–E10 compare byte-for-byte |
| `x == INT_MAX` actually executed | **infeasible, documented** | would require 2^31−1 iterations (~47 GB of output) per implementation; far beyond the 600 s budget. The same code path (guard + `wrapping_add`) is exercised by E4/E5/E7 and by the large-`x` rows of `CONFIGS.md`. |

## Phase C completion

All 10 rows have a differential test that constructs the exact condition, calls
**both** the C `.so` and the Rust `.so` through `libloading`, and asserts the
observable result (the exact byte stream on `fd 1`) is identical — not merely
"both did something".
