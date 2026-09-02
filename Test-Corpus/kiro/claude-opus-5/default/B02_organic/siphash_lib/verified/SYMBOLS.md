# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-907aNI.so
nm -D --defined-only translation/target/release/libsiphash_lib.so
```

## C `.so` exported (defined) symbols

| # | symbol | type | C declaration |
|---|--------|------|---------------|
| 1 | `siphash`          | `T` | `void siphash(int init);` (`include/lib.h`) |
| 2 | `stbds_hash_bytes` | `T` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed);` (`src/lib.c:110`) |

`stbds_siphash_bytes` is `static` in `src/lib.c:6` → **not** part of the ABI, so it
is correctly kept private (`unsafe fn`, no `#[no_mangle]`) in the Rust crate.

## Rust `.so` exported (defined) symbols, non-mangled

| # | symbol | type | Rust item |
|---|--------|------|-----------|
| 1 | `siphash`          | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn siphash(init: c_int)` |
| 2 | `stbds_hash_bytes` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize` |

## Diff

```
$ comm -23 <(nm -D --defined-only $C | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only $R | awk '{print $3}' | sort -u)
<empty>
```

**0 symbols missing from the Rust `.so`.** No module of the C source was skipped:
`src/lib.c` is the only translation unit in `c_src/CMakeLists.txt`, and all three of
its functions (2 public + 1 `static`) are present in `translation/src/lib.rs`.

## Undefined (imported) symbols

C `.so` imports only libc/CRT: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`,
`__cxa_finalize`, `__gmon_start__`, `_ITM_*` (all weak CRT hooks).
(`puts` is gcc's tail-call optimisation of `printf(" },\n")`; behaviourally identical.)

The Rust `.so` imports `printf` from libc plus the usual `libgcc_s`/`libc`
unwinder + `pthread` symbols. **0 undefined non-libc symbols.**

## Verification checklist

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
- [x] No stubs / `unimplemented!()` / `todo!()` in `src/lib.rs` (verified by grep).
