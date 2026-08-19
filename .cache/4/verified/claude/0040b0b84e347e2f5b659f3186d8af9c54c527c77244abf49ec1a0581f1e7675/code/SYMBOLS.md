# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared libraries.

## How the two `.so` files are produced

* C: `gcc -shared -fPIC -fno-strict-aliasing -o cbuild/libdriver_c.so c_src/src/main.c`
  (`-fno-strict-aliasing` is the flag `c_src/CMakeLists.txt` puts on the
  translation unit; `CMakeLists.txt` itself only builds the *executable*
  `add_executable(driver src/main.c)`, so the shared-library variant is built
  with the same single translation unit and the same flag. `c_src/` is not
  modified.)
* Rust: `cargo build` → `target/debug/libdriver.so`
  (`[lib] crate-type = ["cdylib"]` in `Cargo.toml`).

The Rust crate has **two** targets that share one implementation file
(`src/imp.rs`), mirroring the two C build products:

| C build product | Rust target |
|---|---|
| `add_executable(driver src/main.c)` → `driver` | `[[bin]] driver` (`src/main.rs`, `#![no_main]`, so the `#[no_mangle] extern "C" fn main` **is** the ELF entry point, exactly like C's `int main()`) |
| `gcc -shared … main.c` → `libdriver_c.so` | `[lib] crate-type=["cdylib"]` (`src/lib.rs` → `libdriver.so`) |

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so`:

```
0000000000001193 T driver
00000000000011ec T main
```

`nm -D --defined-only` on the Rust `.so`:

```
0000000000017e00 T driver
0000000000017e20 T main
```

| # | C symbol | type | C declaration | exported by Rust `.so`? | Rust item |
|---|----------|------|---------------|--------------------------|-----------|
| 1 | `driver` | `T` (global text) | `void driver(int floors)` | YES | `#[no_mangle] pub extern "C" fn driver(floors: c_int)` in `src/imp.rs` |
| 2 | `main`   | `T` (global text) | `int main()`              | YES | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/imp.rs` |

**Symbol diff (C defined − Rust defined): EMPTY.** ✅

### Not exported (and correctly so)

| C entity | why absent from both `.so` files |
|---|---|
| `static void print_hex(unsigned char *p, int len)` | `static` → internal linkage, never in `.dynsym`. Translated as the private `fn print_hex` in `src/imp.rs`. |
| `typedef struct { … } house_t` | a type, not a symbol. Translated as the private `#[repr(C)] struct House`. |

## Undefined symbols

C `.so` imports (all libc):
`__isoc99_scanf@GLIBC_2.7`, `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`
(gcc rewrote `printf("\n")` into `putchar`), plus the standard weak
`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__`,
`__cxa_finalize`.

Rust `.so` imports: only libc (`read`, `write`, `writev`, `malloc`, `memcpy`,
`open64`, …), libgcc unwinder (`_Unwind_*@GCC_*`), and the same standard weak
symbols. `ldd target/debug/libdriver.so` resolves everything
(`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`) — **0 unresolved /
non-libc undefined symbols**. ✅

## Verification commands

```sh
gcc -shared -fPIC -fno-strict-aliasing -o cbuild/libdriver_c.so c_src/src/main.c
cargo build
diff <(nm -D --defined-only cbuild/libdriver_c.so     | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort)
# -> no output (identical export sets)
```

This diff is asserted automatically by three tests:

| test | what it checks |
|---|---|
| `differential.rs::symbol_parity_nm_defined_only` | `nm -D --defined-only` export sets are **identical** (and non-empty) |
| `differential.rs::symbol_dlsym_both_libs` | `dlopen` + `dlsym("driver")`/`dlsym("main")` succeed on **both** libraries, through `libloading` |
| `differential.rs::symbol_no_unresolved_in_rust_so` | `ldd` reports no `not found` dependency for the Rust `.so` |
| `inprocess.rs` | both libraries are `dlopen`ed into one process and both symbols are resolved before any call |

Verified in both the dev and the release profile:

```
$ nm -D --defined-only target/release/libdriver.so
driver main
$ nm -D --defined-only target/release/deps/libdriver.so
driver main
```

### Completeness

`c_src/` contains exactly one translation unit (`src/main.c`, 55 lines) and it is
fully translated: 2/2 exported functions, 1/1 static function, 1/1 struct type.
No C file, function, or symbol was skipped, so no symbol needed to be stubbed.
