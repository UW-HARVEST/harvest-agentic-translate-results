# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Public (defined, dynamic) symbols exported by the C `.so`

| # | symbol | type | C declaration (include/driver.h) | exported by Rust `.so`? |
|---|--------|------|----------------------------------|-------------------------|
| 1 | `driver` | `T` (global text) | `void driver(int x, int y);` | YES (`#[unsafe(no_mangle)] pub extern "C" fn driver`) |

That is the complete set: `c_src` contains exactly one translation unit
(`src/driver.c`) with exactly one non-static function (`driver`), declared in the
single public header `include/driver.h`. There are no macro-generated symbols, no
exported globals/data symbols, no `static` helpers promoted by macros, and no
other C source files — so nothing was skipped by the translation.

## Symbol diff

```
comm -23 <(nm -D --defined-only .../c_src/.../libdriver.so | awk '{print $NF}' | sort) \
         <(nm -D --defined-only .../translation/.../libdriver.so | awk '{print $NF}' | sort)
```
=> **empty**. 0 symbols missing from the Rust `.so`.

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC`, `puts@GLIBC`, plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` stubs.

Rust `.so` imports: `printf@GLIBC`, `putchar@GLIBC` (LLVM rewrites the
`puts("")` translation's empty-string call into `putchar('\n')`, an
identical observable write on the *same* glibc `stdout` stream), plus the
Rust runtime's libc/`_Unwind_*` imports (allocator, panic machinery,
backtrace support). All Rust-side undefined symbols are libc / libgcc
unwinder symbols — **0 missing or undefined non-libc symbols**.

## Cargo features

`translation/Cargo.toml` declares no `[features]` table, so the only feature
combination that exists is the (empty) default one:

* `cargo test` == `cargo test --no-default-features` == `cargo test --all-features`.

Phases B and C are therefore run under the single existing configuration, and
that trivially covers "every feature combination".
