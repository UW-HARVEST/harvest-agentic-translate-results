# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

`c_src` contains exactly two files:

| file | functions defined |
|------|-------------------|
| `c_src/include/driver.h` | declares `driver` only |
| `c_src/src/driver.c`     | `printLine`, `bad`, `good`, `driver` |

There is no un-translated C module: 4 of 4 C functions have a Rust
implementation in `translation/src/lib.rs`. Nothing is stubbed or
`unimplemented!()`.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | C signature |
|---|--------|---------|------------|-------------|
| 1 | `printLine` | T | T | `void printLine(const char *line)` |
| 2 | `bad`       | T | T | `void bad(void)` |
| 3 | `good`      | T | T | `void good(void)` |
| 4 | `driver`    | T | T | `void driver(int useGood)` |

**Symbol diff (C − Rust): EMPTY.** Verified with:

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

The Rust `.so` exports no *extra* non-Rust-runtime symbols either
(`nm -D --defined-only` on the Rust cdylib yields exactly the 4 names above).

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C       | none (only `puts@GLIBC`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*`) |
| Rust    | none (only glibc `puts`/`putchar`/`memcpy`/… and `_Unwind_*` from the Rust std/panic runtime) |

**0 missing / 0 undefined non-libc symbols in the Rust build.**

## Automated enforcement

`tests/phase_d_symbols.rs` re-derives all of the above at test time:

| test | asserts |
|------|---------|
| `d01_every_c_symbol_is_exported_by_rust` | C − Rust symbol diff is empty |
| `d02_expected_symbol_set_is_exactly_the_four_public_functions` | the C surface is exactly the 4 names and Rust exports each |
| `d03_all_four_symbols_are_dlsym_resolvable_in_both` | all 4 resolve via `dlsym` in both `.so`s |
| `d04_rust_imports_no_non_libc_symbols` | no unresolved Rust-mangled imports |

Verified for **both** the debug and the `--release` (`panic = "abort"`) cdylib.

## Codegen note

`printf("%s\n", line)` is folded to `puts(line)` by *both* toolchains
(see `objdump -d`: C `printLine` → `puts@plt`; Rust `printLine` → `puts`).
The observable byte stream is therefore identical for every
NUL-terminated input.
