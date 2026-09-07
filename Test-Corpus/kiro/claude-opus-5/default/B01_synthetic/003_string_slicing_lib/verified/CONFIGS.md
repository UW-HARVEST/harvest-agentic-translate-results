# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

Mechanically derived from `c_src/include/slicing.h` + `c_src/src/slicing.c`.

## Axes the C actually branches on

There are no compile-time `#ifdef`s, no global/runtime option setters, and no
opaque handle to configure — the entire configuration space of this library is
carried in the three arguments of the single entry point. The axes below are
exactly the things the C source tests.

**Axis 1 — `start_ptr` presence** (`if (start_ptr)` at line 43)
- `NULL` → `start = 0` implicitly, and the `start > len` check is skipped
- non-`NULL` → `start = *start_ptr`, range-checked

**Axis 2 — `stop_ptr` presence** (`if (stop_ptr)` at line 52)
- `NULL` → `stop = len` (via a `size_t`→`int` truncating assignment), and
  *both* the `stop > len` and `stop <= start` checks are skipped
- non-`NULL` → `stop = *stop_ptr`, range-checked twice

Axes 1 × 2 give the four presence modes: `(NULL,NULL)`, `(set,NULL)`,
`(NULL,set)`, `(set,set)`. These are genuinely different code paths, not just
different data.

**Axis 3 — input string shape** (feeds `strlen`, and the `mystr + start`
pointer arithmetic and `%.*s` in the final `printf`)
- `len == 0` (empty string) — degenerate: makes `start`'s only valid value `0`
  and makes every `stop_ptr != NULL` an error
- `len == 1` — single element
- `len` many (short, and long enough to cross glibc's `printf` fast paths)
- byte content: pure ASCII; bytes with the high bit set (0x80–0xFF, i.e.
  non-UTF-8 — the C is byte-oriented and must **not** be validated as UTF-8 by
  the Rust); bytes that are `printf` format metacharacters (`%`, `%n`, `%s`) —
  these travel as *data* through the `%.*s` argument and must not be
  reinterpreted; `\n`, `\t`, `\0`-adjacent and 0x7F control bytes

**Axis 4 — index values within the valid range** (drives `stop - start` and
`mystr + start`)
- `start == 0` (front)
- `start` interior
- `start == len` (the maximum *valid* start: `start > len` is false, so a start
  exactly at the terminator is accepted)
- `stop == len` (maximum valid stop)
- `stop == start + 1` (minimum valid width — `stop <= start` is rejected, so 1
  is the narrowest slice)
- `stop - start` == full length

## Entry points

There is exactly **one** public entry point, `slice`, and it *is* the
lowest-level entry point — there are no convenience wrappers, no one-shot
helpers, and no internal functions with external linkage
(`nm -D --defined-only` on the C `.so` lists only `slice`; see `SYMBOLS.md`).
Every row below therefore calls `slice` directly through the `.so` export.

The observable output of `slice` is twofold and **both** are compared
byte-for-byte in every row:
1. the `int` return value, and
2. everything written to **stdout** (captured by redirecting fd 1 around each
   call, then `fflush(NULL)`).

## Table

Every row is exercised with many randomized inputs (fixed seed, deterministic
xorshift PRNG in `tests/common/mod.rs`) — not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; randomized ASCII strings, `len` 1..64 → prints whole string | `c1_both_null_ascii` | [x] |
| C2 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; `len == 0` (empty string) → precision 0, prints bare newline | `c2_both_null_empty` | [x] |
| C3 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; randomized **arbitrary bytes** 0x01–0xFF (non-UTF-8, high-bit set, control bytes) | `c3_both_null_arbitrary_bytes` | [x] |
| C4 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; strings containing `printf` metacharacters (`%s`, `%n`, `%d`, `%%`) as data | `c4_both_null_format_metachars` | [x] |
| C5 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; long strings (`len` 256..4096) crossing `printf` buffering paths | `c5_both_null_long` | [x] |
| C6 | `slice` | `start_ptr` set, `stop_ptr = NULL`; `start` randomized in `0..=len` (so `stop = len` implicitly) | `c6_start_set_stop_null` | [x] |
| C7 | `slice` | `start_ptr` set, `stop_ptr = NULL`; `start == len` exactly (maximum valid start → zero-width slice, prints bare newline) | `c7_start_equals_len_stop_null` | [x] |
| C8 | `slice` | `start_ptr` set, `stop_ptr = NULL`; `len == 0` and `start == 0` (only valid combination for the empty string) | `c8_start_zero_empty_stop_null` | [x] |
| C9 | `slice` | `start_ptr = NULL`, `stop_ptr` set; `stop` randomized in `1..=len` (start implicitly 0; `stop == 0` is the E9 error) | `c9_start_null_stop_set` | [x] |
| C10 | `slice` | `start_ptr = NULL`, `stop_ptr` set; `stop == len` exactly (full string via explicit stop) | `c10_start_null_stop_equals_len` | [x] |
| C11 | `slice` | `start_ptr` set **and** `stop_ptr` set; randomized valid pair `0 <= start < stop <= len`, interior slices | `c11_both_set_interior` | [x] |
| C12 | `slice` | `start_ptr` set **and** `stop_ptr` set; boundary widths — `stop == start + 1` (narrowest legal slice) and `start == 0, stop == len` (widest) | `c12_both_set_width_boundaries` | [x] |
| C13 | `slice` | `slice` with **unconstrained randomized `int` values** for `start`/`stop` over the full `i32` range, and randomized `NULL`/non-`NULL` for each pointer, over randomized byte strings incl. `len == 0` — the cross-product fuzz that mixes valid and rejected configurations in one sweep (return value **and** stdout compared) | `c13_full_cross_product_fuzz` | [x] |
| C14 | `slice` | repeated/sequential invocation: many `slice` calls in a row without intervening flush, verifying stdout **accumulation and ordering** is identical (stateless-ness of the library) | `c14_sequential_calls_stream_order` | [x] |
| C15 | `slice` | `start_ptr` and `stop_ptr` aliasing the **same** `int` object (legal C, forces `stop == start` → E8 path) and pointing into a shared array; also `start_ptr`/`stop_ptr` pointing at adjacent stack ints to confirm neither is written back | `c15_aliased_index_pointers` | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(String_Slice SHARED ...)`
and no `add_executable`, so the project builds **no driver binary** — there is
no stdout-of-binary comparison to make. `translation/Cargo.toml` likewise
declares only `crate-type = ["cdylib"]` and has no `[[bin]]`.

**Gate: 15/15 rows pass across randomized inputs. PASS.**
