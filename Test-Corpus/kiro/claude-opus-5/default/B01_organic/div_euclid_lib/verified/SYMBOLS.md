# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-RAYZOX.so` (built via `c_src/CMakeLists.txt`,
  which compiles exactly one translation unit: `src/lib.c`)
* Rust `.so`: `translation/target/release/libdiv_euclid_lib.so`
  (`[lib] crate-type = ["cdylib"], name = "div_euclid_lib"`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/libharvest-work-RAYZOX.so
nm -D --defined-only translation/target/release/libdiv_euclid_lib.so
```

## Defined dynamic symbols exported by the C `.so`

| # | symbol | type | declared in | exported by Rust `.so`? |
|---|--------|------|-------------|-------------------------|
| 1 | `div_euclid` | `T` (global text) | `c_src/include/lib.h:1` — `int div_euclid(int v1, int v2);` | YES — `#[unsafe(no_mangle)] pub extern "C" fn div_euclid`, `T` |

`c_src/include/lib.h` declares no other prototype, `c_src/src/lib.c` defines no
other function, and there are no macros that generate additional symbol names
(no `#define`-based name mangling anywhere in `c_src/`). So the C surface is a
single symbol, and there is no untranslated C module: `src/lib.c` is fully
covered by `translation/src/lib.rs`.

## Symbol diff

```
C exports not exported by Rust:   (none)
```

**0 missing symbols.**

## Undefined (imported) symbols

The C `.so` imports only the four weak glibc/GCC housekeeping symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`).

The Rust `.so` imports that same set plus libc (`malloc`, `memcpy`, `write`,
`open64`, `pthread_*`, …) and the `_Unwind_*` personality routines pulled in by
the Rust standard library / panic machinery. Every one of these resolves from
`libc`/`libgcc_s` at load time.

```
Rust undefined non-libc / non-unwind symbols: (none)
```

The Rust `.so` therefore loads with `dlopen` and resolves `div_euclid` with no
unresolved application-level dependency, which the integration tests confirm by
actually `dlopen`-ing it.

## Additional Rust-local dynamic symbols

The Rust `cdylib` additionally exposes some lowercase/`t`-type local and
Rust-std-internal entries (allocator shims, panic hooks). These are *extra*
symbols, not missing ones, and are irrelevant to ABI parity: the requirement is
that every C symbol is present in Rust, which holds.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set. `--no-default-features` and
the default build are the same compilation. Verified by
`scripts/verify.sh`, which parses the `[features]` table out of `Cargo.toml`,
enumerates the power set of feature combinations (finding only the empty set
here), and for each one runs `cargo check`, `cargo build --lib`, `cargo test`
and the `nm -D` symbol diff in both the dev and release profiles.

Both `default` and `--no-default-features` were run explicitly; the symbol diff
is empty and all 73 tests pass in each of the 4 (combination × profile) cells.
