# SYMBOLS.md — Exported-symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-c7k0pt.so`
- Rust `.so`: `translation/target/release/libmd5_digest_lib.so`

## C source inventory (completeness check)

The whole library is a single translation unit; `CMakeLists.txt` compiles
exactly `src/lib.c`, and `include/lib.h` is the only public header.

```
c_src/CMakeLists.txt   -> add_library(<proj> SHARED src/lib.c)
c_src/include/lib.h    -> 14 lines: 2 typedefs, 1 struct, 1 function decl
c_src/src/lib.c        -> 21 lines: 1 function definition
```

There are no other `.c` files, no `#ifdef`-gated alternate implementations, no
name-mangling / namespace macros, and no macro-generated symbol families. So
the source-level name is the final linker name, and no C module was skipped by
the translation.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `md5_digest` | `T` (defined, global text) | `T` (defined, global text) | MATCH |

## Diff

```
$ nm -D --defined-only <c.so>   | awk '{print $3}' | sort > /tmp/c.syms
$ nm -D --defined-only <rust.so>| awk '$2=="T"{print $3}' | sort > /tmp/r.syms
$ comm -23 /tmp/c.syms /tmp/r.syms      # in C, missing from Rust
<empty>
```

**Missing from Rust: 0.** No `#[no_mangle]` wrapper needed to be added and no
untranslated C module was found.

## Undefined-symbol audit of the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only:

- glibc imports (`memcpy`, `malloc`, `free`, `abort`, `write`, `mmap64`, ...)
- `_Unwind_*` from `libgcc` (panic machinery)
- weak optional hooks (`__gmon_start__`, `_ITM_*`, `__cxa_finalize`, `statx`,
  `gettid`, `__cxa_thread_atexit_impl`)

**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one (which is also
`--no-default-features`). Verified:

```
$ grep -c '\[features\]' translation/Cargo.toml
0
```

Both `cargo test` and `cargo test --no-default-features` are run in Phase D and
exercise the identical code path; there is no other combination to enumerate.
