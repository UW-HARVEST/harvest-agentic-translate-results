# SYMBOLS.md — Phase A: public symbol surface

Source of truth: `nm -D --defined-only` on the C shared library built from
`c_src/` (`libString_Slice.so`), compared against the Rust `cdylib`
(`translation/target/release/libString_Slice.so`).

## C `.so` translation units

The whole library is a single translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/slicing.c` | `translation/src/lib.rs` | fully translated |

There are no untranslated C source files, so no module is missing and no
symbol needed to be newly translated for this phase.

## Exported (defined, dynamic) symbols

Commands used:

```
nm -D --defined-only c_src/build/libString_Slice.so
nm -D --defined-only translation/target/release/libString_Slice.so
```

| # | symbol | declared in | C `.so` | Rust `.so` | notes |
|---|--------|-------------|---------|-----------|-------|
| 1 | `slice` | `c_src/include/slicing.h` | `T slice` | `T slice` | `int slice(char *mystr, int *start_ptr, int *stop_ptr)`; exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn slice` |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libString_Slice.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/.../libString_Slice.so | awk '{print $NF}' | sort)
(empty)
```

**Result: 0 missing symbols.** Every symbol the C `.so` exports is exported by
the Rust `.so` under the exact same name.

Note: `char *end;` in `slice()` is a declared-but-unused local in the C source
and produces no symbol; the Rust translation correctly omits it.

## Undefined (imported) symbols

The Rust `.so` must not depend on any non-libc symbol that the C `.so` does not.

| symbol | C `.so` | Rust `.so` | libc? |
|--------|---------|-----------|-------|
| `printf` | yes | yes | yes (`libc.so.6`) |
| `strlen` | yes | yes | yes (`libc.so.6`) |
| `__stack_chk_fail` / `_ITM_*` / `__gmon_start__` / `__cxa_*` | toolchain glue | toolchain glue | yes |

**Result: 0 undefined non-libc symbols in the Rust `.so`.**

The Rust translation deliberately calls the platform `printf` (rather than
Rust's `std::io::stdout`) so that stdout buffering and the exact emitted bytes
are identical to the C library's.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (the default). `--no-default-features` and any
`--features <combo>` therefore resolve to the same code, and the symbol table
above is the complete surface for every configuration.
