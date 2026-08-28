# SYMBOLS.md — Phase A: public symbol surface

## Method

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D --defined-only c_src/build/libharvest-work-4UINGg.so
nm -D --defined-only translation/target/release/libwcscat_lib.so
```

## C translation units

The whole C library is ONE translation unit; nothing was skipped.

| C file | public symbols it defines |
|--------|---------------------------|
| `c_src/src/lib.c` | `wcscat` |
| `c_src/include/lib.h` | (declaration only, no definitions, no renaming macros) |

`grep -n '^[a-zA-Z].*(' c_src/src/lib.c` yields exactly one function definition,
and the header declares exactly that one prototype. There are no
`#define`-generated symbol names, no `static` helpers promoted to externals, and
no additional `.c` files listed in `add_library(...)` in `CMakeLists.txt`.

## Symbol table (every `nm -D` global from the C `.so`)

| # | symbol | C `.so` | Rust `.so` | kind | notes |
|---|--------|---------|------------|------|-------|
| 1 | `wcscat` | `T` (0x10f9) | `T` (0x11c30) | text/global func | `int wcscat(wchar_t*, size_t, const wchar_t*)`; exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn wcscat` |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-4UINGg.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libwcscat_lib.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust `.so`: 0.**
**Extra non-libc/non-runtime symbols in Rust `.so`: 0.**

### Undefined (imported) symbols

| `.so` | undefined non-libc symbols |
|-------|----------------------------|
| C | none (leaf translation unit; no libc calls at all) |
| Rust | none (only the usual Rust/libc runtime imports pulled in by `cdylib`, none of which are library API) |

## Name-collision note (important for the harness)

`wcscat` is *also* a glibc symbol (`wchar.h`, 2-argument `wchar_t *wcscat(wchar_t
*, const wchar_t *)`). The library deliberately interposes that name with a
different 3-argument, `int`-returning signature.

Both `.so`s are therefore loaded with `RTLD_LOCAL` (libloading's default) and the
symbol is looked up **per handle** with `dlsym(handle, "wcscat")`, which searches
the handle's own object first. This was verified with `dladdr()`:

```
resolved from: c_src/build/libharvest-work-4UINGg.so
```

so the differential tests really do compare the library's `wcscat` against the
Rust `wcscat`, never against glibc's.

## Platform ABI facts pinned by the tests

| item | value on this target (`x86_64-unknown-linux-gnu`, gcc 11.5) | Rust side |
|------|--------------------------------------------------------------|-----------|
| `sizeof(wchar_t)` | 4 | `pub type wchar_t = i32` (`cfg(not(windows))`) |
| `wchar_t` signedness | signed (`(wchar_t)-1 < 0` is true) | `i32` |
| `size_t` | 8 bytes, unsigned | `usize` |
| return type | `int`, 4 bytes | `core::ffi::c_int` |

## Cargo feature surface

`translation/Cargo.toml` has **no `[features]` section**, so the only feature
combination that exists is the default (empty) one:

| combo | command |
|-------|---------|
| default | `cargo test` |
| no-default-features | `cargo test --no-default-features` (identical: nothing is gated) |
| all-features | `cargo test --all-features` (identical) |

`grep -rn 'feature *=' translation/src/` → no hits, i.e. no `#[cfg(feature …)]`
in the source, confirming a single code path.

The only `cfg` in the crate is `cfg(windows)` / `cfg(not(windows))` selecting the
`wchar_t` width, which is target-driven, not feature-driven, and matches the C
`stddef.h` definition on this target.
