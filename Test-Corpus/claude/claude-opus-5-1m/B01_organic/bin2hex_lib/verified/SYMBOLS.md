# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust (crate-type = ["cdylib"], lib name = bin2hex_lib)
cargo build --no-default-features
# -> target/debug/libbin2hex_lib.so
```

## C `.so` dynamic symbol table (raw `nm -D`)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U abort@GLIBC_2.2.5
0000000000001109 T bin2hex
```

`nm -D --defined-only` on the C `.so` yields exactly one symbol:

```
0000000000001109 T bin2hex
```

The other entries are not library API:

* `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__`,
  `__cxa_finalize` — weak (`w`) toolchain/CRT hooks emitted by GCC into every
  shared object; not defined by `lib.c`.
* `abort@GLIBC_2.2.5` — *undefined* (`U`) import from libc, used by the
  validation path in `bin2hex`.

## Parity table

| # | C symbol (`nm -D --defined-only`) | kind | present in Rust `.so`? | Rust definition |
|---|-----------------------------------|------|------------------------|-----------------|
| 1 | `bin2hex`                         | `T` (global text) | YES — `T bin2hex` | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn bin2hex` |

### Missing symbols

**None.** The C library consists of a single translation unit (`c_src/src/lib.c`)
that defines a single external function, and the whole of it is translated in
`src/lib.rs`. There is no untranslated C module, so no symbol needed to be added
and nothing was stubbed.

### Undefined (imported) symbols in the Rust `.so`

All `U` symbols in `target/debug/libbin2hex_lib.so` are libc / Rust-runtime
imports (`abort`, `memcpy`, `write`, `pthread_*`, `__tls_get_addr`, …). There are
**0 missing/undefined non-libc symbols**, i.e. the Rust `.so` needs nothing from
the C `.so` and resolves standalone (verified by `dlopen`ing it in the
integration tests — `libloading::Library::new` performs full relocation).

### Extra symbols exported by the Rust `.so`

The Rust `cdylib` additionally exports Rust-runtime bookkeeping symbols
(`rust_eh_personality`, `__rust_alloc*`, `_ZN*` monomorphisations of `std`, …).
Extra exports are harmless: every symbol the *C* `.so` exports is also exported
by the Rust `.so` with the exact same name, which is the parity requirement.

## Automated check

`tests/symbol_parity.rs` re-derives this table at test time: it runs
`nm -D --defined-only` on both `.so`s, computes
`c_defined_symbols - rust_defined_symbols`, and asserts the difference is empty.
It also asserts `bin2hex` is `dlsym`-able from both objects.
