# VERIFICATION.md — completion gate

Differential verification of the Rust translation (`src/lib.rs`) against the C
ground truth (`c_src/src/lib.c`). Both are built as shared objects and are
**always** called through their exported `searchAndReplace` symbol, loaded with
`libloading` — the Rust crate is never linked or called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrapper and the C ABI are part of the test.

## Reproduce

```sh
./run_verification.sh          # builds C + Rust (all profiles/feature combos),
                               # diffs symbols, runs Phases B and C on each
```

or manually:

```sh
(cd c_src && mkdir -p build && cd build \
   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)
cargo build --release --offline
cargo test --release --offline --test symbols --test differential
cargo test --release --offline --test error_paths -- --test-threads=1
```

## Gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`, 0 undefined non-libc symbols | **PASS** | `comm -23` of the two `nm -D --defined-only` lists is empty for the release AND debug `.so`; enforced by `run_verification.sh` and by `tests/symbols.rs` |
| Every C source file translated (no skipped module) | **PASS** | the C build compiles exactly one TU (`c_src/src/lib.c`, 1 public function); see `SYMBOLS.md` inventory |
| Phase B: EVERY row of `CONFIGS.md` passes across randomized inputs | **PASS** | `tests/differential.rs`, 28/28 tests, **78 016 randomized input cases** = 156 033 FFI calls through the `.so` exports (counted by the harness, fixed seeds) |
| Phase C: EVERY row of `ERRORS.md` has a passing error-path differential test | **PASS** | `tests/error_paths.rs`, 16/16 tests (12 table rows + 4 generic-boundary tests); 7 052 in-process FFI calls plus 16 forked-child comparison pairs = 32 children (allocation failure / SIGSEGV / SIGALRM) |
| All of the above under EVERY feature combination | **PASS** | `Cargo.toml` has no `[features]`, so the combinations are `<default>` and `--no-default-features`; both are checked, built and tested, in the `release` **and** `debug` profiles |

## What is compared

* **NULL-ness** of the returned pointer.
* **Every byte** of the returned string (`strlen` + full byte compare) — since
  the C fills bytes `[0, total_bytes_allocated-1)` with copied content and
  writes the terminator at `total_bytes_allocated-1`, an identical string
  implies an identical `total_bytes_allocated`.
* **Freeability**: every returned pointer is `free()`d by the harness, which
  would abort if the Rust side had used a different allocator.
* **Process outcome** for UB / non-terminating / out-of-memory inputs: exit
  status vs. killed-by-signal (and which signal) of a forked child.

## Harness validation (mutation testing)

The harness was proven to actually detect divergences by temporarily mutating
`src/lib.rs`:

| mutation | detected? |
|----------|-----------|
| `if inx_start > 0` → `> 1` (drops a 1-byte prefix) | YES — 17/28 Phase B tests failed with byte-level divergence reports |
| removal of the `realloc` NULL check in the loop | YES — Phase C `err04`/`err11` failed: `C=Exited(0)` (NULL) vs `Rust=Signaled(11)` (SIGSEGV) |
| `if inx_start2 > from` → `>= from` | not detected — provably equivalent (`gap == 0` makes the guarded block a no-op: `realloc` to the same size + `strncpy(..., 0)`), so no test can distinguish it |

## Fixes made during verification

1. **`src/lib.rs` — string primitives.** The initial translation reimplemented
   `strlen`/`strstr`/`strncpy`/`strdup` in Rust. Behaviourally that matched the C
   on all valid inputs, but the hand-written `strlen` diverged on the UB NULL
   input in **debug** builds: rustc's null-deref instrumentation aborted
   (`SIGABRT`, "null pointer dereference occurred") where the C faults
   (`SIGSEGV`). The translation now calls the same libc functions the C calls, so
   the behaviour — including glibc's empty-needle `strstr`, `strncpy`'s zero
   padding, and the NULL fault — is identical by construction in every profile.
   (Detected by `err07`/`err08`/`err09` against `target/debug/libdriver.so`.)
2. **Test infrastructure — allocation-failure rows.** `RLIMIT_AS` did not make a
   24 MiB allocation fail, because glibc can serve requests ≤ 32 MiB from a
   thread arena's pre-reserved (already mapped) 64 MiB region. The targeted
   allocation was raised to 128 MiB and the huge inputs are built without
   duplicating them in the parent, which makes rows 2–6 and 11 fail
   deterministically.
3. **Test infrastructure — accidental empty `search`.** One boundary test could
   generate `orig == search == ""`, i.e. the non-terminating C path, which leaks
   until the process' allocator is exhausted (it aborted the whole test binary).
   `harness::check` now refuses an empty `search` in-process; that input is only
   exercised in forked children (`err10`, `err11`).

No changes were made to anything in `c_src/`.

## Notable C behaviours that are reproduced (not "fixed")

* A failing `realloc` returns `NULL` **and leaks** the previous buffer.
* An empty `search` never terminates; with a non-empty `value` it grows the
  buffer without bound and finally returns `NULL` on allocation failure.
* No NULL checks on the three arguments: a NULL argument faults inside `strlen`,
  and `strlen(value)` runs before the `strstr` early-out, so a NULL `value`
  faults even when `search` does not occur in `orig`.
* Overlapping occurrences are consumed non-overlapping (the re-scan starts at
  `orig + inx_start + search_len`).
* The redundant `from > 0` guard on the trailing copy is kept.
