# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

C:    `c_src/build/liblong.so`
Rust: `translation/target/release/liblong.so`

## Defined (exported) symbols in the C `.so`

| symbol | type | C size | present in Rust `.so`? | Rust type | Rust size |
|--------|------|--------|------------------------|-----------|-----------|
| `array` | `B` (bss object) | `0x100000` | YES | `B` | `0x100000` |
| `long_exec` | `T` (text/func) | — | YES | `T` | — |
| `perform_expensive_operations` | `T` (text/func) | — | YES | `T` | — |

## Undefined / imported symbols in the C `.so`

| symbol | kind | in Rust `.so`? |
|--------|------|----------------|
| `printf@GLIBC_2.2.5` | `U` libc | YES (imported) |
| `rand@GLIBC_2.2.5` | `U` libc | YES (imported) |
| `srand@GLIBC_2.2.5` | `U` libc | YES (imported) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` libc | YES |
| `_ITM_deregisterTMCloneTable` | `w` toolchain | (toolchain-generated, not API) |
| `_ITM_registerTMCloneTable` | `w` toolchain | (toolchain-generated, not API) |
| `__gmon_start__` | `w` toolchain | (toolchain-generated, not API) |

## Header surface (`c_src/include/long.h`)

Only `void long_exec(unsigned int seed);` is declared in the public header.
`perform_expensive_operations` and `array` have external linkage in
`c_src/src/long.c` (no `static`), so they are part of the exported ABI and are
reproduced in Rust with `#[no_mangle]`.

## Result

- Symbol diff (C defined symbols not defined by Rust): **EMPTY**
- Undefined non-libc symbols in Rust `.so`: **NONE**
- Every C module (`src/long.c`, the only translation unit) is translated.

Verification command used:

```sh
diff <(nm -D c_src/build/liblong.so            | awk '$2 ~ /^[A-TBD]$/ {print $3}' | sort) \
     <(nm -D translation/target/release/liblong.so | awk '$2 ~ /^[A-TBD]$/ {print $3}' | sort)
```

## Phase D result

```
$ diff <(nm -D c_src/build/liblong.so | ...) <(nm -D translation/target/release/liblong.so | ...)
OK: defined-symbol diff is EMPTY
OK: every libc import of the C .so is also imported by Rust
```

* 0 missing symbols; 0 undefined non-libc symbols in the Rust `.so`.
* Nothing is stubbed: all three symbols are real translations of `src/long.c`.
* The Rust `.so` additionally imports `_Unwind_*` / allocator symbols from its
  own `std`; those are runtime-support imports, not part of the library's API,
  and every symbol the C `.so` needs is present.
* Feature combinations: `translation/Cargo.toml` declares no `[features]`, and
  `src/lib.rs` contains no `cfg(feature = ...)`, so there is exactly one
  configuration. `run_all.sh` still runs the suite under both the default and
  `--no-default-features`; both are green.
