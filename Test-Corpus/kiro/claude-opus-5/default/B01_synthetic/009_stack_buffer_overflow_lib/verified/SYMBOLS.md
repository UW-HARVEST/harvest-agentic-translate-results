# SYMBOLS.md — dynamic symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/src/driver.c`)

| C function | linkage in C | must be exported? |
|------------|--------------|-------------------|
| `printLine(const char *)` | external | yes |
| `printIntLine(int)`       | external | yes |
| `bad(int)`                | external | yes |
| `goodG2B(void)`           | `static` | no — file-local, not in the ABI |
| `goodB2G(int)`            | `static` | no — file-local, not in the ABI |
| `good(int)`               | external | yes |
| `driver(int, int)`        | external | yes |

`c_src` contains exactly one translation unit (`src/driver.c`) and one public
header (`include/driver.h`). No module was skipped by the translation; the whole
library is one file and it is fully present in `translation/src/lib.rs`.

## `T` (defined text) symbol comparison

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `bad`          | T | T | MATCH |
| 2 | `driver`       | T | T | MATCH |
| 3 | `good`         | T | T | MATCH |
| 4 | `printIntLine` | T | T | MATCH |
| 5 | `printLine`    | T | T | MATCH |

Missing from Rust: **none**. Extra in Rust: **none** (no extra `T`/`D`/`B`
symbols; Rust exports only the five C entry points).

`goodG2B` / `goodB2G` are absent from BOTH `.so` files, as required — they are
`static` in C and private (`fn`, no `#[no_mangle]`) in Rust.

## Weak / undefined symbols

Weak symbols are toolchain runtime hooks, not API. C has
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`; Rust has those plus `__cxa_thread_atexit_impl`, `gettid`,
`statx` (glibc-version-guarded weak refs emitted by Rust `std`). These are not
part of the library surface.

Undefined (`U`) symbols in the Rust `.so` are all libc / `libgcc` unwinder
imports (`printf`, `puts`, `memcpy`, `malloc`, `_Unwind_*`, …). There are **0
missing/undefined non-libc symbols** — nothing the loader cannot resolve:

```
$ ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

The C `.so` imports `printf` and `puts`. Note that GCC lowered
`printf("%s\n", line)` in `printLine` to `puts(line)`; the Rust translation
calls `printf("%s\n", line)` directly. The two are byte-identical on stdout
(`puts` appends exactly one `\n`), so this import difference is not observable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (`default` == no features). `--no-default-features` and
the empty feature set are the same build. Phase D's "every feature combination"
therefore collapses to a single combination, which is verified by
`scripts/check_features.sh`.

## How to reproduce

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo test --release          # one configuration
cd translation && ./scripts/check_features.sh   # every feature combo x both profiles
```

Exit status 0 and `ALL PHASES PASSED` mean the symbol diff was empty and every
`CONFIGS.md` / `ERRORS.md` row matched.

## Harness pitfall worth recording

`cargo test` does **not** build a `crate-type = ["cdylib"]` library: an
integration test cannot link against a cdylib, so Cargo has no reason to produce
it. A differential test that simply picks up `target/<profile>/libdriver.so`
therefore verifies whatever `.so` was left behind by an earlier `cargo build` —
edits to `src/lib.rs` are invisible and every run passes vacuously. This was
observed here: five deliberately injected bugs all "passed" until it was fixed.

`tests/common/mod.rs::rust_lib()` now (a) shells out to `cargo build --lib` with
the current profile and feature flags before locating the artifact, and (b)
asserts the resulting `.so` is not older than `src/lib.rs` / `Cargo.toml`,
aborting with `STALE ARTIFACT: …` otherwise.

The suite's sensitivity was then confirmed by mutation testing — each of these
injected defects is caught, so the assertions are not vacuous:

| injected defect | rows that failed |
|-----------------|------------------|
| `"…out-of-bounds"` → `"…out-of-bounds."` | CONFIGS 22,23,26–32; ERRORS 5,6,7,11,G7 |
| `goodB2G`: `data < 10` → `data <= 10` | CONFIGS 22,26,29,30,32; ERRORS 7,11,G7 |
| `goodG2B`: `data = 7` → `data = 6` | CONFIGS 20–32; ERRORS 5,6,7,10,11,12,G7 |
| `printLine`: NULL guard removed | CONFIGS 32; ERRORS row 1 (all three checks) |
| `bad()`: prints 9 elements instead of 10 | CONFIGS 13–19; ERRORS 3,8,9,G8 |
