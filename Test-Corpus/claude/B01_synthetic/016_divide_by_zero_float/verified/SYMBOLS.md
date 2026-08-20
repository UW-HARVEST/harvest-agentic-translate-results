# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

## How this table was produced

`c_src/CMakeLists.txt` builds `src/main.c` with `add_executable(driver src/main.c)`.
The translation unit itself contains no `main`-only code: every non-`static`
function in it is an ordinary external symbol, so the same translation unit
compiles cleanly into a shared object:

```sh
# C shared object (position independent, default -O0, same as the cmake build)
gcc -fPIC -shared -o build_c/libdriver_c.so c_src/src/main.c -lm

# Rust shared object (crate-type = ["cdylib"] in Cargo.toml)
cargo build --offline --release --no-default-features   # -> target/release/libdriver.so
```

Symbol lists are then taken mechanically from `nm -D`:

```sh
nm -D --defined-only build_c/libdriver_c.so      | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt   # missing from Rust  -> MUST be empty
comm -13 c_syms.txt rust_syms.txt   # extra in Rust
```

Both are regenerated and re-diffed by `tests/differential.rs::symbol_parity_c_so_vs_rust_so`,
so the parity check is enforced by the test suite and not just by this document.

## Dynamic symbols defined by the C `.so`

| # | symbol | C declaration | exported by Rust `.so`? | Rust implementation |
|---|--------|---------------|--------------------------|---------------------|
| 1 | `printLine`    | `void printLine(const char * line)`      | YES | `src/lib.rs::printLine` -> `imp::print_line_bytes` |
| 2 | `printIntLine` | `void printIntLine(int intNumber)`       | YES | `src/lib.rs::printIntLine` -> `imp::print_int_line` |
| 3 | `bad`          | `void bad()`                             | YES | `src/lib.rs::bad` -> `imp::bad` |
| 4 | `good`         | `void good()`                            | YES | `src/lib.rs::good` -> `imp::good` |
| 5 | `main`         | `int main(int argc, char *argv[])`        | YES | `src/lib.rs::main` -> `imp::program_main` |

### Symbol diff result

```
$ comm -23 c_syms.txt rust_syms.txt     # C symbols missing from Rust
(empty)

$ comm -13 c_syms.txt rust_syms.txt     # Rust symbols not present in C
(empty)
```

**0 missing symbols. 0 extra symbols. The diff is empty.**

## Deliberately NOT exported (internal linkage in C)

`nm build_c/libdriver_c.so | grep ' t '` shows these lower-case (local) text
symbols. `goodG2B` and `goodB2G` are declared `static` in the C source, so they
have internal linkage and are *not* part of the dynamic symbol table. The Rust
translation keeps them private (`fn good_g2b`, `fn good_b2g` in `src/imp.rs`)
so it exports neither. The remaining entries are toolchain-generated CRT glue
(`_init`, `_fini`, `frame_dummy`, `register_tm_clones`, …), not translatable
source-level symbols.

| local symbol | origin | Rust equivalent |
|--------------|--------|-----------------|
| `goodG2B` | `static void goodG2B()` in `main.c` | `imp::good_g2b` (private, not exported) |
| `goodB2G` | `static void goodB2G()` in `main.c` | `imp::good_b2g` (private, not exported) |
| `_init`, `_fini`, `frame_dummy`, `register_tm_clones`, `deregister_tm_clones`, `__do_global_dtors_aux` | CRT/toolchain glue | n/a |

## Undefined (imported) symbols

The C `.so` imports only libc: `atof`, `fgets`, `printf`, `puts`, `stdin`
(plus `__cxa_finalize`, `__gmon_start__`, `_ITM_*` glue). Note that GCC
rewrites `printf("%s\n", line)` into `puts(line)`, which is why `puts` appears;
the emitted bytes are identical.

The Rust `.so` imports only libc / libgcc-unwind symbols (`malloc`, `free`,
`memcpy`, `read`, `write`, `writev`, `open64`, `_Unwind_*`, `pthread_key_*`, …).

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Completeness of the translation

The C project consists of exactly one source file (`c_src/src/main.c`,
115 lines, verified with `find c_src -type f`). There is no untranslated
module: `src/imp.rs` covers every function in it —

| C function | translated to |
|---|---|
| `printLine`    | `imp::print_line` / `imp::print_line_bytes` |
| `printIntLine` | `imp::print_int_line` |
| `bad`          | `imp::bad` |
| `goodG2B`      | `imp::good_g2b` |
| `goodB2G`      | `imp::good_b2g` |
| `good`         | `imp::good` |
| `main`         | `imp::program_main` |

The libc facilities the C code relies on (`fgets`, `atof`/`strtod`, the
x86-64 `double`->`int` conversion) are re-implemented in `src/imp.rs` as
`fgets`, `atof` and `double_to_int`. No symbol is stubbed or
`unimplemented!()`.

## A real defect this phase found

Plain `cargo test` originally failed to build at all:

```
error: entry symbol `main` declared multiple times
error: could not compile `driver` (lib test)
```

Exporting `#[no_mangle] extern "C" fn main` (needed for parity with the C `.so`,
which exports `main`) collides with the `main` that libtest generates for the
library's own test harness. Fixed by gating just that one export with
`#[cfg(not(test))]`: the `cdylib` the differential tests `dlopen` is built
without `cfg(test)`, so it still exports `main`, while the lib test target links
cleanly. Verified in both profiles:

```
$ nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[A-Z]$/ {print $2, $3}'
T bad
T good
T main
T printIntLine
T printLine
```

## Results

| item | value |
|---|---|
| symbols exported by the C `.so` | 5 |
| symbols exported by the Rust `.so` | 5 (identical names) |
| missing from Rust | **0** |
| extra in Rust | **0** |
| undefined non-libc symbols in Rust `.so` | **0** |
| verified in profiles | `debug` and `release` |
| enforced by test | `symbol_parity_c_so_vs_rust_so` |
| compiler warnings (`cargo check --all-targets`) | 0 |

`./run_all_configs.sh` re-derives both symbol lists with `nm -D` and re-runs the
`comm` diff for every feature combination and profile; it prints
`symbol diff empty (missing: none, extra: none)` and exits non-zero otherwise.
