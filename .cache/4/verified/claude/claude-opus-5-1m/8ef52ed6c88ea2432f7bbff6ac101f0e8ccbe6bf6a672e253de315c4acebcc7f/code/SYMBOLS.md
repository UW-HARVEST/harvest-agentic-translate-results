# SYMBOLS.md — symbol surface parity (Phase A / Phase D)

## How the two shared libraries are produced

`c_src/CMakeLists.txt` only declares

```cmake
add_executable(driver src/main.c)
```

i.e. the C project is a **single-translation-unit executable** (`c_src/src/main.c`,
6 statements). It has no `add_library`, no `target_compile_definitions`, no
`option()`, and the source contains no `#ifdef`. To make the C code loadable for
differential testing it is compiled — without modifying anything in `c_src/` —
as a position-independent shared object:

```sh
gcc -shared -fPIC -o target/difftest/libdriver_c.so   c_src/src/main.c      # -O0, matches the CMake default build type
gcc -shared -fPIC -O2 -o target/difftest/libdriver_c_O2.so c_src/src/main.c # optimised variant
gcc -o target/difftest/driver_c c_src/src/main.c                            # same as `cmake --build .`
```

The Rust side is built as both a `cdylib` and a `bin` from the same
`src/driver_core.rs`:

```sh
cargo build --offline --lib   # target/debug/libdriver.so
cargo build --offline --bin driver
```

## `nm -D --defined-only` — C shared library (ground truth)

| # | symbol | type | C declaration |
|---|--------|------|---------------|
| 1 | `main` | `T` (global text) | `int main()` |
| 2 | `printHexCharLine` | `T` (global text) | `void printHexCharLine (char charHex)` |

That is the complete list — the C `.so` exports exactly 2 non-weak, non-libc
dynamic symbols.

## `nm -D --defined-only` — Rust `cdylib` (`target/debug/libdriver.so`)

| # | symbol | type | Rust definition (`src/lib.rs`) |
|---|--------|------|--------------------------------|
| 1 | `main` | `T` | `#[no_mangle] pub extern "C" fn main() -> c_int` |
| 2 | `printHexCharLine` | `T` | `#[no_mangle] pub extern "C" fn printHexCharLine(charHex: c_int)` |

`printHexCharLine` takes a `c_int` and truncates with `as i8` on purpose. On the
x86-64 SysV ABI a `char` argument travels in a 32-bit register whose upper 24
bits are *undefined*, and gcc's callee re-derives the value from the low byte
only:

```asm
printHexCharLine:
    movsbl %dil,%esi        ; low byte, sign extended
    ...
    jmp printf@plt
```

Taking `c_int` + `as i8` reproduces that byte-exactly for every possible
register content and is ABI-identical to a `char` parameter (verified by the
`wide_int_*` tests in `tests/differential.rs`).

## Diff

```text
symbols in C .so but NOT in Rust .so:   (none)
symbols in Rust .so but NOT in C .so:   (none)
```

Verified mechanically by `tests/symbols.rs` (4 tests:
`c_so_exports_exactly_main_and_print_hex_char_line`,
`every_c_symbol_is_exported_by_the_rust_so`,
`rust_so_has_no_unresolved_non_libc_symbols`,
`both_symbols_are_dlsym_able_in_both_libraries`) and by `./verify.sh`, which runs

```sh
diff <(nm -D --defined-only libdriver_c.so | awk '{print $NF}' | sort) \
     <(nm -D --defined-only libdriver.so   | awk '{print $NF}' | sort)
```

## Undefined (imported) symbols

Nothing outside libc/libgcc is left undefined in either library.

| library | undefined symbols |
|---------|-------------------|
| C `.so` | `printf`, `__isoc99_fscanf`, `stdin` (glibc) + the usual weak `_ITM_*`, `__gmon_start__`, `__cxa_finalize` |
| Rust `.so` | glibc (`malloc`, `memcpy`, `write`, `read`, `open64`, …), libgcc unwinder (`_Unwind_*`) + the usual weak `_ITM_*`, `__gmon_start__` |

No missing translation units: `c_src/src/main.c` is the only C source file in
the project (`find c_src -name '*.c' -o -name '*.h'` → 1 file), and both of its
functions are translated in `src/driver_core.rs` and exported from `src/lib.rs`.
Nothing is stubbed or `unimplemented!()`.
