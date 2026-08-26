# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```sh
# C shared library
cd translated_rust/c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust shared library (crate-type = ["cdylib"])
cd translated_rust && cargo build --offline
# -> target/debug/libdriver.so
```

## C source inventory (completeness check)

Every non-build file under `c_src/`:

| file | translated to | status |
|------|---------------|--------|
| `c_src/CMakeLists.txt` | (build system, no options/`#define`s) | n/a |
| `c_src/include/driver.h` | declaration of `driver` | covered |
| `c_src/src/driver.c` | `src/lib.rs` (`driver`) | fully translated |

`c_src` contains exactly one translation unit and one public declaration, so no
C module was skipped (`find c_src -type f | grep -v build/` → 3 files above).

## `nm -D --defined-only` — C `.so`

```
0000000000001109 T driver
```

## `nm -D --defined-only` — Rust `.so`

```
00000000000122a0 T driver
```

## Symbol parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `driver` | `T` | `T` | `#[unsafe(no_mangle)] pub extern "C" fn driver(c_int, c_int)` |

**Missing from Rust `.so`: none. Symbol diff is EMPTY.**

## Undefined (imported) symbols

Both libraries import only libc / runtime symbols, no cross-library
dependencies:

* C `.so` imports: `puts@GLIBC_2.2.5` (gcc rewrites every
  `printf("literal\n")` into `puts("literal")` — at *all* optimization levels,
  including the reference build's implicit `-O0`), plus the usual weak
  `_ITM_*`, `__cxa_finalize`, `__gmon_start__`.
* Rust `.so` imports: `puts@GLIBC_2.2.5` — deliberately the *same* libc entry
  point as the reference library, so that the `write(2)` framing matches too
  (see `CONFIGS.md` row 22) — plus the standard Rust-std/libgcc set
  (`_Unwind_*`, `malloc`, `memcpy`, `pthread_*`, …). All are libc / unwinder
  symbols; **0 missing/undefined non-libc symbols**.

## Verification script

`./check_all.sh` regenerates both `.so`s and re-diffs the symbol lists for
every feature combination (see `CONFIGS.md` §Feature combinations). Result:

```
=== [combo1] symbol parity ===   (--no-default-features)
OK: all 1 C symbol(s) exported by the Rust .so
OK: no non-libc undefined symbols
=== [combo2] symbol parity ===   (default)
OK: all 1 C symbol(s) exported by the Rust .so
OK: no non-libc undefined symbols
=== [combo3] symbol parity ===   (--all-features)
OK: all 1 C symbol(s) exported by the Rust .so
OK: no non-libc undefined symbols
```

`target/release/libdriver.so` (profile with `panic = "abort"`) also exports
`driver` and passes the full suite.

Checklist:

* [x] `nm -D` shows 0 symbols missing from the Rust `.so` (`comm -23` diff empty).
* [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
* [x] No C source file was left untranslated (inventory above).
