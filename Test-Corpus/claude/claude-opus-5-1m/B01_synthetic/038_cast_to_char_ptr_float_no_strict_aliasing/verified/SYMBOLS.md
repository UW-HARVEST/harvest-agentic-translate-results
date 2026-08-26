# SYMBOLS.md — Phase A symbol surface

`c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`, so the C
project has **no** shared-library target of its own.  To obtain a comparable
symbol surface the same single translation unit is additionally compiled as a
shared object (this does not modify anything in `c_src/`):

```
gcc -shared -fPIC -fno-strict-aliasing -o cbuild/libcdriver.so c_src/src/main.c
```

The Rust side gained a `[lib] crate-type = ["cdylib"]` target (`src/lib.rs`)
that re-exports the same C ABI, so both `.so` files can be `dlopen`ed side by
side by the differential tests.  The executable behaviour is unchanged: the
implementation lives in `src/scan.rs` and is shared verbatim by the `driver`
binary (`src/main.rs`) and the cdylib (`src/lib.rs`).

## Defined (exported) symbols

`nm -D --defined-only` on each `.so`, ignoring Rust name-mangled (`_ZN…`,
`_R…`) and Rust-runtime (`__rust_*`, `rust_*`) internals, plus the standard
crt/ITM weak symbols that both toolchains emit.

| # | C symbol (`cbuild/libcdriver.so`) | C source | present in `target/release/libdriver.so` | Rust definition |
|---|-----------------------------------|----------|------------------------------------------|-----------------|
| 1 | `driver` (`T`) | `void driver(float x)` — `c_src/src/main.c:34` | **yes** (`T driver`) | `#[no_mangle] pub extern "C" fn driver(x: f32)` — `src/lib.rs` |
| 2 | `main`   (`T`) | `int main(void)` — `c_src/src/main.c:40` | **yes** (`T main`) | `#[no_mangle] pub extern "C" fn main() -> c_int` — `src/lib.rs` |

`print_hex` (`c_src/src/main.c:27`) is declared `static` and is therefore
**not** exported by the C `.so`; the Rust counterpart (`scan::print_hex`) is
likewise a private module function.  No symbol is missing and nothing had to be
stubbed: every C symbol has a real translated implementation behind it.

## Undefined (imported) symbols

| C `.so` import | kind | Rust `.so` equivalent |
|----------------|------|-----------------------|
| `__isoc99_scanf@GLIBC_2.7` | libc | emulated in pure Rust (`scan::scan_float`) — see `src/scan.rs` header comment |
| `printf@GLIBC_2.2.5`       | libc | `write!`/`writeln!` on `io::stdout()` |
| `putchar@GLIBC_2.2.5`      | libc | (gcc's strength-reduction of `printf("\n")`) — `writeln!` |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | weak crt/ITM | present as weak crt symbols in the Rust `.so` too |

Everything the Rust `.so` leaves undefined is libc / libgcc / ld.so
(`memcpy`, `write`, `pthread_*`, `__libc_start_main`, …) — i.e. **0 missing or
undefined non-libc symbols**.

## Verification

`tests/symbol_parity.rs` re-derives both symbol sets with `nm -D` at test time
and asserts the C→Rust difference is empty (`test_c_exports_are_all_present_in_rust`),
so the table above cannot silently rot.  Checked by hand as well:

```
$ comm -23 <(nm -D --defined-only cbuild/libcdriver.so   | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort -u)
            <- empty: full parity
```

`main` is only exported by the cdylib, not by the `rlib`/bin path: it carries
`#[cfg(not(test))]` because `cargo test` compiles `src/lib.rs` with `--test`,
which generates its own `main` symbol.

Note that plain `cargo test` does not emit the cdylib artifact (only
`cargo build` does), so `tests/common/mod.rs::rust_shared_lib` falls back to
compiling the identical `src/lib.rs` into a cdylib with `rustc` directly
(`cbuild/libdriver_rustc_{debug,release}.so`).  Both variants were verified to
export the same two symbols.  `tools/build_all.sh` builds the cargo variant up
front so the tests use it.

## Build-time configurations

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` has no
options, `#ifdef`s or `option()`s (the only compile flag is
`-fno-strict-aliasing`).  `c_src/src/main.c` contains no preprocessor
conditionals at all:

```
$ grep -cE '^\s*#\s*(if|ifdef|ifndef|else|elif|endif)' c_src/src/main.c
0
```

The complete set of valid feature combinations is therefore the single empty
set.  `tools/check_features.sh` enumerates it mechanically and runs
`cargo check` for `--no-default-features`, the default, and `--all-features`
(all three are the same configuration here, and all three are checked).
