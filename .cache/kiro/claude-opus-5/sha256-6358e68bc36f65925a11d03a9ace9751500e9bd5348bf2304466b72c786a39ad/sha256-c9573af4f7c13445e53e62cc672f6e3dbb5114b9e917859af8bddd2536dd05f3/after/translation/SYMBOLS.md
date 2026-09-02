# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Mechanically derived from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C source inventory

The whole library is two files:

| C file | functions defined | translated in |
|--------|-------------------|---------------|
| `c_src/src/pow.c` | `my_pow` | `translation/src/pow.rs` |
| `c_src/include/pow.h` | (declaration only, no code) | — |

No C source file is untranslated; there is no skipped module.

## Exported (defined) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `my_pow` | `T` | `T` | `#[unsafe(no_mangle)] pub extern "C" fn my_pow` in `src/pow.rs` |

Symbol diff (C-exported minus Rust-exported): **EMPTY**.

There are no macro-generated symbols in the C source (no function-generating
macros are used), so the exported set is exactly the one public function
declared in `include/pow.h`.

## Undefined (imported) symbols

The C `.so` imports, besides the weak CRT/ITM markers:

| symbol | Rust `.so` also imports it |
|--------|----------------------------|
| `__errno_location@GLIBC_2.2.5` | yes |
| `fprintf@GLIBC_2.2.5` | yes |
| `pow@GLIBC_2.29` | yes |
| `stderr@GLIBC_2.2.5` | yes |

All four are imported by the Rust `.so` too — i.e. the translation really calls
libm's `pow` (not `llvm.pow.f64`), really reads glibc's thread-local `errno`,
and really writes glibc's `stderr` `FILE`, which is what makes the `errno`
branches and the `%.2f` byte formatting observable in the same way.

The Rust `.so` imports additional symbols (`_Unwind_*`, `malloc`, `memcpy`,
`dl_iterate_phdr`, …). These come from the Rust runtime / panic machinery that
`cdylib` always links; all are libc / libgcc_s symbols, none are undefined
non-libc symbols. Verified with the check below.

## Completion check

```
# 0 lines of output == parity
comm -23 <(nm -D --defined-only c_src/build/libpow.so   | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libpow.so | awk '{print $NF}' | sort -u)
```

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined *non-libc* symbols in the Rust `.so`.
