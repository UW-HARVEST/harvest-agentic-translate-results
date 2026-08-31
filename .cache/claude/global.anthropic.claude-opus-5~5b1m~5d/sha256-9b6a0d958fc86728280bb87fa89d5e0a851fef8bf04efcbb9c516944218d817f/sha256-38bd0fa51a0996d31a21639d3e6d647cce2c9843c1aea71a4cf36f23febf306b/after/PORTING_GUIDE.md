# zstd C -> Rust transliteration guide (MANDATORY reading)

Goal: a **mechanical, semantics-preserving transliteration** of the C in `c_src/`
into Rust in `translation/src/`. The Rust cdylib must export the same public
symbols and produce byte-identical results. Do NOT redesign, do NOT "improve",
do NOT fix bugs. Keep function order, check order, and arithmetic exactly.

## Build configuration (already fixed, do not change)
* `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`
* `ZSTD_MULTITHREAD` is **NOT** defined  -> threading/pool are no-ops
* `DEBUGLEVEL` is 0 -> `assert()`, `DEBUGLOG()`, `RAWLOG()` are **no-ops**:
  simply drop them. Never translate an `assert()` into a Rust `assert!`.
* `ZSTD_TRACE == 1` (weak symbols available) but the hooks are never defined,
  so they are always NULL. `crate::zstd_trace` provides
  `ZSTD_trace_compress_begin` etc. as `None`. Keep the NULL checks.
* No ASAN / MSAN -> drop all `__asan_*` / `__msan_*` code.
* Target is x86-64 little-endian, 64-bit.

## Crate layout
```
translation/src/
  lib.rs                     module list
  cmem.rs                    mem.h  (MEM_read*/MEM_write*/MEM_swap*, ZSTD_memcpy/memmove/memset/memcmp,
                             malloc/calloc/free externs, type aliases BYTE,U8,S8,U16,S16,U32,S32,U64,S64)
  bits.rs                    bits.h (ZSTD_highbit32, ZSTD_countTrailingZeros*, ZSTD_NbCommonBytes, ZSTD_rotateRight_*)
  error_private.rs           zstd_errors.h + error_private.{h,c}
                             `ERROR(ZSTD_error_xxx)` -> usize;  ERR_isError, ERR_getErrorCode, ERR_getErrorName
  bitstream.rs               bitstream.h (BIT_CStream_t, BIT_DStream_t, BIT_* fns)
  fse.rs                     fse.h  (constants, FSE_CState_t, FSE_DState_t, inline fns)
  huf.rs                     huf.h  (constants, HUF_CElt, HUF_DTable, HUF_CTableHeader)
  xxhash.rs                  xxhash (XXH32/XXH64; exported as ZSTD_XXH32... )
  zstd_h.rs                  public zstd.h constants/enums/structs
  zstd_internal.rs           zstd_internal.h + allocations.h  (tables, ZSTD_wildcopy, ZSTD_customMalloc, ...)
  zstd_trace.rs              zstd_trace.h
  zstd_common.rs             zstd_common.c
  pool.rs                    pool.c (non-MT) + threading.c + debug.c globals
  entropy_common.rs          common/entropy_common.c
  fse_decompress.rs          common/fse_decompress.c
  compress/mod.rs  + one file per c_src/src/compress/*.c and the shared headers
  decompress/mod.rs + one file per c_src/src/decompress/*.c
  dictbuilder/mod.rs + one file per c_src/src/dictBuilder/*.c
  deprecated/mod.rs + one file per c_src/src/deprecated/*.c
  legacy/mod.rs    + v01.rs .. v07.rs
```
Shared *type* definitions already exist; **reuse them, never redefine**:
* `compress/zstd_cwksp.rs`            <- `zstd_cwksp.h`
* `compress/zstd_compress_internal.rs` <- `zstd_compress_internal.h`
  (SeqDef, SeqStore_t, ZSTD_MatchState_t, ZSTD_CCtx, ZSTD_CCtx_params, ZSTD_CDict,
   optState_t, ldmState_t, ldmParams_t, RawSeqStore_t, rawSeq, ZSTD_match_t,
   ZSTD_optimal_t, hashes, ZSTD_count, ZSTD_storeSeq, ZSTD_window_*, ...)
* `decompress/zstd_decompress_internal.rs` <- `zstd_decompress_internal.h`
  (ZSTD_DCtx, ZSTD_DDict, ZSTD_seqSymbol, ZSTD_entropyDTables_t, LL_base, OF_base, ML_base, ...)

READ those files before writing code so you use the exact names/types.

## Style rules

1. **Every file starts with**
   ```rust
   //! Translation of `<relative c path>`
   #![allow(dead_code)]
   ```
   and imports what it needs from `crate::...`.

2. **Public C functions** (those visible in `nm -D` of the C .so) get
   ```rust
   #[unsafe(no_mangle)]
   pub unsafe extern "C" fn NAME(args...) -> Ret { ... }
   ```
   Use the exact final linker name (watch for `#define foo PREFIX_foo` macros,
   e.g. xxhash's `XXH_NAMESPACE`, and the legacy `FSEv05_`/`HUFv06_` renames).
   `static` C functions become plain `pub(crate) unsafe fn` (no `no_mangle`,
   no `extern "C"`) unless they are used as function pointers, in which case
   they must be `unsafe extern "C" fn` (but still no `#[unsafe(no_mangle)]`).

3. **Types**: map C to Rust as
   `size_t`->`usize`, `ptrdiff_t`->`isize`, `int`->`c_int`, `unsigned`->`c_uint`,
   `char`->`c_char`, `void*`->`*mut c_void`, `const void*`->`*const c_void`,
   `unsigned long long`->`u64`, `BYTE`->`u8`.
   Enums are `pub type X = c_uint;` + `pub const` values (already done for shared ones).
   Structs are `#[repr(C)]` with the **same field order** as C.

4. **Arithmetic**: C has wrapping semantics for unsigned and (in practice) for
   signed. The crate is built with `overflow-checks = false`, but be explicit
   where overflow is expected: use `wrapping_add/sub/mul`, and for pointers use
   `(p as usize).wrapping_add(n) as *const T` style when the C code can create
   out-of-range pointers. Prefer `.add()/.sub()/.offset()` where valid.
   `a - b` on pointers -> `a.offset_from(b)` (isize) or
   `(a as usize) - (b as usize)`.

5. **Shifts**: C `x << n` where n may equal the width is UB; reproduce the x86
   behaviour only if the C actually relies on it (rare). Otherwise plain `<<`.

6. **`memcpy`/`memmove`/`memset`** -> `ZSTD_memcpy/ZSTD_memmove/ZSTD_memset`
   from `crate::cmem` (they take `*mut c_void`).

7. **Control flow**: `do { X } while (cond);` becomes
   `loop { X; if !(cond) { break; } }`. `goto` becomes a labelled loop / early
   return; preserve semantics exactly. C `switch` fallthrough must be replicated
   by duplicating the code or restructuring carefully.

8. **Macros** `RETURN_ERROR_IF(cond, err, ...)`,
   `RETURN_ERROR(err, ...)`, `FORWARD_IF_ERROR(e, ...)`, `CHECK_F(f)`:
   ```rust
   if cond { return ERROR(ZSTD_error_err); }
   return ERROR(ZSTD_error_err);
   { let e = expr; if ERR_isError(e) != 0 { return e; } }
   ```
   `MIN`/`MAX` -> explicit `if`/`.min()`/`.max()` (careful with mixed signs).

9. **Static const tables** -> `static NAME: [T; N] = [...];`
   Function-local `static const` tables -> module-level `static`.

10. **Function pointers** -> `Option<unsafe extern "C" fn(...) -> R>`; a NULL
    check becomes `.is_none()`.

11. Do NOT add bounds checks, `Option`, `Result`, slices, or iterators where the
    C used raw pointers. Raw pointers everywhere is expected and correct here.

12. Never use `std::` collections or allocation; use the libc `malloc`/`free`
    externs from `crate::cmem` (via `ZSTD_customMalloc`/`ZSTD_customFree`).

13. If a C construct genuinely cannot be expressed, leave a
    `/* TODO(port): ... */` comment — but this should essentially never happen.

## Verification
`cd translation && cargo build --release` must succeed with no errors.
Then `nm -D --defined-only target/release/libzstd.so` must contain every symbol
that the C build exports.
