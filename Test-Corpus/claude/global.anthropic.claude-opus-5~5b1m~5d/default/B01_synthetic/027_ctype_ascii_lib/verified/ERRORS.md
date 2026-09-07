# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping every rejection construct in the C source:

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|return [A-Z_]*ERR|assert|errno|if *\(|switch|<|>|==|!=' c_src/src/driver.c c_src/include/driver.h
```

Result: `c_src/src/driver.c` contains **no** `return` statements at all (the
function is `void`), **no** `assert`, **no** `errno` use, **no** `if`/`switch`,
**no** null checks, **no** range checks, **no** error enums, and **no**
min/max constants. `driver.h` declares no error codes.

The C function `void driver(char c)` therefore has an *empty* intrinsic error
surface: every one of the 256 representable `char` values is accepted and
produces output. The only observable result is the byte stream written to
`stdout`, so "same error/rejection" degenerates to "same bytes on stdout, same
(void) return, no crash".

The rows below are the boundary/adversarial conditions that DO exist for this
API — the extremes of the single argument's domain plus the generic FFI
boundaries the instructions require. Each row has a differential test that
asserts C and Rust behave identically (byte-identical stdout, normal return).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `driver` | `c = 0` (NUL — the C-string terminator; `printf("%c")` writes a bare NUL byte, not "nothing") | no error; 14 lines; `iscntrl` bit `2` set; `to lower`/`to upper` each emit byte `0x00` | `err_nul_byte` | ✅ |
| 2 | `driver` | `c = -128` (`CHAR_MIN`, most-negative `char`; `(int)c` indexes glibc's table below 0) | no error; all 12 class bits `0`; `%c` emits byte `0x80` | `err_char_min` | ✅ |
| 3 | `driver` | `c = 127` (`CHAR_MAX`, DEL — last positive `char`, `iscntrl` but not `isprint`) | no error; `control: 2`, `printing: 0`, `graphical: 0`; `%c` emits `0x7f` | `err_char_max` | ✅ |
| 4 | `driver` | `c = -1` (i.e. `0xFF`; the value `EOF` aliases to when a byte is sign-extended — classic ctype misuse) | no error; all class bits `0`; `%c` emits byte `0xFF` | `err_minus_one` | ✅ |
| 5 | `driver` | every negative `char`, `-128 ..= -1` (the whole out-of-`unsigned char`-range index region glibc's negative table half covers) | no error; all 12 class bits `0` for each; `%c` emits the original byte | `err_all_negative_chars` | ✅ |
| 6 | `driver` | one step past the ASCII range: `c = 0x80` passed as an unsigned byte (wraps to `-128` in signed `char`) and `c = 0x7F` | no error; C and Rust agree; `0x80` behaves exactly like `-128` (identical output) | `err_one_past_ascii` | ✅ |
| 7 | `driver` | an out-of-range "enum-like" `int` crossing the FFI boundary: the exported symbol is re-typed as `extern "C" fn(c_int)` and called with `256`, `257`, `-129`, `-256`, `1000`, `-1000`, `65536`, `65663`, `i32::MIN`, `i32::MAX`, values whose low byte is `'A'`/`'z'`/`'0'`, plus 96 randomized bytes lifted into four high-garbage patterns (`0x0000_01xx`, `0xFFFF_FFxx`, `0x1234_56xx`, `0xDEAD_BExx`). C enums/`char` params accept any `int`; the callee narrows to the low 8 bits and there is no valid-variant check | no error; output equals that of `(char)value`, i.e. the low byte reinterpreted as signed | `err_out_of_range_int_truncation` | ✅ |
| 8 | `driver` | oversized/repeated invocation: `driver` called for all 256 values back to back in one process (state leakage through the repeated `setlocale(LC_ALL,"C")` call) | no error; output is the concatenation of the individual per-value outputs; `setlocale` is idempotent | `err_repeated_calls_no_state_leak` | ✅ |
| 9 | `driver` | interleaved C-then-Rust and Rust-then-C calls in one process (Rust's own `"C"`-locale tables must not be perturbed by, nor perturb, glibc's `setlocale`) | no error; each call's output independent of ordering | `err_interleaved_c_and_rust` | ✅ |

## Notes on the two generic boundaries that do not apply

* **Null pointers** — `driver` takes no pointer arguments and returns no
  pointer, so there is no null-pointer row to construct. (Row 1 covers the NUL
  *character*, which is the nearest analogue.)
* **Zero / oversized lengths** — there is no length, size, or count parameter
  in the API. Row 8 covers the "oversized" analogue (maximum number of calls,
  i.e. the entire input domain in one process).
* **Out-of-range enum values** — the API has no `enum` parameter; row 7 covers
  the equivalent case of an `int` with no valid `char` representation crossing
  the FFI boundary.

## Verification result

All 10 rows pass (`tests/phase_c_error_paths.rs`, single `#[test]`
`phase_c_all_error_rows` which runs every row and reports each on stderr):

```
=== ERRORS.md: 10 rows ===
  [ 1/10] err_nul_byte ... ok
  [ 2/10] err_char_min ... ok
  [ 3/10] err_char_max ... ok
  [ 4/10] err_minus_one ... ok
  [ 5/10] err_all_negative_chars ... ok
  [ 6/10] err_one_past_ascii ... ok
  [ 7/10] err_out_of_range_int_truncation ... ok
  [ 8/10] err_repeated_calls_no_state_leak ... ok
  [ 9/10] err_interleaved_c_and_rust ... ok
  [10/10] err_generic_boundary_sweep_exhaustive ... ok
=== ERRORS.md: all 10 rows passed ===
```

Verified under the default feature set, `--no-default-features`, and both the
release and dev cdylib profiles.

### Divergence found and fixed by row 7

Row 7 caught a real bug. Passing a full `int` whose low byte is the intended
`char` (a mismatched prototype / an out-of-range enum value — exactly the case C
accepts silently):

* **C**: gcc compiles `void driver(char c)` into a callee that reads only the
  low 8 bits of the argument register and sign-extends them; the upper bits are
  unspecified by the SysV ABI and are ignored. `driver(256)` therefore behaves
  as `driver((char) 0)` → `control: 2`, `to lower`/`to upper` emit `\0`.
* **Rust (before the fix)**: `extern "C" fn(c_char)` carries LLVM's `signext`
  parameter attribute, so the optimiser *assumed* the upper register bits were a
  valid sign extension, folded away the `-128 ..= 255` range predicates in
  `src/ctype.rs`, and read **past the end of the ctype tables**. `driver(256)`
  printed garbage (`alphabetic: 1024`, `lowercase: 512`, `uppercase: 256`,
  `digit: 2048`, `to lower: \x80`, `to upper: \xd2`) — an out-of-bounds read
  reachable from an ordinary FFI caller.

Fixes applied to the Rust (the C was not touched):

1. `src/lib.rs` — the exported `driver` now takes a `c_int` and narrows it
   itself with `(c as u8) as c_char`, which is what the compiled C callee does.
   This is ABI-identical for every conforming caller (a `char` argument occupies
   the same register) and reproduces C's truncation for non-conforming ones.
2. `src/ctype.rs` — `class`, `tolower` and `toupper` now use checked
   `slice::get` lookups instead of a range predicate plus an unchecked index, so
   no table read can go out of bounds even if an out-of-range value ever reaches
   them again.
