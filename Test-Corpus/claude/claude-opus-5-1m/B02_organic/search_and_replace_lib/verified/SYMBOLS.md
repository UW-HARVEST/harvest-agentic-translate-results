# SYMBOLS.md — Public ABI surface (Phase A)

Derived mechanically from `nm -D` on both shared libraries.

```
C   .so : c_src/build/libdriver.so   (cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)
Rust.so : target/release/libdriver.so (cargo build --release --offline)
```

## Source inventory (completeness check)

The C build (`c_src/CMakeLists.txt`) compiles exactly one translation unit:

| C source file | lines | translated to |
|---------------|-------|---------------|
| `c_src/src/lib.c`       | 90 | `src/lib.rs` |
| `c_src/include/lib.h`   | 1  | (declaration only — no code) |

There are **no** `#ifdef` / `#if` / `#define` directives in the C sources and no
`option()` / `add_definitions()` / `target_compile_definitions()` in
`CMakeLists.txt`, so there is exactly **one** C build configuration and no
namespace-renaming macros to account for.
`Cargo.toml` has **no `[features]` section**, so the Rust crate likewise has
exactly one build configuration (the empty feature set); `cargo check` and
`cargo check --no-default-features` are the complete set of feature
combinations (see `CONFIGS.md` §0).

## Exported (defined, dynamic) symbols

`nm -D --defined-only`:

| # | C symbol | type | exported by Rust `.so` | notes |
|---|----------|------|------------------------|-------|
| 1 | `searchAndReplace` | `T` (global text) | YES — `T searchAndReplace` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn`, signature `char *(const char*, const char*, const char*)` |

**Symbol diff (C defined − Rust defined): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `realloc`, `strdup`, `strlen`,
`strncpy`, `strstr` (plus the weak CRT symbols `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`).

The Rust `.so` imports **the same six libc functions** — `malloc`, `realloc`,
`strdup`, `strlen`, `strncpy`, `strstr` — because the translation calls the very
same libc primitives the C calls (guaranteeing identical `strstr`/`strncpy`
corner-case behaviour, identical allocator, and identical faulting behaviour on
NULL input in every build profile). On top of that it imports the usual Rust
runtime set (`memcpy`, `memmove`, `memset`, `bcmp`, `free`, `calloc`,
`posix_memalign`, `abort`, `_Unwind_*`, `dl_iterate_phdr`, pthread TLS helpers
and the file/IO syscalls used by the panic-backtrace machinery).

**Undefined non-libc / non-runtime symbols in the Rust `.so`: 0.** Verified by
loading the Rust `.so` with `libloading` (which performs full relocation
resolution) in every test — a missing dependency would fail the `Library::new`
call.

## Verification command

```sh
nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort > /tmp/c.syms
nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms     # must be empty
```
