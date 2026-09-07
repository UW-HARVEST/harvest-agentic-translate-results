# VERIFICATION.md — results and harness-adequacy evidence

## Result

The Rust translation in `src/lib.rs` is **byte-identical in behaviour** to the C
in `c_src/src/lib.c` across every configuration and error condition enumerated in
`CONFIGS.md` and `ERRORS.md`. **No divergence was found, so no fix to the Rust
was required.** `cargo check` was clean from the start.

## How the tests work

Every test `dlopen`s **both** shared libraries via `libloading` and calls the
exported `UTIL_createLinePointers` symbol. The Rust functions are **never**
called directly, so the `#[no_mangle] extern "C"` export wrapper is itself under
test:

* C: `../c_src/build/libdriver.so`
* Rust: `target/<profile>/libdriver.so` (profile-strict — see below)

Comparison metric: the returned sentinel (`NULL` vs non-`NULL`) plus, on success,
all `numLines` returned pointers normalized to `ptr - buffer`. The same buffer
pointer is handed to both libraries, so these offsets are exactly the `buffer +
pos` values the C computes and are directly comparable; the heap address of the
returned array is allocation-dependent and therefore not part of the contract.

| file | phase | tests |
|------|-------|-------|
| `tests/common/mod.rs` | harness | loader, comparison, seeded xorshift64* PRNG |
| `tests/phase_b_configs.rs` | B | 17 (one per `CONFIGS.md` row) |
| `tests/phase_c_errors.rs` | C | 10 (one per `ERRORS.md` row + boundary sweep) |
| `tests/phase_c_heap.rs` | C | 4 (memory contract: the `free()` on the reject path) |
| `tests/phase_d_symbols.rs` | D | 2 (symbol parity, no unresolved symbols) |

**33 tests total, all passing.** Run everything with `./run_all_combos.sh`.

## Two harness bugs found and fixed during verification

1. **Wrong artifact under test.** `cargo test` does *not* build the `cdylib`, so
   the debug-profile run was silently `dlopen`ing the *release* `.so` through a
   cross-profile fallback. The fallback was removed (`tests/common/mod.rs` now
   panics instead of loading another profile's library) and
   `run_all_combos.sh` builds the library for each profile before testing.
2. **Flaky heap measurement.** `mallinfo2()` reports process-wide allocator
   state, so the four heap tests measured each other when run on parallel
   harness threads. They now share a `HEAP_LOCK` mutex and use a noise floor
   that each test asserts is ≥5–20x smaller than the leak it is looking for.

`run_all_combos.sh` also originally ran `cargo test` twice per combo, which let a
flaky pass mask a failure; it now runs once and judges by cargo's exit status.

## Harness adequacy — mutation testing

Passing tests only mean something if they can fail. Eleven mutants were injected
into `src/lib.rs`; the suite was re-run against each and the source restored
afterwards (`src/lib.rs` verified byte-identical to the original at the end).

| mutant | change | caught? |
|--------|--------|---------|
| M2 | line pointer off by one (`buffer + pos` → `buffer + pos + 1`) | **YES** (16 tests) |
| M4 | drop `free(bufferPtrs)` on the rejection path | **YES** (4 heap tests) |
| M7 | return `NULL` when `numLines == 0` | **YES** (9 tests) |
| M8 | `pos += len` → `pos += len + 1` | **YES** (10 tests) |
| M9 | return a fake non-heap pointer for `numLines == 0` | **YES** (allocator abort) |
| M10 | outer guard `pos < bufferSize` → `pos <= bufferSize` | **YES** (5 tests) |
| M11 | return `NULL` on the success path | **YES** (18 tests) |
| M1 | drop the `if (pos < bufferSize)` guard on the NUL skip | no — **equivalent** |
| M3 | `lineIndex != numLines` → `lineIndex < numLines` | no — **equivalent** |
| M5 | `wrapping_mul` → `saturating_mul` | no — **equivalent** |
| M6 | inner guard `pos + len < bufferSize` → `<=` | no — **equivalent** |

The four survivors were each analysed and are **provably equivalent mutants** —
no input can distinguish them through the function's observable result:

* **M1 / M6** — both only change `pos` (or read one byte further) *after* the
  last iteration. The loop guard `pos < bufferSize` is false for both `pos ==
  bufferSize` and `pos > bufferSize`, and `lineIndex` is already incremented, so
  the loop exits with an identical `lineIndex` either way. (M6 additionally reads
  one byte out of bounds — undefined behaviour, but not an output difference.)
* **M3** — the loop guard is `lineIndex < numLines`, so `lineIndex` can never
  exceed `numLines`; `<` and `!=` are the same predicate at that point.
* **M5** — the two differ only when `numLines * 8` overflows, which requires
  `numLines >= 2^61`. Any such call can never reach `lineIndex == numLines`
  (that would need 2^61 lines), so both variants return `NULL` for every
  possible input — one via `malloc` failure, the other via the line-count check.

The remaining `#`-not-covered case is genuinely untestable: `buffer == NULL` with
`numLines > 0 && bufferSize > 0` dereferences NULL inside the C at line 17, which
segfaults in both implementations. It is documented as such in `ERRORS.md` rather
than tested.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` symbol diff is **empty**; 0 unresolved non-libc
      symbols in the Rust `.so` (enforced by `tests/phase_d_symbols.rs`).
- [x] Phase B: all **17** `CONFIGS.md` rows pass across randomized inputs
      (fixed seed `0x5EED_1234`; ~5,000 fuzz cases in row 16 alone).
- [x] Binary/driver stdout comparison: **N/A** — `c_src/CMakeLists.txt` builds
      only `add_library(driver SHARED src/lib.c)`, there is no executable.
- [x] Phase C: all **9** `ERRORS.md` rows have a passing differential test, plus
      a generic boundary sweep (NULL pointers, zero/oversized lengths, one past
      each valid range, extreme `size_t` values). The API takes no enums, so
      out-of-range enum values degenerate to the out-of-range `size_t` rows.
- [x] All of the above hold under **every** feature combination. `Cargo.toml`
      declares no `[features]`, so the combinations are default and
      `--no-default-features`, each run in both the debug profile
      (overflow-checks **on** — relevant because the C relies on unsigned
      wraparound) and the release profile. `run_all_combos.sh` derives the list
      from `Cargo.toml` automatically and reports `ALL COMBOS PASSED`
      (reproduced 4 consecutive times, no flakes).

## Reproducing

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && ./run_all_combos.sh
```

Note: this environment has no crates.io access, so `--offline` is used
throughout (`libloading 0.8.9` is present in the local cargo registry cache).
