# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Generated mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## Build-time configuration surface

* `c_src/CMakeLists.txt` builds exactly one target: `add_library(driver SHARED src/lib.c)`.
  There are **no** `option()`, `#ifdef`, `target_compile_definitions`, or
  `#cmakedefine` switches anywhere in `c_src/`. Therefore the C library has
  exactly **one** build configuration.
* `Cargo.toml` has **no `[features]` section** (verified: `grep -rn feature
  Cargo.toml src/` → no matches, and `src/` contains no `#[cfg(feature = ...)]`).
  Therefore the complete enumeration of valid feature combinations is:

  | # | feature combination | cargo invocation | status |
  |---|---------------------|------------------|--------|
  | 1 | *(none — the only combination)* | `cargo check --no-default-features --features ""` | ✅ clean |

  `--all-features` is identical to the above because there are no features to enable.

## Exported (defined, dynamic) symbols

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|------------|-------|
| `w_utf8_drop`   | `T` @ 0x1169 | `T` | declared in `src/lib.c` only (not in `include/lib.h`), but non-`static` ⇒ exported. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn w_utf8_drop` |
| `w_utf8_filter` | `T` @ 0x1375 | `T` | declared in `include/lib.h`. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn w_utf8_filter` |

There are no macro-generated symbols: the four `valid_N` macros in `src/lib.c`
are object-like/function-like preprocessor macros that expand inline and emit no
symbols. `REPLACEMENT_INC` is a plain integer macro.

### Symbol diff

```
comm -23 <(C defined) <(Rust defined)   ->  (empty)   # nothing missing from Rust
```

**MISSING FROM RUST: none.** The diff is empty. ✅

## Undefined (imported) symbols

C `.so` imports: `__assert_fail`, `malloc`, `memcpy`, `realloc`, `strdup`,
`strlen` (all glibc) plus the standard weak
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`.

Rust `.so` imports the same six glibc functions used by the translation
(`__assert_fail`, `malloc`, `memcpy`, `realloc`, `strdup`, `strlen`) plus the
symbols pulled in by the Rust `std`/panic-unwind runtime (`_Unwind_*`,
`abort`, `free`, `calloc`, `memset`, `mmap64`, `pthread_key_*`, `write`, …).
All of these are libc / libgcc symbols — **0 missing or unresolvable non-libc
symbols**:

```
ldd -r target/debug/libdriver.so   ->  no "undefined symbol" lines
ldd -r c_src/build/libdriver.so    ->  no "undefined symbol" lines
```

## Completion

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] No stubbed / `unimplemented!()` symbols: both public functions are full
      translations of the C bodies.
- [x] No C source file in `c_src/` was left untranslated (`c_src/src/lib.c` is
      the only translation unit; `c_src/include/lib.h` contains only the one
      prototype).

## Verification tooling in this crate

| file | purpose |
|------|---------|
| `tests/common/mod.rs` | loads **both** `.so`s via `libloading` (`RTLD_NOW \| RTLD_LOCAL`, so neither library can interpose the other's `w_utf8_drop`) and provides the differential comparators |
| `tests/configs.rs` | Phase B — one test per `CONFIGS.md` row (47 rows) |
| `tests/errors.rs` | Phase C — one test per `ERRORS.md` row (28 rows) |
| `verify_all.sh` | Phase D — enumerates every feature combination, `cargo check`s each, diffs `nm -D` C-vs-Rust, and runs the whole suite in the `dev` **and** `release` profiles |
| `mutation_check.sh` | proves the suite discriminates: injects 19 small bugs into `src/lib.rs` one at a time and requires every one to be caught (19/19), then restores the file |

### Important harness detail

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library: the
integration tests do not reach it through the crate graph, so Cargo happily
leaves a stale `target/<profile>/libdriver.so` in place. Loading that file
directly makes the whole differential suite vacuous — it was silently passing
against a `.so` built 15 minutes earlier, and `mutation_check.sh` reported
**0/12 caught**. `tests/common/mod.rs::rust_so_path()` therefore shells out to
`cargo build --lib` with a separate `CARGO_TARGET_DIR`
(`target/difftest-so/`, so there is no lock contention with the outer
`cargo test`) and loads *that* `.so`, guaranteeing it matches the current
sources, profile and feature set.
