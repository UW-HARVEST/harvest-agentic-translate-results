# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

## Mechanical derivation

Every branch/rejection construct in the entire C library, found by grepping the
whole of `c_src` (only two non-generated files exist: `src/driver.c`,
`include/driver.h`):

```
$ grep -nE 'return|assert|NULL|errno|error|ERROR|exit|abort|if|else|switch|case|while|for|\?|<|>|==|!=|#if' \
      src/driver.c include/driver.h | grep -v '://' | grep -v Copyright
src/driver.c:26:#include <stdio.h>
src/driver.c:27:#include <string.h>
src/driver.c:30:    for (int i = 0; i < len; i++) {      <-- the ONLY conditional in the library
include/driver.h:24:#ifndef DRIVER_H_                    <-- include guard, not a runtime branch
include/driver.h:29:#endif //DRIVER_H_
```

Counts, all verified to be zero:

| construct searched for | occurrences |
|------------------------|-------------|
| `return <value>` / error return | 0 (`driver` and `print_hex` are both `void`, no `return` statement at all) |
| `RETURN_ERROR`-style macro | 0 |
| error enum / status code type | 0 |
| `assert` / `static_assert` | 0 |
| `NULL` check | 0 |
| explicit range / bounds check | 0 |
| min/max constant, `#define` limit | 0 |
| `errno` use | 0 |
| `exit` / `abort` / `longjmp` | 0 |
| runtime `if` / `switch` / ternary | 0 |
| `#if` / `#ifdef` affecting code | 0 (only the `DRIVER_H_` include guard) |

**Conclusion: the C library has NO explicit rejection or error path.**
`void driver(float x)` validates nothing, returns nothing, and accepts the
entire 2^32-value domain of `float`. There is therefore no error code or
sentinel to compare — the differential obligation for Phase C becomes
"for every degenerate / boundary / would-be-invalid input, C and Rust must both
accept it and emit byte-identical output, and neither may trap, abort or panic".
Rows below are written against exactly that, one row per distinct
degenerate condition that the C code actually admits.

## Error / rejection surface table

`expected C result` is what the real `c_src/build/libdriver.so` does; each row is
verified differentially against the Rust `.so` in `tests/error_paths.rs`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `print_hex` (`driver.c:30`) | loop guard `i < len` with `len == 0` — the sole conditional in the library, taken zero times | prints only `"\n"`; no hex digits | unreachable from the public API (`driver` always passes `sizeof(float)` == 4); asserted structurally: C output is *always* exactly 8 hex digits + `\n`, never the empty-body form → `err_e1_loop_guard_never_degenerates` | [x] |
| E2 | `print_hex` (`driver.c:30`) | loop guard with negative `len` (`int len` is signed, so `len < 0` is representable) | prints only `"\n"`; pointer never dereferenced, no out-of-bounds read | unreachable from the public API for the same reason as E1; same structural assertion → `err_e1_loop_guard_never_degenerates` | [x] |
| E3 | `driver` | quiet NaN (`0x7fc00000`) — no NaN rejection, and no canonicalisation is permitted | accepts; prints `0000c07f` (LE byte order) | `err_e3_quiet_nan` | [x] |
| E4 | `driver` | **signalling** NaN (`0x7f800001`) — passing an sNaN through the `float` ABI must not quieten it | accepts; prints `0100807f` | `err_e4_signalling_nan` | [x] |
| E5 | `driver` | negative NaN (`0xffc00000`) / NaN with sign bit set | accepts; prints `0000c0ff` | `err_e5_negative_nan` | [x] |
| E6 | `driver` | NaN with a non-zero mantissa payload (`0x7fc0dead`, `0x7fabcdef`) — payload bits must survive verbatim | accepts; prints the exact payload bytes | `err_e6_nan_payloads` | [x] |
| E7 | `driver` | `+inf` (`0x7f800000`) | accepts; prints `0000807f` | `err_e7_infinities` | [x] |
| E8 | `driver` | `-inf` (`0xff800000`) | accepts; prints `000080ff` | `err_e7_infinities` | [x] |
| E9 | `driver` | negative zero (`0x80000000`) — must NOT be normalised to `+0.0` | accepts; prints `00000080`, distinct from `+0.0`'s `00000000` | `err_e9_signed_zeros` | [x] |
| E10 | `driver` | positive zero (`0x00000000`) | accepts; prints `00000000` | `err_e9_signed_zeros` | [x] |
| E11 | `driver` | smallest positive subnormal (`0x00000001`) and largest subnormal (`0x007fffff`), and their negatives — must not be flushed to zero | accepts; prints the exact bits | `err_e11_subnormals` | [x] |
| E12 | `driver` | `FLT_MIN` (`0x00800000`), `FLT_MAX` (`0x7f7fffff`), `-FLT_MAX`, `FLT_EPSILON` — the documented range extremes, and one step past each (`FLT_MAX` + 1 ulp == `+inf`, `FLT_MIN` − 1 ulp == largest subnormal) | accepts every one; prints the exact bits | `err_e12_range_extremes_and_one_past` | [x] |
| E13 | `driver` | every one of the 2^32 bit patterns is a legal argument, including the whole NaN space — i.e. there is no "out-of-range" value at all | accepts all; output is a pure function of the 32 argument bits | `err_e13_exhaustive_boundary_bitpatterns`: exhaustive over all 65 536 **high** halves × low half ∈ {0x0000, 0x0001, 0x8000, 0xffff}, exhaustive over all 65 536 **low** halves × high half ∈ {0x0000, 0x7f80, 0x7fc0, 0xffc0}, plus 196 608 uniform random 32-bit patterns (≈720 k calls per implementation) | [x] |
| E14 | `driver` | repeated back-to-back invocation (state / buffering carry-over: the C `printf` writes into the shared `stdout` `FILE*`, so a translation that used Rust's own `std::io::stdout` buffer would interleave differently) | accepts; output of N calls is the exact concatenation of the N single-call outputs, in call order | `err_e14_repeated_calls_no_state` | [x] |

## Generic C-API boundaries that do not apply, with justification

The task's generic checklist (null pointers, zero/oversized lengths, one-past
valid range, out-of-range enum values) is enumerated here so the omissions are
explicit rather than accidental:

| generic boundary | applicable? | justification |
|------------------|-------------|---------------|
| null pointer argument | **no** | the only public function is `void driver(float)`. It has no pointer parameter, and `driver.h` exposes no other function. There is no pointer that a caller could pass as `NULL`. (`print_hex`'s `unsigned char *p` is `static`/internal and is only ever handed the address of a live 4-byte stack buffer.) |
| zero length / oversized length | **no** | no length, size or count parameter exists in the public API. `print_hex`'s `len` is always the compile-time constant `sizeof(float)`; rows E1/E2 record the degenerate guard anyway. |
| out-of-range enum value across FFI | **no** | the library declares no `enum`, no `typedef`, no struct and no integer mode/flag parameter. There is no int-valued parameter at all, so there is no "value with no valid variant" to pass. |
| one step past a documented valid range | **yes — covered** | the "range" of a `float` argument is the whole IEEE-754 binary32 space; one step past the finite extremes is `±inf`, one step below `FLT_MIN` is a subnormal. Rows E7, E8, E11, E12 cover these. |
| output-buffer / return-value truncation | **no** | `driver` returns `void` and writes to no caller-supplied buffer; its only observable effect is `stdout`. |

## Phase C gate

All 14 rows are checked `[x]` — each has a differential test in
`translation/tests/error_paths.rs` that loads both `.so`s via `libloading`,
constructs the exact condition, and asserts byte-identical output (and, for
E1/E2, the structural invariant). Phase D may proceed.
