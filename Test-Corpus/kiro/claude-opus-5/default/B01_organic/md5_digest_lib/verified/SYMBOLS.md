# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Commands used:

```
nm -D --defined-only c_src/build/libharvest-work-srn5eJ.so
nm -D --defined-only translation/target/release/libmd5_digest_lib.so
```

## C `.so` exported (defined, dynamic) symbols

| # | symbol | type | present in Rust `.so`? | action |
|---|--------|------|------------------------|--------|
| 1 | `md5_digest` | `T` (text/global func) | YES (`T md5_digest`) | none — already exported via `#[unsafe(no_mangle)] pub unsafe extern "C"` |

Total C defined dynamic symbols: **1**
Total missing from Rust `.so`: **0**

## Source-level completeness cross-check

The C tree contains exactly two files (`find c_src -name '*.c' -o -name '*.h'`,
excluding `build/`):

* `c_src/include/lib.h` — typedefs `tflac_u8`, `tflac_u32`; `struct tflac_md5`;
  one function declaration.
* `c_src/src/lib.c` — one function definition, `md5_digest`.

There are no additional translation units, no `#define`-generated symbol names,
no macro-expanded function families, and no namespace-renaming macros in the
header, so the source-level name `md5_digest` is also the final linker symbol.
No C module was skipped by the translation; nothing needed to be translated in
this phase.

`tflac_u8`, `tflac_u32`, and `struct tflac_md5` are type-level constructs and
emit no linker symbols in either language; they are ABI contract only (checked
in Phase B via `size_of`/field-offset-sensitive differential calls).

## Undefined-symbol check

The Rust `.so` imports only libc / libgcc-unwind symbols
(`memcpy`, `malloc`, `_Unwind_*`, `pthread_key_*`, …) pulled in by the Rust
runtime. Zero undefined **non-libc** symbols, i.e. no unresolved references to
library code that failed to get translated.

Completion gate item: **`nm -D` shows 0 missing / 0 undefined non-libc symbols
in Rust — PASS.**
