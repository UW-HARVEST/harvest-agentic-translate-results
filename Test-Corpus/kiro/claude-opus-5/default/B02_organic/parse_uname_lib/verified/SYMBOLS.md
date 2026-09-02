# SYMBOLS.md — public symbol surface

Source of truth: `nm -D --defined-only` on both shared objects.

```
$ nm -D --defined-only c_src/build/libdriver.so
00000000000011c9 T get_os_arch
0000000000001378 T parse_uname_string
00000000000012ca T w_regexec

$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011bf0 T get_os_arch
0000000000011d50 T parse_uname_string
0000000000011f80 T w_regexec
```

## Parity table

| # | C symbol | type | exported by Rust `.so` | notes |
|---|----------|------|------------------------|-------|
| 1 | `get_os_arch` | `T` (text, global) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn get_os_arch` | not declared in `include/lib.h`, but has external linkage in `src/lib.c`, so it *is* part of the ABI surface |
| 2 | `w_regexec` | `T` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn w_regexec` | not in the header either; still exported by the C `.so` |
| 3 | `parse_uname_string` | `T` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_uname_string` | the only symbol declared in `include/lib.h` |

**Missing from Rust `.so`: 0.**
**Extra non-libc defined symbols in Rust `.so`: 0** (verified with a set diff of
`nm -D --defined-only` output on both files; see `scripts/symbol_diff.sh`).

## Undefined (imported) symbols

The C `.so` imports only libc/glibc symbols:

```
fprintf free malloc regcomp regexec regfree snprintf stderr
strchr strdup strlen strstr
(+ weak _ITM_*/​__gmon_start__/__cxa_finalize)
```

`strchr` appears in the C `.so` because glibc's `strstr` is inlined by GCC into
a `strchr` call when the needle is one byte long (`"|"`). This is an
implementation detail of the compiler, not part of the API, so it is not
required of the Rust `.so`.

The Rust `.so` imports the same regex/alloc/stdio family plus ten
`_Unwind_*@GCC_*` entry points from `libgcc_s`. Those belong to the Rust
language runtime, not to the translated library, and they all resolve:

```
$ ldd -r translation/target/release/libdriver.so | grep -i 'undefined\|not found'
(no output)
```

So there are **0 unresolved imports**. The only import the C `.so` has that the
Rust `.so` lacks is `strchr`, which appears in the C build purely because GCC
rewrites the one-byte `strstr(s, "|")` into `strchr`; it is not part of the API.

Verified by `scripts/symbol_diff.sh`, which fails unless both the missing-export
set and the unresolved-import set are empty.

## Modules translated

`c_src` contains exactly one translation unit (`src/lib.c`, 147 lines) and one
header (`include/lib.h`). There is no untranslated module: every function
definition in `src/lib.c` (`get_os_arch`, `w_regexec`, `parse_uname_string`)
has a corresponding `extern "C"` definition in `translation/src/lib.rs`, and
the `os_data` struct from the header is mirrored as `#[repr(C)] pub struct
os_data` with the same 9 `*mut c_char` fields in the same order.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, and no
`#[cfg(feature = ...)]` / `#[cfg(...)]` attributes exist anywhere in
`src/lib.rs`. `c_src/src/lib.c` contains no `#ifdef` other than the implicit
include guards. Therefore the *only* build configuration is the default one,
and "every feature combination" is satisfied by the default feature set. This
is verified mechanically by `scripts/feature_matrix.sh`.
