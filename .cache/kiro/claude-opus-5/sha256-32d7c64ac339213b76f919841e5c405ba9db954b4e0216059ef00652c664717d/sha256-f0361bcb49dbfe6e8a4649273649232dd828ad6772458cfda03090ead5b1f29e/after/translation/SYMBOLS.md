# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Source of truth

C `.so`: `c_src/build/libdriver.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
Rust `.so`: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. `--no-default-features` is
therefore identical to the default build. Verified mechanically in
`scripts/check_features.sh`.

## Symbol table

Every `T` (global text) symbol exported by the C `.so`, and whether the Rust
`.so` exports it with the exact same name.

| # | C symbol | type | in C `.so` | in Rust `.so` | notes |
|---|----------|------|-----------|---------------|-------|
| 1 | `get_os_arch` | T | yes | yes | `#[no_mangle] pub unsafe extern "C" fn get_os_arch` — not declared in `include/lib.h` but non-`static`, so it is part of the ABI surface |
| 2 | `parse_uname_string` | T | yes | yes | the only symbol declared in `include/lib.h` |
| 3 | `w_regexec` | T | yes | yes | non-`static` helper, also part of the ABI surface |

No macro-generated symbols exist in this library (no function-defining macros
in `c_src/src/lib.c` or `c_src/include/lib.h`).

### Data symbols

The C source defines no non-`static` objects. `ARCHS` is a function-local
`const char *[]`, so it is not exported (confirmed: it does not appear in
`nm -D`). The Rust `static ARCHS` is private and likewise not exported.

## Diff result

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
          <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(no output — empty diff)
```

**0 missing symbols. 0 extra symbols.**

## Undefined (imported) symbols

The Rust `.so` must not reference any symbol that cannot be resolved.

C imports (16): `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `fprintf`, `free`, `malloc`, `regcomp`,
`regexec`, `regfree`, `snprintf`, `stderr`, `strchr`, `strdup`, `strlen`,
`strstr`. (`strchr` comes from gcc's inlining of `strstr` with a one-character
needle — the `"|"` lookup.)

Rust imports (57): the identical glibc set — `fprintf`, `free`, `malloc`,
`regcomp`, `regexec`, `regfree`, `snprintf`, `stderr`, `strchr`, `strdup`,
`strlen`, `strstr` — plus the `std`/`libgcc` runtime that any Rust `cdylib`
pulls in (`_Unwind_*`, `memcpy`, `memmove`, `memset`, `pthread_key_*`,
`__tls_get_addr`, `mmap64`, `abort`, …). Every one resolves against
`libgcc_s`/`libc`/`ld-linux`.

Verified with `scripts/symbols.sh` — `ldd -r` reports **no** undefined symbol
for either `.so`, in both the `debug` and `release` profiles.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
