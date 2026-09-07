# SYMBOLS.md — Public symbol parity

## Source of truth

C shared library: `c_src/build/libdriver.so` (built from the single TU
`c_src/src/lib.c`; public header `c_src/include/lib.h` declares exactly one
function).

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001163 T encode_base64
```

Rust shared library: `translation/target/release/libdriver.so`
(`crate-type = ["cdylib"]`).

```
$ nm -D --defined-only translation/target/release/libdriver.so
00000000000116d0 T encode_base64
```

## Symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `encode_base64` | `T` (global text) | `T` (global text) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

## Non-exported (internal) C symbols

| C symbol | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `encode` | `static char encode(unsigned char)` — TU-local, not in `nm -D` | `fn encode(u: u8) -> c_char` (private, `#[inline]`) | no — correct, must NOT be exported |

## Undefined / imported symbols

The two libc functions the translation genuinely needs are the same two the C
uses:

| imported by | C `.so` | Rust `.so` |
|-------------|---------|------------|
| `calloc@GLIBC_2.2.5` | yes | yes |
| `strlen@GLIBC_2.2.5` | yes | yes |

Every remaining undefined symbol in the Rust `.so` is glibc (`@GLIBC_*`), the
GCC unwinder (`_Unwind_*@GCC_*`), or a weak symbol (`w`) — i.e. the standard
Rust `std`/panic-machinery and allocator glue, not untranslated code:

```
$ nm -D --undefined-only translation/target/release/libdriver.so \
    | awk '{print $NF}' \
    | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^gettid\|^statx'
(empty)
```

**Non-libc undefined symbols: 0.** (Checked automatically by `verify.sh`.)

## Diff

**Symbols in C `.so` missing from Rust `.so`: 0.**
**Non-libc undefined symbols in Rust `.so`: 0.**

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table and no optional
dependencies, so there is exactly one build configuration (the default). There
is no `[[bin]]` target and no `src/main.rs`, so the project builds **no binary
driver** — the "compare C and Rust stdout" gate is not applicable.
