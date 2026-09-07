# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## C source inventory

`c_src/CMakeLists.txt` builds exactly one shared library (`driver`) from exactly
one translation unit (`src/driver.c`). There is **no** second module and **no**
binary/driver executable target, so there is no missing-file class of failure
here: `src/driver.c` is 86 lines and is fully translated in `translation/src/lib.rs`.

Functions defined in `c_src/src/driver.c`:

| C function | linkage | exported? |
|------------|---------|-----------|
| `printLine(const char *)` | external | yes |
| `printIntLine(int)`       | external | yes |
| `bad(float)`              | external | yes |
| `goodG2B(void)`           | `static` | **no** (internal) |
| `goodB2G(float)`          | `static` | **no** (internal) |
| `good(float)`             | external | yes |
| `driver(float, float)`    | external | yes |

`goodG2B` / `goodB2G` are `static` and therefore have `t` (local) linkage in the
C `.so`; they are deliberately *not* exported from the Rust `.so` either
(they are private `unsafe fn`s in `lib.rs`).

## Dynamic symbol table comparison

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `bad`          | `T` | `T` | MATCH |
| 2 | `driver`       | `T` | `T` | MATCH |
| 3 | `good`         | `T` | `T` | MATCH |
| 4 | `printIntLine` | `T` | `T` | MATCH |
| 5 | `printLine`    | `T` | `T` | MATCH |

Symbol diff (C-exported symbols absent from Rust): **EMPTY** — 0 missing.

Verified by `tests/symbols.rs::symbol_parity_c_vs_rust`, which shells out to
`nm -D --defined-only` on both objects, filters to the C source's own symbols
(excluding the libc/loader boilerplate `_init`, `_fini`, `__bss_start`, `_edata`,
`_end`, and Rust's allocator/panic runtime symbols), and asserts the C set is a
subset of the Rust set.

## Undefined (imported) symbols

The C `.so` imports `printf` and (folded to `andps` by the compiler, so not
actually imported) `fabs` from libc. The Rust `.so` imports `printf` from libc
too — intentional, so that both libraries share the *same* `stdout` FILE buffer
and produce identically ordered output when both are loaded into one process.

Non-libc undefined symbols in the Rust `.so`: **NONE** (checked with
`nm -D --undefined-only target/release/libdriver.so`).

## Cargo feature combinations

`translation/Cargo.toml` declares **no** `[features]` table and no optional
dependencies, so the only build configuration is the default one. Confirmed by
`grep -n '\[features\]' Cargo.toml` returning nothing. Phase D's
"repeat for every feature combination" therefore reduces to the single default
combination, which is additionally re-run explicitly with
`--no-default-features` (a no-op here, but checked).

## Verification evidence

```
$ nm -D --defined-only --format=posix ../c_src/build/libdriver.so | awk '{print $1}' | sort
bad
driver
good
printIntLine
printLine

$ comm -23 <(C symbols) <(Rust symbols)      # debug profile
<empty>
$ comm -23 <(C symbols) <(Rust symbols)      # release profile
<empty>
```

Symbol diff is EMPTY for **both** profiles. Automated by `run_all.sh`.

### Harness caveat that was found and fixed

`cargo test` does **not** build `crate-type = ["cdylib"]` artifacts, because the
test harness never links them. The first version of the harness fell back to
whichever profile's `.so` happened to exist, which meant a `debug` test run
silently exercised the `release` object — coverage that looked real but was not.
`common::rust_so_path()` is now profile-strict: it uses only
`target/<this profile>/libdriver.so`, honours a `DRIVER_RUST_SO` override, and
panics with build instructions rather than falling back. `run_all.sh` therefore
runs `cargo build [--release]` before each `cargo test`.

### Harness self-checks

`tests/symbols.rs` also contains two negative controls, so the suite cannot pass
vacuously:

* `harness_capture_observes_real_bytes` — asserts the fd-1 capture returns the
  exact expected bytes from both libraries (a capture that silently returned
  nothing would make every differential test trivially pass).
* `harness_diff_detects_divergence` — feeds `diff` two deliberately different
  payloads and asserts it panics.
