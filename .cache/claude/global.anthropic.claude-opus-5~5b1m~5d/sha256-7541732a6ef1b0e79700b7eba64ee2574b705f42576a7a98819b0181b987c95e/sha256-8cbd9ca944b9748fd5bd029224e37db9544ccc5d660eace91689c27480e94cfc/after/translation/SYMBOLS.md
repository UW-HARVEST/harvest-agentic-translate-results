# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The whole C library is two files; there is no untranslated module.

| C file | contents | translated in Rust? |
|--------|----------|---------------------|
| `c_src/include/driver.h` | declares `void driver(int x);` | yes — `translation/src/lib.rs` |
| `c_src/src/driver.c` | `static void print_hex(unsigned char *p, int len)`, `void driver(int x)` | yes — `print_hex` (private fn), `driver` (`#[no_mangle] extern "C"`) |

`print_hex` is `static` in C, therefore it is deliberately NOT a dynamic symbol
in either library. It is kept private (`unsafe fn print_hex`) in Rust. This is
correct parity, not a missing export.

## `nm -D --defined-only` — exported (dynamic) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|--------------|-------|
| 1 | `driver` | `T driver` | `T driver` | `void driver(int x)`; Rust: `#[no_mangle] pub extern "C" fn driver(x: c_int)` |

### Symbol diff

```
diff <(nm -D --defined-only c_src/build/libdriver.so          | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`, 0 extra.

## `nm -D --undefined-only` — imported symbols

C imports (non-weak): `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`.

> `putchar` appears because LLVM/GCC rewrite `printf("\n")` into
> `putchar('\n')`. The Rust `.so` imports the same two symbols for the same
> reason, which is the strongest possible evidence that the Rust translation
> routes its output through the very same libc `stdout` object as the C code
> (not through Rust's `std::io::stdout`, which would buffer separately and
> could reorder output relative to C writers in the same process).

Rust imports the same `printf`/`putchar`, plus the usual Rust `std`/`libunwind`
runtime set (`_Unwind_*`, `malloc`, `free`, `memcpy`, `mmap64`, `pthread_key_*`,
`dl_iterate_phdr`, …). Every one of these is **libc / libgcc_s**, i.e. resolved
by the platform. There are **no undefined non-libc symbols** in the Rust `.so`.

## Verdict

- [x] `nm -D` shows **0 missing** exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 extra** exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 undefined non-libc** symbols in the Rust `.so`.
- [x] No symbol is a stub / `unimplemented!()`; `driver` is a real translation.

## Enforced as a test

Symbol parity is not a one-off manual check; `tests/cases/phase_d_symbols.rs`
shells out to `nm` and asserts it on every run:

| test | asserts |
|------|---------|
| `symbols_exported_by_c_are_all_exported_by_rust` | `C_exports \ Rust_exports == {}` |
| `rust_exports_nothing_extra` | `Rust_exports \ C_exports == {}` (no widened API surface) |
| `rust_has_no_undefined_non_libc_symbols` | every `nm -D --undefined-only` entry in the Rust `.so` is libc / libgcc_s / ld.so |
| `c_source_inventory_is_fully_covered` | the C `.so` still exports exactly 1 symbol, so any future C addition forces `CONFIGS.md` / `ERRORS.md` to be revisited instead of silently going untested |

Additionally `tests/cases/phase_c_errors.rs::err_generic_no_extra_exports` probes
`dlsym` for `print_hex`, `Driver`, `driver_`, `_driver`, `driver2`,
`rust_driver` and asserts each is absent from **both** libraries.

`scripts/verify_all.sh` re-runs the raw `nm -D` diff for every feature
combination. Latest result for all 2 configurations: `ok: symbol sets identical`.
