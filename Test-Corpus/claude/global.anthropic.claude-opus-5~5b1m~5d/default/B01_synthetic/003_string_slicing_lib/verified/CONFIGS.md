# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/include/slicing.h` and `c_src/src/slicing.c`.

## Public entry points (complete set)

`c_src/include/slicing.h` declares exactly one function, and it is also the
lowest-level entry point — there is no convenience wrapper layer to skip past:

```
int slice(char *mystr, int *start_ptr, int *stop_ptr);
```

## Axes the C code actually branches on

Extracted from every `if` / `else` in `slice()`:

| axis | values the C distinguishes | where |
|---|---|---|
| A. `start_ptr` nullness | `NULL` → `start = 0`; non-`NULL` → `start = *start_ptr` + bound check | `if (start_ptr)` |
| B. `stop_ptr` nullness | `NULL` → `stop = len`; non-`NULL` → `stop = *stop_ptr` + two checks | `if (stop_ptr)` |
| C. string length `len` | `0` (empty), `1`, small, large/multi-hundred | `strlen(mystr)` feeds both bound checks |
| D. `start` position | `0`, interior, `len - 1`, `len` (upper boundary, valid) | `start > len` (unsigned compare) |
| E. `stop` position | `start + 1` (min valid), interior, `len` (upper boundary) | `stop > len`, `stop <= start` |
| F. slice width `stop - start` | `1`, interior, full `len` (whole string) | `printf("%.*s", stop - start, ...)` precision |
| G. byte content | printable ASCII, bytes `0x01–0xFF` incl. high/non-UTF-8 bytes, embedded `%` and `\n` (must not be interpreted as a format string), whitespace | the `%.*s` payload |

There are no `#ifdef` compile-time branches in the C source, no global/static
state, no runtime option struct, and no byte-order or element-type axis: the
only "options" are the two nullable-pointer parameters (axes A and B) and the
values they carry.

Axes A × B give the four modes below; C–G are crossed within each.

## Configuration-surface table

Every row is exercised in `translation/tests/differential.rs` with many
randomized inputs (fixed seed, deterministic xorshift PRNG) unless the row is
inherently a single point (e.g. the empty string with both pointers null).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `slice` | A=`NULL`, B=`NULL`; `len == 0` (empty string) → whole-string default, prints just `\n` | [x] |
| 2 | `slice` | A=`NULL`, B=`NULL`; `len == 1` | [x] |
| 3 | `slice` | A=`NULL`, B=`NULL`; `len` random 2..64, printable ASCII | [x] |
| 4 | `slice` | A=`NULL`, B=`NULL`; `len` random 256..1024 (long string, multi-buffer) | [x] |
| 5 | `slice` | A=`NULL`, B=`NULL`; bytes drawn from the full `0x01..0xFF` range (non-UTF-8 payload) | [x] |
| 6 | `slice` | A=`NULL`, B=`NULL`; content contains `%s`/`%n`/`%d` (format-string injection must be inert — the payload is an argument, not a format) | [x] |
| 7 | `slice` | A=`NULL`, B=`NULL`; content contains embedded `\n`, `\t`, spaces | [x] |
| 8 | `slice` | A=`NULL`, B non-`NULL`; `*stop_ptr == 1` (minimum valid stop when `start` defaults to 0) | [x] |
| 9 | `slice` | A=`NULL`, B non-`NULL`; `*stop_ptr` random in `1..=len` (prefix slices), `len` random 1..64 | [x] |
| 10 | `slice` | A=`NULL`, B non-`NULL`; `*stop_ptr == len` (full string via explicit stop, upper boundary) | [x] |
| 11 | `slice` | A non-`NULL`, B=`NULL`; `*start_ptr == 0` (whole string via explicit start) | [x] |
| 12 | `slice` | A non-`NULL`, B=`NULL`; `*start_ptr` random in `0..=len` (suffix slices), `len` random 1..64 | [x] |
| 13 | `slice` | A non-`NULL`, B=`NULL`; `*start_ptr == len` (upper boundary, valid; width 0 → prints just `\n`) | [x] |
| 14 | `slice` | A non-`NULL`, B=`NULL`; `*start_ptr == len - 1` (last character only) | [x] |
| 15 | `slice` | A non-`NULL`, B=`NULL`; long string (256..1024) with random valid `*start_ptr` | [x] |
| 16 | `slice` | A non-`NULL`, B non-`NULL`; `start == 0`, `stop == len` (whole string, both explicit) | [x] |
| 17 | `slice` | A non-`NULL`, B non-`NULL`; width exactly 1 (`stop == start + 1`) at random valid `start` | [x] |
| 18 | `slice` | A non-`NULL`, B non-`NULL`; random valid interior `0 <= start < stop <= len`, `len` random 1..64 | [x] |
| 19 | `slice` | A non-`NULL`, B non-`NULL`; `stop == len` with random `start < len` (suffix, upper boundary on stop) | [x] |
| 20 | `slice` | A non-`NULL`, B non-`NULL`; `start == len - 1`, `stop == len` (last character, both boundaries) | [x] |
| 21 | `slice` | A non-`NULL`, B non-`NULL`; long string (256..1024), random valid `start`/`stop` pair | [x] |
| 22 | `slice` | A non-`NULL`, B non-`NULL`; non-UTF-8 byte payload with a random valid `start`/`stop` pair (slice may split a would-be multi-byte sequence) | [x] |
| 23 | `slice` | A non-`NULL`, B non-`NULL`; payload containing `%` conversions with a random valid `start`/`stop` pair | [x] |
| 24 | `slice` | A non-`NULL`, B non-`NULL`; `len == 1`, `start == 0`, `stop == 1` (the only valid slice of a 1-char string) | [x] |
| 25 | `slice` | aliasing: `start_ptr == stop_ptr` (same `int` object passed for both) — C reads it twice; `stop <= start` then always fires | [x] |
| 26 | `slice` | called repeatedly in sequence on the same process (no hidden state carried between calls; interleaved success/error calls) | [x] |
| 27 | `slice` | full-domain fuzz: random `len` 0..80, each pointer independently null with p≈1/3, bound values drawn from a mix of in-range, near-boundary, and arbitrary `i32` — return code **and** stdout compared (covers valid × invalid cross-product) | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the crate has a
single configuration. `cargo test`, `cargo test --no-default-features`, and
`cargo test --all-features` compile identical code; all three are run in
Phase D to confirm.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(String_Slice SHARED ...)` —
there is **no driver binary** in either tree, so the "compare C and Rust
stdout from the binaries" gate is not applicable. stdout is nevertheless
compared byte-for-byte for every row above, because `slice()` writes its
result to stdout via `printf`.
