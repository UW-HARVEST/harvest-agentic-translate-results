# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping **every** `return`,
`if (`, `!`/`!!` truthiness test, comparison against a min/max constant, and
every implicit-guard branch. Result of the grep sweep:

* `RETURN_ERROR`-style macros: **none**
* `assert` / `NDEBUG`: **none**
* `return -1` / `return NULL` / error enums: **none**
* explicit null-pointer checks: **none** (the two pointer-taking functions
  dereference unconditionally)
* guarded/rejecting branches and clamping range checks: the 12 rows below

This library has **no error-code channel**. Its "rejection" behaviour is
expressed as (a) *skip the operation and return the unchanged state*,
(b) *clamp the value to a threshold constant*, and (c) *substitute the `0777`
sentinel*. Each distinct such branch is one row.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `divide_multiplier` | `b == 0` (division-by-zero rejection; `if (b != 0)` at lib.c:54 is false) | division **skipped**; `multiplier` unchanged; `operation_count` still incremented; returns the unchanged `multiplier` |
| E2 | `divide_multiplier` | `b == INT_MIN`, or any `b` with `|b| > |multiplier|` | truncating-toward-zero integer division performed, typically yielding `0`; returns new `multiplier` |
| E3 | `divide_multiplier` | `multiplier == INT_MIN && b == -1` (signed-division overflow) | **hardware trap**: on x86-64 the emitted `idiv` raises `SIGFPE` and the process dies. Not reachable from `findrep` (which always passes `b == 2`). The Rust translation emits the same `idiv` via the `c_idiv` helper in `src/lib.rs`, so it dies with the identical signal. Verified in an isolated child process by `tests/error_paths.rs::e3_int_min_div_minus_one_same_signal` (both `SIGFPE`, signal 8). |
| E4 | `find_and_replace_char` | `search_char` not present in `str` (`memchr` returns `NULL`, `if (found)` at lib.c:69 false) | **no write** performed; string left byte-identical |
| E5 | `find_and_replace_char` | `str` is the empty string `""` (`strlen == 0` → `memchr(p, c, 0)` is always `NULL`) | no write; `str[0]` stays `'\0'` |
| E6 | `find_and_replace_char` | `search_char == 0` (searching for the terminator, which lies *outside* the `strlen(str)` window) | never found → no write |
| E7 | `find_and_replace_char` | `search_char` outside `unsigned char` range (e.g. `256`, `321`, `-1`, `INT_MIN`, `INT_MAX`) — C `memchr` converts the `int` to `unsigned char`, so only the low 8 bits are compared | matches iff `(unsigned char)search_char` occurs in the string; e.g. `search_char == 0x141` behaves exactly like `'A'` (`0x41`) |
| E8 | `find_and_replace_char` | `str == NULL` | undefined behaviour → `strlen(NULL)` dereferences address 0 and the process dies with `SIGSEGV`. Rust does the same (`c_strlen` reads through `core::ptr::read`, which does **not** pick up rustc's `debug_assertions` null check, so the dev and release `.so`s both segfault exactly like the C rather than aborting). Verified in an isolated child process by `e8_null_str_find_and_replace_same_signal` (both signal 11). |
| E9 | `validate_and_normalize` | `0 < value < 0100` (=64): below the lower threshold (lib.c:82) | **clamped**, returns `0100` (64) |
| E10 | `validate_and_normalize` | `value > 0777` (=511): above the upper threshold (lib.c:84) | **clamped**, returns `0777` (511) |
| E11 | `validate_and_normalize` | `value <= 0` (`is_nonzero && value > 0` at lib.c:81 is false — covers `0`, all negatives, `INT_MIN`) | **no clamping at all**; returns `value` verbatim, including out-of-range negatives |
| E12 | `process_octal_string` | `dest == NULL` | undefined behaviour → `strcpy` to `NULL` writes to address 0 and the process dies with `SIGSEGV`. Rust matches (`c_strcpy_bytes` writes through `core::ptr::write`; same reasoning as E8). Verified in an isolated child process by `e12_null_dest_process_octal_string_same_signal` (both signal 11). |
| E13 | `findrep` | the computed `result` is exactly `0` (`!result_exists` at lib.c:169) | returns the sentinel `0777` (511) instead of `0` |
| E14 | `findrep` | all four params `0` (`active_params == 0`, so *both* the `mode_add` and `mode_multiply` dispatches at lib.c:132/137 are skipped) | only the `memchr` offset `9`, the `both_active` term and the `operation_count * 010` term contribute |

## Generic FFI-boundary boundaries also covered by the Phase C tests

Beyond the table rows (per the task's explicit instruction), `tests/error_paths.rs`
also drives:

* **zero and extremal integer widths**: `0`, `1`, `-1`, `INT_MIN`, `INT_MAX`,
  `INT_MIN+1`, `INT_MAX-1` on every `int` parameter of all 8 exports.
* **one step past every documented range**: `0100-1`, `0100`, `0100+1`,
  `0777-1`, `0777`, `0777+1` for `validate_and_normalize` and for each
  `findrep` param; `0150-1/0150/0150+1` for the `accumulator` gate;
  `0100-1/0100/0100+1` for the `multiplier` gate.
* **out-of-range "enum" values across the FFI boundary**: `findrep`'s dispatch
  is driven by `active_params` compared against the octal *mode* constants
  `01/02/03/04`. There is no C `enum` type, but the same bug class applies to
  `find_and_replace_char`'s `int search_char`, which is a `char`-domain value
  passed as a full-width `int`. Every out-of-domain integer (`256`, `-1`,
  `0x141`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFF00`) is passed across the FFI
  boundary and the truncation-to-`unsigned char` behaviour is compared
  byte-for-byte (row E7).
* **signed overflow / wraparound** in the accumulator and multiplier
  (`accumulator += a + b`, `multiplier *= a * b`) driven to and past
  `INT_MAX`/`INT_MIN`, since the C is built at `-O0` and wraps two's-complement.
* **NUL-boundary behaviour** for the string functions: writes are checked over
  the *whole* 256-byte destination buffer (not just up to the terminator) so any
  difference in trailing bytes or terminator placement is caught.

## Row status

**All 14 rows pass a differential test against both `.so`s.** There are no
accepted divergences.

Rows E1, E2, E4, E5, E6, E7, E9, E10, E11, E13, E14 are ordinary value
comparisons. Rows **E3, E8, E12** terminate the process, so they are exercised
in an isolated child process (`crash_worker` re-execs the test binary) and the
*exact termination signal* is compared rather than a return value:

| row | C | Rust | verdict |
|-----|---|------|---------|
| E3  | `SIGFPE` (8)  | `SIGFPE` (8)  | identical |
| E8  | `SIGSEGV` (11) | `SIGSEGV` (11) | identical |
| E12 | `SIGSEGV` (11) | `SIGSEGV` (11) | identical |

Two translation changes were required to reach that state, both driven by these
rows:

1. `divide_multiplier` originally used `wrapping_div`, which silently returned
   `INT_MIN` where the C trapped. Replaced with `c_idiv`, a helper that emits
   the same `idiv` instruction the C compiler emits.
2. The string helpers originally used raw place-expression derefs (`*p`), which
   pick up rustc's `debug_assertions` null-pointer check and made the **dev**
   profile abort (`SIGABRT`) where the C segfaults. Replaced with
   `core::ptr::read` / `core::ptr::write`, so dev and release behave identically
   to the C.

Both changes are pinned by mutation tests in `mutation_check.sh`
(`c_idiv -> wrapping_div`, `raw deref in c_strlen`, `raw deref in
c_strcpy_bytes`) — reverting either is caught.
