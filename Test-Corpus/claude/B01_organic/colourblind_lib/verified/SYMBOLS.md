# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so   (name comes from parent dir via cmake_path)

# Rust
cargo build --release
# -> target/release/libcolourblind_lib.so   ([lib] name = "colourblind_lib", crate-type = cdylib)
```

Note: the C `.so` is compiled with `C_FLAGS = -fPIC` only (`CMAKE_BUILD_TYPE` is
empty ⇒ **-O0**, no `-ffast-math`, no `-mfma`). Verified via
`c_src/build/CMakeFiles/translated_rust.dir/flags.make`.

## `nm -D` raw output

### C — `c_src/build/libtranslated_rust.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000013d2 T colourblind
```

### Rust — `target/release/libcolourblind_lib.so`

```
0000000000011df0 T colourblind
```

## Symbol parity table

Only `T`/`D`/`B` (defined, exported) non-libc symbols are in scope. The four `w`
entries in the C `.so` are undefined *weak* glibc/ITM crt hooks
(`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`) emitted by the C runtime startup files, not part of the
library's API surface.

| # | C symbol | type | Rust `.so` exports it? | Rust item |
|---|----------|------|------------------------|-----------|
| 1 | `colourblind` | `T` (global text) | **YES** | `#[unsafe(no_mangle)] pub unsafe extern "C" fn colourblind` in `src/lib.rs` |

### Symbols in C but NOT in Rust

**NONE.** The diff is empty.

### Static (non-exported) C functions — intentionally not exported by either

These are `static` in `c_src/src/lib.c`, so they are *not* in the C `.so`'s
dynamic symbol table and must **not** be exported by Rust either. They are
translated as private Rust `fn`s and are reached only through `colourblind`.

| C static function | Rust counterpart |
|-------------------|------------------|
| `static void Protanopia(float*, float*, float*)`   | `fn protanopia(&mut f32, &mut f32, &mut f32)` |
| `static void Deuteranopia(float*, float*, float*)` | `fn deuteranopia(&mut f32, &mut f32, &mut f32)` |
| `static void Tritanopia(float*, float*, float*)`   | `fn tritanopia(&mut f32, &mut f32, &mut f32)` |

Confirmed present in the C `.so`'s *local* symbol table only:
`nm c_src/build/libtranslated_rust.so | grep ' t '` ⇒ `Protanopia`,
`Deuteranopia`, `Tritanopia`.

### Undefined non-libc symbols in the Rust `.so`

`nm -D --undefined-only target/release/libcolourblind_lib.so` ⇒ only libc /
`ld-linux` imports. No unresolved crate-level symbols.

## Verdict

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] 0 missing symbols; 0 undefined non-libc symbols in Rust.
- [x] No whole C module/file was skipped by the translation — `c_src` contains
      exactly one translation unit (`src/lib.c`, 35 lines) plus one public
      header (`include/lib.h`, 7 lines), and both are fully translated.
- [x] No stubs / `unimplemented!()` anywhere in `src/lib.rs`.

## Automated re-verification

`tests/phase_d_symbols.rs` re-derives both symbol sets with `nm -D
--defined-only` at test time and fails if the Rust `.so` is missing anything the
C `.so` exports. `./verify.sh` additionally does the diff with `comm -23` for
every feature combination and both profiles.

Latest result: **1 C API symbol, 0 missing** in both the `dev` and `release`
profiles.
