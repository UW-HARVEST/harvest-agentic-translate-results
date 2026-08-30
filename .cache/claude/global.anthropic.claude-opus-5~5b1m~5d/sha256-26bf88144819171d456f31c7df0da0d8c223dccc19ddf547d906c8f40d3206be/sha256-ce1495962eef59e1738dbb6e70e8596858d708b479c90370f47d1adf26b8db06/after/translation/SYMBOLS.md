# SYMBOLS.md — Public symbol surface (Phase A / Phase D)

Source of truth: `nm -D` on the C shared library built from `c_src/`.

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D libSieve.so
```

## C `.so` dynamic symbol table (`c_src/build/libSieve.so`)

| symbol | type | kind | must Rust export? |
|--------|------|------|-------------------|
| `sieve` | `T` (defined, global text) | **public API** (declared in `include/sieve.h`) | **YES** |
| `printf` | `U` (undefined) | imported from libc | no (import, not export) |
| `__cxa_finalize` | `w` (weak undefined) | C runtime / glibc | no |
| `__gmon_start__` | `w` (weak undefined) | profiling hook | no |
| `_ITM_registerTMCloneTable` | `w` (weak undefined) | GCC TM clone hook | no |
| `_ITM_deregisterTMCloneTable` | `w` (weak undefined) | GCC TM clone hook | no |

The complete *exported* (defined, `T`/`D`/`B`) surface of the C library is
therefore a single symbol: **`sieve`**.

`include/sieve.h` declares exactly one entry point:

```c
void sieve(int start);
```

There are no other translation units in `c_src/` (`src/sieve.c` is the only
`.c` file in `CMakeLists.txt`), no macro-generated symbol families, no
exported globals, and no `#ifdef`-gated alternate symbol names (the only
preprocessor conditional in the whole project is the `SIEVE_H_` include
guard). So there is no missing/untranslated module.

## Rust `.so` dynamic symbol table (`translation/target/<profile>/libSieve.so`)

Exported (defined) non-libc symbols:

| symbol | type | provided by |
|--------|------|-------------|
| `sieve` | `T` | `#[unsafe(no_mangle)] pub extern "C" fn sieve(val: c_int)` in `src/lib.rs` |

`[lib] name = "Sieve"` + `crate-type = ["cdylib"]` makes the artifact
`libSieve.so`, matching the C target name.

## Symbol diff (Phase D gate)

```
comm -23 <(c_exported_defined_symbols) <(rust_exported_defined_symbols)
```

| direction | result |
|-----------|--------|
| in C `.so` but **missing** from Rust `.so` | **(empty)** |
| undefined (`U`) non-libc symbols in Rust `.so` | **(empty)** — all `U` entries resolve to `libc.so.6` / `libgcc_s` (`printf`, `memcpy`, `malloc`, `_Unwind_*`, …) |

Automated check: `tests/symbols.rs::c_exports_are_all_present_in_rust`
shells out to `nm -D` on both libraries at test time and fails if the diff is
non-empty, so this gate is re-verified on every `cargo test` run.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` and the default
build are byte-identical in terms of code paths — there is nothing to gate).
Phase D's "repeat B–C for every feature combination" is therefore satisfied by
the single default combination; `tests/features.rs` asserts that no feature
axes have been introduced, so this claim cannot silently rot.
