# CONFIGS.md — configuration-surface table (Phase B gate)

Derived **mechanically** from the public header and the branches in
`c_src/src/slicing.c`.

## Build-time configuration

`c_src/CMakeLists.txt` contains no `option()`, no `add_definitions`, no
`target_compile_definitions`, and `slicing.c`/`slicing.h` contain no `#ifdef`
other than the header include guard. There is therefore exactly **one** C build
configuration, and the Rust crate mirrors it with an **empty default feature
set** (`[features] default = []`, no other features). Feature combinations to
verify: **1** — `--no-default-features` (identical to `--all-features` and to the
default build, since the set is empty).

## Public entry points (complete)

`c_src/include/slicing.h` declares exactly one prototype; `nm -D` on the C `.so`
exports exactly one symbol. `slice` *is* the lowest-level entry point — there is
no convenience wrapper layered on top of anything, so "exercise the low-level
API directly" is satisfied by definition.

| entry point | prototype |
|-------------|-----------|
| `slice` | `int slice(char *mystr, int *start_ptr, int *stop_ptr)` |

## Axes the C code actually branches on

| axis | values the source distinguishes | where |
|------|--------------------------------|-------|
| `start_ptr` | `NULL` → `start = 0` (default) / non-NULL → `start = *start_ptr` | `if (start_ptr)` |
| `stop_ptr` | `NULL` → `stop = (int)len` (default) / non-NULL → `stop = *stop_ptr` | `if (stop_ptr)` |
| `len = strlen(mystr)` | `0`, `1`, small (2–8), medium, large; also "buffer longer than `strlen`" (embedded NUL) | `strlen`, both range checks |
| `start` value | `0`, `1`, `len-1`, `len` (boundary — accepted), interior (random) | `start > len`, pointer arithmetic |
| `stop` value | `start+1` (minimal window), interior, `len` (boundary — accepted) | `stop > len`, `stop <= start` |
| slice width `stop - start` | `0` (only reachable via `stop_ptr == NULL` with `start == len`), `1`, `len`, random | `%.*s` precision |
| byte content | printable ASCII / bytes that look like printf conversions (`%s %n %%`) / high bytes 0x80–0xFF (invalid UTF-8) / multibyte UTF-8 cut mid-codepoint / all byte values 0x01–0xFF | `%.*s` is raw bytes, not text |
| call sequencing | the function is stateless; a long randomised call *sequence* must produce the same transcript | whole function |

Every row is driven with **many randomised inputs** (xorshift64\* PRNG, fixed
seed `0x5DEECE66D`, printed on failure) unless the row is a fixed boundary case,
and both `.so`s are called through their exported `slice` symbol with fd 1
redirected, so the assertion is `(retval, stdout bytes)` equality.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| 1 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; `len == 0` (empty string) | [x] |
| 2 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; `len == 1` | [x] |
| 3 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; random `len` in 2..=64, random printable ASCII | [x] |
| 4 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; large `len` in 256..=4096, random bytes 0x01–0xFF (invalid UTF-8) | [x] |
| 5 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; content contains printf conversion specifiers (`%s %d %n %%`) — data must not be interpreted as a format | [x] |
| 6 | `slice` | buffer physically longer than `strlen` (embedded NUL, garbage after it), with defaults **and** with indices at / past the embedded NUL but still inside the allocation — bounds follow `strlen`, not the allocation | [x] |
| 7 | `slice` | `start_ptr = &0`, `stop_ptr = NULL`; whole string via explicit start | [x] |
| 8 | `slice` | `start_ptr = &len`, `stop_ptr = NULL`; `start == len` boundary → zero-width slice (the only path that reaches `%.*s` with precision 0) | [x] |
| 9 | `slice` | `start_ptr = &(len-1)`, `stop_ptr = NULL`; last byte only | [x] |
| 10 | `slice` | `start_ptr = &s`, `stop_ptr = NULL`; random `s` in `[0, len]`, random `len`/content | [x] |
| 11 | `slice` | `start_ptr = NULL`, `stop_ptr = &len`; `stop == len` boundary, default start | [x] |
| 12 | `slice` | `start_ptr = NULL`, `stop_ptr = &1`; minimal valid `stop` against default `start = 0` | [x] |
| 13 | `slice` | `start_ptr = NULL`, `stop_ptr = &e`; random `e` in `[1, len]` | [x] |
| 14 | `slice` | both non-NULL; `start = 0`, `stop = len` (full range explicitly) | [x] |
| 15 | `slice` | both non-NULL; `start = len-1`, `stop = len` (single trailing byte) | [x] |
| 16 | `slice` | both non-NULL; `stop = start + 1` (minimal window) at random `start` in `[0, len-1]` | [x] |
| 17 | `slice` | both non-NULL; random `0 <= start < stop <= len` over random `len`/content | [x] |
| 18 | `slice` | both non-NULL; `len == 1`, `start = 0`, `stop = 1` (smallest non-empty slice) | [x] |
| 19 | `slice` | both non-NULL; UTF-8 text with `start`/`stop` cutting **mid-codepoint** → raw, invalid-UTF-8 bytes on stdout | [x] |
| 20 | `slice` | both non-NULL; string of all byte values 0x01..=0xFF, sliced at every 16-byte boundary | [x] |
| 21 | `slice` | all four pointer combos × `len` 0..=8 × every valid `start`/`stop` — exhaustive valid-domain cross-product | [x] |
| 22 | `slice` | statelessness: one randomised transcript of 400 mixed calls (all pointer combos, valid + invalid, random lengths/content) replayed against both `.so`s in the same order; full concatenated stdout compared | [x] |
| 23 | `slice` | `strlen(mystr) > INT_MAX` (a real 2 GiB string, `len == 2^31`): the `size_t → int` truncation of the default `stop`, `INT_MAX` as a *valid* `start` (`== len-1`), the wrapped `stop - start` precision, plus the three rejection paths at that length | [x] |

Row 23 lives in `tests/huge_string.rs` because it needs ~2 GiB of RAM; it is a
real, passing differential row here (`SKIP_HUGE_STRING=1` skips it, and it
reports `skip` rather than a bogus pass if the allocation fails). It asserts C
really took the wrapped-precision path (`k+1` bytes printed for
`start = INT_MAX-k`), so it cannot silently stop testing what it claims to.

## Test mapping

| rows | test binary |
|------|-------------|
| 1–22 | `tests/differential.rs` (`cargo test --test differential`) |
| 23 | `tests/huge_string.rs` (`cargo test --test huge_string`) |

## Status

- [x] 23/23 rows pass byte-for-byte across their randomised inputs
      (5 195 + 20 differential cases), under the single (empty) feature
      configuration, in both the `debug` and the `release` profile.
- [x] Sensitivity of the suite is itself verified by `./mutation_check.sh`,
      which injects 8 deliberate bugs into `src/lib.rs` and requires every one
      of them to be caught.
