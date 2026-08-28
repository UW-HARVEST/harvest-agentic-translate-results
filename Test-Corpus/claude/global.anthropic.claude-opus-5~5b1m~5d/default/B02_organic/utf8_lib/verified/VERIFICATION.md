# VERIFICATION.md — result of the C↔Rust differential verification

`c_src/` is the ground truth (`c_src/src/lib.c`, 119 lines, two exported
functions). `translation/` is the Rust crate under test.

**Result: the translation is byte-for-byte faithful. No divergence was found,
and no change to `src/lib.rs` was required.**

## How the comparison is done

Every test loads **both** shared objects with `libloading` and calls them only
through their exported C symbols — the Rust functions are never called
directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers and the ABI are
under test as well.

```
c_src/build/libdriver.so          <- gcc / cmake
translation/target/{debug,release}/libdriver.so
```

Compared per call:

| entry point | compared quantity |
|-------------|-------------------|
| `w_utf8_drop`   | the returned pointer (both sides get the *same* input pointer, so the returned pointers must be bit-identical) |
| `w_utf8_filter` | NULL-ness; the whole NUL-terminated payload; `malloc_usable_size(result) >= len+1` (heap-overflow detector) |
| `w_utf8_filter` | the **exact sequence of allocator requests** (`malloc`/`realloc`/`strdup` + requested size), recorded by an `LD_PRELOAD` interposer in a child process |
| both | agreement with an *independent third* implementation of the C semantics (`model_drop`, `model_filter`, `model_allocs` in `tests/phase_c_errors.rs`), so the two sides are not merely equal to each other |

`malloc_usable_size` is deliberately **not** used as an equality oracle: it
depends on which heap chunk the allocator recycles and differs by ±16 bytes
between two identical requests. The allocation-trace comparison replaces it.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | `nm -D` surface of both `.so`s; C source inventory |
| `ERRORS.md`  | error-surface table, **32 rows**, one per distinct rejection in the C |
| `CONFIGS.md` | configuration-surface table, **45 rows**, cross-product of options × input shapes |
| `tests/common/mod.rs` | loader, differential drivers, splitmix64 PRNG, input generators |
| `tests/common/child.rs` | child-process plumbing (abort + `LD_PRELOAD` modes) |
| `tests/fixtures/failalloc.c` | allocator interposer: deterministic OOM injection + allocation tracing |
| `tests/phase_b_configs.rs` | 45 tests, one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 34 tests: one per `ERRORS.md` row + the oracle + the child dispatcher |
| `tests/smoke.rs` | loader sanity check |

## Scripts

| script | purpose |
|--------|---------|
| `./check_symbols.sh [so]` | Phase A/D symbol parity (`nm -D` diff, undefined-symbol allowlist) |
| `./run_tests.sh [features]` | builds C + Rust (debug **and** release) and runs the whole suite against **both** cdylibs |
| `./check_all_features.sh` | enumerates every feature combination from `Cargo.toml` and runs `run_tests.sh` for each |
| `./check_rows.sh` | mechanical cross-check: every row in `ERRORS.md`/`CONFIGS.md` names a test that exists **and** passed |
| `./mutation_check.sh` | injects 32 bugs into `src/lib.rs` one at a time and proves the suite catches each one |

### Precision of the OOM injection

The interposer only fails requests of at least `min_size` bytes (set to the
input length, several KiB), and it counts how many injected failures actually
fired. Every OOM row asserts `fired=1` (or `fired=0` for the rows that must
*not* fire, e.g. arming the 2nd `realloc` when only one happens, or arming
`realloc` with `replacement == 0`). Without the size gate an unrelated small
allocation by the Rust runtime could swallow the injected failure — that showed
up as a 1-in-8 flake during development and is now impossible: the run was
repeated 8× plus 3 full gate runs with zero failures.

All cargo commands run with `--offline` (`.cargo/config.toml` sets
`net.offline = true`); `libloading 0.8` came from the local registry cache.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff (C defined − Rust defined) is **empty**;
      both export exactly `w_utf8_drop` and `w_utf8_filter`. The Rust `.so` has
      **0** undefined non-libc / non-unwinder symbols. `./check_symbols.sh`
      prints `SYMBOL PARITY: OK` for the debug *and* the release cdylib.
      The whole C library is one translation unit and it is translated in full —
      there is no stub, no `unimplemented!()`, no `todo!()` in `src/lib.rs`.
- [x] **Phase B** — all **45** `CONFIGS.md` rows pass. Randomised rows use 2000
      inputs each from a fixed splitmix64 seed; three rows are exhaustive
      (every 1-byte input, all 65 025 2-byte inputs, 23³ and 23⁴ sweeps over the
      boundary-byte set, all 0xEF/0xE0/0xED/0xF0/0xF4 leads × all 256 second
      bytes). Both entry points are driven directly, including the low-level
      `w_utf8_drop` (which is not even declared in `lib.h`) and the composed
      `drop → filter → drop` pipeline.
- [x] **Phase C** — all **32** `ERRORS.md` rows have a passing differential
      test, including:
      * both `assert(string != NULL)` sites → same `SIGABRT`, same assertion
        expression / line / function (child processes);
      * all three `NULL`-return branches (`strdup`, `malloc`, `realloc`) reached
        deterministically with the `LD_PRELOAD` interposer — including "fail the
        n-th realloc" and the proof that `replacement == 0` performs no realloc
        at all;
      * every individual clause of `valid_1`…`valid_4`, each probed one step
        inside and one step outside its boundary (0xC1/0xC2, 0xE0+0x9F/0xA0,
        0xED+0x9F/0xA0, 0xF0+0x8F/0x90, 0xF4+0x8F/0x90, 0xF4/0xF5);
      * out-of-range values for the C `_Bool` parameter — every non-zero byte
        (2, 3, 0x7F, 0x80, 0xFE, 0xFF) and registers with garbage in the upper
        56 bits (`0x…EF00` vs `0x…EF01`), which GCC reads with `cmpb $0x0`;
      * one byte past the end of the buffer: the input's NUL placed as the last
        readable byte before a `PROT_NONE` guard page.
- [x] **Every feature combination** — `Cargo.toml` declares **no `[features]`
      table**, so the complete set of configurations is `<default>` and
      `--no-default-features`; `./check_all_features.sh` runs the full suite for
      both, against both the debug and the release cdylib, and reports
      `ALL FEATURE COMBINATIONS: OK`. The suite also passes with the tests
      themselves compiled in release (`cargo test --release`).
- [x] **The suite is proven to have detection power** — `./mutation_check.sh`
      injects 32 distinct bugs and **all 32 are detected** (`survivors: 0`),
      among them two *content-equivalent* mutations that only change which bytes
      are read; they are caught solely by the guard-page row, and two that only
      change allocation arithmetic (`REPLACEMENT_INC`, `repl < 3`), caught
      solely by the allocation-trace rows.

## Notable C behaviours that were reproduced, not "fixed"

1. `valid_2`'s `(x)[0] >= (char)0xC2` is a **signed** `char` comparison. Given
   the preceding `(x[0] & 0xE0) == 0xC0` it rejects exactly `0xC0`/`0xC1`, so
   the Rust uses `(b0 as i8) >= (0xC2u8 as i8)`.
2. `valid_3`'s last clause `((x)[0] != (char)0xEF || (unsigned char)(x)[1] <= 0xBF)`
   is **dead**: line 21 already forces `x[1] <= 0xBF`. It is kept verbatim, and
   `e18_valid3_ef_clause_unreachable` proves acceptance of 0xEF sequences never
   depends on it.
3. The `repl` bookkeeping is odd but non-overflowing: a `realloc(+4096)` happens
   on the 1st replacement and then only every 1365th (4096 = 3·1365 + 1), so the
   buffer grows by 4096 for every 4095 bytes of U+FFFD emitted. Reproduced
   exactly, including `repl -= 3` after the growth. Verified by comparing exact
   allocation traces at run lengths 1, 2, 3, 1364, 1365, 1366, 2730, 2731, 4096
   and 1 MiB.
4. On `realloc` failure the C returns `NULL` and leaks the old buffer. The Rust
   does the same (the leak is unobservable, and "fixing" it would change
   behaviour under an interposed allocator).
5. `w_utf8_filter` calls `strdup` (not `malloc` + `memcpy`) when the input is
   already fully valid, which is a *different* allocation request. Kept — the
   trace test distinguishes the two.
6. `w_utf8_drop` is exported even though `lib.h` does not declare it, so it is
   part of the ABI and is tested as a first-class entry point.

## Only documented deviation

`assert()` passes `__FILE__` to `__assert_fail`. The C's value is the absolute
build path CMake handed to GCC (`/…/harvest-work-…/c_src/src/lib.c` here), which
is not reproducible on another machine; the Rust passes the repository-relative
`c_src/src/lib.c`. This affects the diagnostic string on stderr only. The abort
signal, the assertion expression, the line number and the function name are
asserted identical (and mutation-tested).
