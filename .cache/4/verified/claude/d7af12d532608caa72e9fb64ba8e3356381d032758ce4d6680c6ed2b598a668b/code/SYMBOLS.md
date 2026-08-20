# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D` on both shared libraries.

## Build commands

```
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> translated_rust/c_src/build/libtranslated_rust.so

# Rust
cd translated_rust && cargo build --release
# -> translated_rust/target/release/libcall_predict_lib.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libtranslated_rust.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `call_predict` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub extern "C" fn call_predict` |

C `.so` also lists these *weak/undefined* entries, which are toolchain/libc
artifacts and not part of the library surface (they are ignored for parity, as
required — "non-libc symbols"):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only target/release/libcall_predict_lib.so`

| # | symbol | type |
|---|--------|------|
| 1 | `call_predict` | `T` (global text) |

## Diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
          <(nm -D --defined-only target/release/libcall_predict_lib.so | awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: NONE.** The symbol diff is empty in both directions.

## Undefined symbols in the Rust `.so`

All undefined (`U`/`w`) symbols in the Rust `.so` are libc / libgcc-unwind
imports pulled in by the Rust `std` runtime (`malloc`, `memcpy`, `mmap64`,
`_Unwind_*`, `pthread_key_create`, …). There are **0 missing/undefined
non-libc symbols**.

## Notes on non-exported C functions

`c_src/src/lib.c` contains 14 further functions, all of which are `static` in C
and therefore *deliberately* absent from the C `.so`'s dynamic symbol table.
They are translated in `src/lib.rs` as private (non-`no_mangle`) items, which
reproduces the C linkage exactly:

| C function (static) | Rust counterpart | exported? (C / Rust) |
|---------------------|------------------|----------------------|
| `BTAC1C2_PredictSample` | `BTAC1C2_PredictSample` | no / no |
| `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` (12 fns) | same names | no / no |
| `BTAC1C2_GetPredictFunc` | `BTAC1C2_GetPredictFunc` | no / no |

Adding `#[no_mangle]` exports for these would *break* parity with the C `.so`
(the Rust library would export symbols the C library does not), so they stay
private. They are still verified differentially — see `CONFIGS.md` rows 5–35,
which link both TUs' statics into auxiliary shim libraries
(`tests/aux/aux_c.c`, generated `aux_rust.rs`) built **outside** `c_src/`, so
the internal predictor math and the function-pointer dispatch table are
compared behaviourally, not just through `call_predict`.

## `c_src/include/lib.h`

```c
int get_predict_func(int pfcn);
```

The public header declares `get_predict_func`, but **no translation unit in the
project defines it** (`lib.c` defines the `static` `BTAC1C2_GetPredictFunc`
instead). It therefore does not appear in `nm -D` of the C `.so`, and must NOT
be exported by the Rust `.so` either. Confirmed absent from both:

```
$ nm -D c_src/build/libtranslated_rust.so | grep -c get_predict_func   # 0
$ nm -D target/release/libcall_predict_lib.so | grep -c get_predict_func # 0
```
