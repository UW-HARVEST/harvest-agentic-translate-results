# SYMBOLS.md — public symbol parity

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-pYQ2Dj.so

cd translation && cargo build --release
# -> translation/target/release/libupdate_md5_lib.so
```

## Defined (exported) symbols

`nm -D --defined-only` on each `.so`:

| # | symbol | in C `.so` | in Rust `.so` | note |
|---|--------|-----------|---------------|------|
| 1 | `tflac_pack_u64le`   | T | T | `src/lib.c:5`  — non-static, so externally visible even though absent from `include/lib.h` |
| 2 | `tflac_md5_addsample`| T | T | `src/lib.c:17` — non-static, likewise externally visible |
| 3 | `update_md5`         | T | T | `include/lib.h:21` — the only header-declared entry point |

**Missing from Rust `.so`: 0.**
**Extra in Rust `.so` (beyond the 3 above): 0.**

There are no macro-generated symbols in this library: `c_src/src/lib.c` and
`c_src/include/lib.h` contain no function-defining macros (`grep -n '#define'`
returns nothing), so the exported set is exactly the three non-static function
definitions.

No symbol required translation of un-translated C source, and no symbol needed a
new `#[no_mangle]` wrapper — all three C functions are implemented in
`translation/src/lib.rs` with `#[unsafe(no_mangle)] pub unsafe extern "C"`.

## Undefined (imported) symbols

`nm -D --undefined-only`:

* C `.so`: only the weak toolchain symbols `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC`, `__gmon_start__`.
* Rust `.so`: the same weak toolchain symbols plus glibc
  (`malloc`, `memcpy`, `memset`, `read`, `write`, `open64`, `pthread_key_*`, …)
  and libgcc unwinder (`_Unwind_*`) imports pulled in by the Rust standard
  library's panic/backtrace machinery.

**0 missing / undefined non-libc symbols in the Rust `.so`.** Every `U`/`w`
entry above resolves against `libc.so.6`, `libgcc_s.so.1` or is a weak
no-op, exactly as for the C `.so`.

## ABI layout parity (checked, since all three functions take raw struct pointers)

Verified with a C probe compiled against `c_src/include/lib.h`:

| type | C size | C align | field offsets (C) | Rust `#[repr(C)]` equivalent |
|------|--------|---------|-------------------|------------------------------|
| `tflac_md5` | 88 | 8 | `pos`=0, `total`=8, `buffer`=16 | identical |
| `tflac`     | 96 | 8 | `md5_ctx`=0, `cur_blocksize`=88, `channels`=92 | identical |

`translation/src/lib.rs` hard-codes `MD5_BUFFER_OFFSET = 16`, which matches the
probed `offsetof(tflac_md5, buffer)`. This constant matters because the C's
carry-down loop reads *past* the end of `buffer`, and the Rust reproduces that
read with pointer arithmetic rooted at the struct base.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so `default` is
the only build configuration. The feature-combination sweep in Phase D
therefore degenerates to a single combo, but is still executed explicitly
(`--no-default-features`, and default) by `run_all.sh`.
