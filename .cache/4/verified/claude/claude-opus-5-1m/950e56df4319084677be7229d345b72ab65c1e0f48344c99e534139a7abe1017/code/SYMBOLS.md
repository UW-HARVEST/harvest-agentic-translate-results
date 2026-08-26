# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cargo build --offline            # -> target/debug/libdriver.so
```

## C translation-unit inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source | lines | translated to | status |
|----------|-------|---------------|--------|
| `c_src/src/driver.c`     | 40 | `src/lib.rs` | fully translated |
| `c_src/include/driver.h` | 28 | `src/lib.rs` (signature) | fully translated |

No C source file is missing from the Rust translation, so no module had to be
newly translated in this phase.

## Defined (exported) symbols

`nm -D --defined-only` on each `.so`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` (0x1173, size 37) | `T` (0x12250) | `void driver(int x)` from `driver.h` |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

### Deliberately NOT exported

| C symbol | linkage in C | Rust |
|----------|--------------|------|
| `print_hex` | `static` (file-local, absent from `.dynsym`) | private `fn print_hex`, not exported — correct |

## Undefined symbols (must all be libc/toolchain)

* C `.so` undefined: `printf`, `putchar` (both `@GLIBC_2.2.5`) + the 4 standard
  weak CRT symbols (`_ITM_*TMCloneTable`, `__cxa_finalize`, `__gmon_start__`).
  Note `printf("\n")` was lowered by the C compiler to `putchar` — the Rust
  translation calls the same two libc entry points, so buffering and the emitted
  byte stream are identical.
* Rust `.so` undefined: 52 entries, **all** of them glibc (`printf`, `putchar`,
  `memcpy`, `malloc`, `write`, …) or the libgcc unwinder (`_Unwind_*`).
  There are **0 non-libc / non-toolchain undefined symbols**.
* `ldd -r` on both `.so` files reports no unresolved symbols.

## Result

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
