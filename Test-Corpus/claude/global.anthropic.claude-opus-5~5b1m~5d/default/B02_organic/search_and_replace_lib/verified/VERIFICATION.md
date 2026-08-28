# VERIFICATION.md — differential verification of the C → Rust translation

Ground truth: `c_src/` (never modified). Subject: `translation/` (`libdriver.so`).
Both are loaded as **shared objects** with `libloading` and called only through
their exported `searchAndReplace` symbol, so the `#[no_mangle] extern "C"`
wrapper is part of what is verified.

## Reproduce

```sh
# 1. C shared object
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust shared object + the whole suite, under every configuration
cd translation && ./scripts/verify_all.sh

# 3. anti-vacuity: 9 injected bugs must all be caught
cd translation && ./scripts/mutation_check.sh
```

Individual phases:

```sh
cd translation
cargo build --release
cargo test --release --test phase_b_configs   # Phase B, 30 tests (CONFIGS.md)
cargo test --release --test phase_c_errors    # Phase C, 22 tests (ERRORS.md)
```

## Artifacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md` | A | `nm -D` surface of both `.so`s + C-source inventory (nothing untranslated) |
| `ERRORS.md` | A/C | 20-row error-surface table, each row → a passing differential test |
| `CONFIGS.md` | A/B | 30-row configuration-surface table, each row → a passing randomized differential test |
| `tests/common/mod.rs` | — | harness: dual `dlopen`, byte-exact comparison, `free()` of both results, `SplitMix64` PRNG |
| `tests/phase_b_configs.rs` | B | 30 tests, ~120 000 randomized C-vs-Rust comparisons (fixed seeds) |
| `tests/phase_c_errors.rs` | C | 22 tests: OOM (`ulimit -v` children), `SIGSEGV`, non-termination, boundaries |
| `scripts/verify_all.sh` | D | symbol parity + suite × {feature combos} × {release/debug `.so`} × {release/debug harness} |
| `scripts/mutation_check.sh` | D | injects 9 bugs, requires the suite to fail for each |

## Completion gate

* [x] **`SYMBOLS.md`**: `nm -D --defined-only` on the C `.so` yields exactly
      `searchAndReplace`; the Rust `.so` (release **and** debug) exports it with
      the identical name. `comm -23` of the two sorted lists is **empty**
      (0 missing symbols). Undefined symbols in the Rust `.so` are libc/libgcc
      only (`ldd` → `libc.so.6`, `libgcc_s.so.1`, `ld-linux`); no unresolved
      non-libc symbol. No C source file, function, or macro-generated name was
      left untranslated (`c_src` contains one header with one declaration and one
      `.c` file with one definition).
* [x] **Phase B**: all 30 rows of `CONFIGS.md` pass across randomized inputs
      (per-row loops of 6–20 000 cases, fixed seeds; plus an exhaustive
      cross-product over the tiny shapes and a 64 KiB stress row). Results are
      compared byte-for-byte, including `NULL`-ness, and every returned buffer is
      released with libc `free()` (so a wrong allocator would abort the test).
* [x] **Phase C**: all 20 rows of `ERRORS.md` have a passing differential test —
      4 distinct `return NULL` allocation-failure sites (`malloc` prefix,
      `realloc` value, `realloc` gap, `realloc` tail), the failing `strdup`,
      the 4 NULL-pointer `SIGSEGV` shapes (compared by *signal number*, not just
      "both failed"), the 2 non-termination shapes (both must still be running
      after 3 s), the empty-needle OOM, the oversized replacement, the
      zero-length-but-valid inputs, the one-past-the-end needle length, the
      `from == orig_len` boundary, aliased arguments, and the mechanical guard
      that the public API still has no enum/integer parameter (so the
      "out-of-range enum variant" class stays vacuous).
* [x] **Every configuration**: the crate declares no `[features]`, so the three
      possible invocations (`default`, `--no-default-features`,
      `--all-features`) resolve to the same empty set; all three are run anyway,
      each against the **release** and the **debug** Rust `.so`
      (`debug_assertions` + integer-overflow checks) and with the harness built
      in both profiles — 9 suite runs plus 6 symbol-parity checks, all green.
* [x] **Not vacuous**: `scripts/mutation_check.sh` — 9/9 injected bugs detected;
      `src/lib.rs` restored bit-identically afterwards (md5 compared).

## Behavioural notes confirmed against the C (not "fixed")

* No match → `strdup(orig)`; a `NULL` from `strdup` is returned as-is.
* Allocation failure → `NULL`, leaking whatever was already allocated.
* Empty `search` → `strstr` matches at offset 0 forever and nothing advances, so
  the C loops **forever** (never allocating when `value` is empty, and growing
  until `realloc` fails when it is not). The Rust does exactly the same; this is
  reproduced, not repaired.
* The `&& from > 0` term on line 78 is dead code for every terminating input
  (`from == 0` requires `search_len == 0`, i.e. the infinite-loop case); it is
  kept verbatim.
* `strncpy`'s NUL-padding is reproduced (`c_strncpy`) although the algorithm
  overwrites or terminates every byte it depends on, so no uninitialised byte is
  ever observable in the result.
* Matches are non-overlapping: the scan restarts at
  `orig + inx_start + search_len` (`"aa"` in `"aaa"` replaces one occurrence and
  keeps the trailing `"a"`), and the replacement text is never re-scanned.
