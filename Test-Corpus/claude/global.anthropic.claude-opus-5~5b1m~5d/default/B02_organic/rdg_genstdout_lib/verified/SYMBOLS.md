# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported symbols (`nm -D --defined-only c_src/build/libdriver.so`)

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `T` (global text) | `c_src/src/lib.c:20` |
| 2 | `extractFilename`               | `T` (global text) | `c_src/src/lib.c:7`  |

There are no macro-generated symbols, no exported data symbols, and no
`static` helpers promoted to global linkage in `c_src/src/lib.c`.
`c_src/include/lib.h` declares only `FIO_createFilename_fromOutDir`;
`extractFilename` is *not* declared in the header but has external linkage
(it is not `static`), so it is part of the ABI and must be exported by Rust
too.

## Rust `.so` exported symbols (`nm -D --defined-only translation/target/release/libdriver.so`)

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `T` (global text) | `translation/src/lib.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn`) |
| 2 | `extractFilename`               | `T` (global text) | `translation/src/lib.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn`) |

## Diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: NONE.** No stubs, no `unimplemented!()`; both symbols are
genuine translations of the C bodies. No C source file in `c_src/` was left
untranslated (`src/lib.c` is the only translation unit).

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, which is required for ABI
compatibility (the returned buffer must be `free()`-able by the caller, so it
must come from libc `calloc`):

| symbol | why |
|--------|-----|
| `calloc` | allocate the result buffer with the same allocator as C, so the caller can `free()` it |
| `exit`   | `exit(30)` on allocation failure |
| `fprintf`, `stderr` | the diagnostic message on allocation failure |
| `strerror`, `__errno_location` | `strerror(errno)` in that message |
| `strlen`, `strrchr`, `memcpy` | the exact same libc routines the C calls |

`strlen`/`strrchr`/`memcpy` are deliberately **imported from libc rather than
re-implemented in Rust**. A hand-written Rust `strlen` trips Rust's raw-pointer
UB checks under `-C debug-assertions` and `abort()`s (SIGABRT) where the C
faults (SIGSEGV) — a real, test-detected divergence on the null-pointer paths
(`ERRORS.md` E4/E8/E9/E10). Calling the identical libc symbols removes that
whole class of divergence. The rest of the undefined symbols come from the Rust
standard library runtime (`_Unwind_*`, `mmap64`, `pthread_key_*`, …), all of
which are provided by libc/libgcc; `tests/phase_d_symbols.rs::d3` asserts that
none of them is an unresolved Rust-internal (`_ZN…`/`_R…`) symbol.

0 missing / 0 undefined non-libc symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
feature combination that exists is the (empty) default one. Phases B and C are
consequently run once, and additionally re-run with
`--no-default-features` to prove the empty-feature-set build is identical.
