# Verification report

Differential verification of the Rust translation in `translation/` against the
C ground truth in `c_src/`.

Both implementations are built as shared libraries and loaded with `libloading`;
**every** call in every test goes through the exported C symbols of a `.so`, so
the `#[unsafe(no_mangle)] extern "C"` wrappers are themselves under test. The
Rust crate is never called directly.

## How to reproduce

```bash
# 1. build the C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build + run the differential suite
cd ../../translation
cargo build --release
cargo test --release

# 3. every profile x feature combination (Phase D)
./scripts/check_features.sh

# 4. validate that the suite can actually fail (mutation testing)
./scripts/mutation_check.sh
```

## Completion gate

| gate | status | evidence |
|---|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | C exports `call_fma`, `driver`, `fma_array`; Rust exports exactly the same 3. Symbol diff is empty. No C source was left untranslated (`c_src` is one file). |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | 34 rows, 34 tests, all green (`tests/phase_b.rs`) |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** | 21 rows + 2 generic-boundary rows, 23 tests, all green (`tests/phase_c.rs`) |
| All of the above under **every** feature combination | **PASS** | `Cargo.toml` declares no `[features]`, so the matrix is `{--no-default-features, default} x {release, dev}` = 4 configurations, all green via `scripts/check_features.sh` |

## Surface summary

| | |
|---|---|
| C source | `c_src/src/driver.c` (63 lines), `c_src/include/driver.h` |
| exported symbols | `fma_array`, `call_fma`, `driver` (3/3 matched) |
| `CONFIGS.md` rows | 34 |
| `ERRORS.md` rows | 21 + 2 generic boundary rows |
| tests | 57 (`phase_b`: 34, `phase_c`: 23) |
| mutants injected | 32 — 28 must-detect (all detected), 4 provably-equivalent (all correctly survived) |

Note that `driver.h` declares only `driver()`, but `fma_array()` and
`call_fma()` are non-`static` and therefore part of the exported ABI. Both are
exercised **directly** as low-level entry points, not merely through the
`driver()` one-shot wrapper.

## Divergences found and fixed

1. **Debug-profile null-pointer UB trapped differently.** `fma_array` used
   `*mul1.offset(i)` / `*out.offset(i) = v`. Under `-C debug-assertions=on`
   (the dev profile) rustc inserts a null-pointer check around a raw-pointer
   *place* expression, and `<*const T>::offset` additionally carries a
   debug-checked in-bounds precondition. A caller passing `NULL` therefore made
   the debug build die with `SIGABRT` where the C build dies with `SIGSEGV`
   (`err20`). Fixed by using `wrapping_offset` plus `core::ptr::read` /
   `core::ptr::write`, which lower to the same instructions and carry no such
   checks — the NULL-pointer UB now dies identically to C in **both** profiles.
   `driver`'s cursor advance was changed to `wrapping_add` for the same reason.
   This is exactly the class of bug that only appears when the suite is run
   across every build configuration rather than just the default one.

2. **`sscanf` entry point.** A C compiler targeting glibc redirects `sscanf` to
   `__isoc99_sscanf` (confirmed by `nm -D -u` on the C `.so`), and the two glibc
   entry points are not identical (they differ for `%a` and positional `%n$`).
   The Rust translation now binds `__isoc99_sscanf` explicitly on
   `target_env = "gnu"`. For the format actually used, `"%d%zn"`, the two are
   equivalent — the `mutation_check.sh` mutant `sscanf: legacy glibc entry
   point` demonstrates this empirically — but binding the same symbol removes
   the divergence class entirely.

## Documented, deliberate deviations

Both are cases where the C behaviour is *not a computed result*, so there is
nothing to reproduce byte-for-byte. Each is characterised by a test rather than
hidden.

1. **`call_fma` with `len < 0`** (`ERRORS.md` note A). `int out[len]` with a
   negative size is UB; the C build returns indeterminate garbage that changes
   between runs (measured: `-991075438`, then `32767`) and faults outright for
   `INT_MIN`. The Rust version returns `0` without touching memory. `err04`
   asserts the Rust export is total and confirms the C side really is the UB
   case.

2. **`call_fma` with `len` beyond the stack budget** (`ERRORS.md` note D).
   `call_fma` puts three `int[len]` VLAs on the *caller's* stack and never
   checks the size, so with an 8 MiB stack the C build works at
   `len = 690_000` and faults at `len = 700_000`. Rust heap-allocates and keeps
   working. `err21` verifies agreement for every `len` that fits the stack and
   pins down the boundary behaviour on both sides.

## Harness notes worth knowing

* **`driver`'s stdout is captured in a forked child.** An in-process `dup2` of
  fd 1 does not work here: `libtest` writes its own `"test foo ... ok"` progress
  lines to fd 1 from the main thread, which got spliced into the captured bytes
  and produced bogus divergences. Each test drives all of its inputs inside one
  child per library and the transcript is split into one line per input.
* **Core dumps are disabled in forked children** (`setrlimit(RLIMIT_CORE, 0)`).
  The crash-path tests provoke `SIGSEGV` deliberately; dumping core for each one
  took 80+ seconds and littered the tree. The suite now runs in ~4 s.
* **Large-`len` rows run on an explicitly sized thread stack.** `libtest` gives
  each test a 2 MiB stack, which would cap the C VLAs at `len ~ 170_000`;
  without `common::on_big_stack` those tests would be measuring the harness
  rather than the translation.
* **The `.so` must match the test binary's profile — no fallback.** `cargo test`
  does not build a `cdylib`-only lib target, so an earlier version of the
  harness silently loaded `target/release/libdriver.so` while running the *dev*
  test binary. Since the two profiles genuinely differ for raw-pointer UB, that
  would have invalidated the dev-profile result. The harness now requires
  `target/<profile>/libdriver.so` and fails with the exact `cargo build` command
  otherwise (override with `DRIVER_RUST_SO`).
* **Randomized inputs use a fixed seed** (`0x5EED_1234_ABCD_9876`, SplitMix64)
  so every run is reproducible.
