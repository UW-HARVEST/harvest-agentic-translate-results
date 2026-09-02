# SYMBOLS.md — dynamic-symbol parity between the C and Rust shared objects

Generated mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D -u   <each>
```

## Public (defined, dynamic) symbols exported by the C `libdriver.so`

| # | symbol | C type/binding | source | exported by Rust `.so`? |
|---|--------|----------------|--------|-------------------------|
| 1 | `encode_base64` | `T` (global text) | `c_src/src/lib.c:29`, declared in `c_src/include/lib.h:1` | YES — `translation/src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn encode_base64` |

That is the complete list. The C `.so` exports exactly **one** defined dynamic
symbol; there are no macro-generated symbols, no exported data objects, and no
weak aliases.

`encode()` (`c_src/src/lib.c:6`) is `static`, therefore **not** part of the ABI.
It appears in neither `.so`'s dynamic symbol table, so the Rust translation
correctly keeps it a private `fn encode`.

## Missing-symbol diff

```
C defined dyn syms      : encode_base64
Rust defined dyn syms   : encode_base64
--------------------------------------------
missing from Rust .so   : (none)
extra in Rust .so       : (none)
```

**0 missing symbols.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated: `c_src` contains a single translation unit
(`src/lib.c`, 83 lines) and a single-line public header, and both are fully
translated in `translation/src/lib.rs`.

## Undefined (imported) symbols

The C `.so` imports only `calloc@GLIBC_2.2.5` and `strlen@GLIBC_2.2.5`, plus
the usual weak CRT hooks (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports those same two libc symbols and additionally pulls in
the Rust runtime's libc/unwinder dependencies (`_Unwind_*`, `malloc`, `free`,
`memcpy`, `mmap64`, `dl_iterate_phdr`, `pthread_key_*`, …). All of these are
libc / libgcc symbols satisfied by the platform, so:

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default (empty) feature set. `--no-default-features`
and the default build are therefore the same code, and symbol parity above
holds for every configuration that exists (verified by `check_features.sh`).

## Verification transcript

```
$ ./check_features.sh
Cargo.toml declares no [features]; the only configuration is the default.

=== default ===
  symbol parity: OK (1 C symbol(s), 0 missing)
  undefined in Rust .so (all must be libc/libgcc): 42 symbols
  tests: 4 suites ok, 49 tests passed

=== --no-default-features ===
  symbol parity: OK (1 C symbol(s), 0 missing)
  undefined in Rust .so (all must be libc/libgcc): 42 symbols
  tests: 4 suites ok, 49 tests passed

===============================
ALL COMBINATIONS PASS
```

The 42 undefined symbols in the Rust `.so` are the Rust runtime's libc /
libgcc imports (`_Unwind_*`, `malloc`, `free`, `memcpy`, `mmap64`,
`dl_iterate_phdr`, `pthread_key_*`, `calloc`, `strlen`, …) — all resolved by
the platform, none belonging to this library.
