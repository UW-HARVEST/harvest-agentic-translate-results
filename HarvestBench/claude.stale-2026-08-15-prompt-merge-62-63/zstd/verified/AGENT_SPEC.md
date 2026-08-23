# Translation conventions for sub-agents

You are translating ONE C source file from the zstd library (in `c_src/src/...`)
into ONE Rust file in this crate. The crate is a `cdylib` reproducing the C
library's public ABI **byte-for-byte**.

## Hard rules
- Target platform: little-endian 64-bit Linux. You may assume `size_t`/`usize` = 8 bytes.
- Every C function that is a GLOBAL symbol (appears in the provided symbol list)
  MUST be exported as:
  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C" fn NAME(args...) -> ret { ... }
  ```
  The Rust name MUST exactly equal the linker symbol name in the list.
- Preserve EXACT C function signatures using core::ffi types
  (`c_void`, `c_int`=i32, `c_uint`=u32, `size_t`=usize, `c_char`=i8, etc.).
  Use `*const`/`*mut` for pointers.
- Do NOT fix bugs. Reproduce behavior, error-check order, and integer overflow
  (use `wrapping_*` where C relies on wraparound) exactly.
- `static`/file-local C functions -> private (non-`no_mangle`) Rust fns.
- Reproduce error codes as `(-(code as isize)) as usize`. Error enum values are
  in `crate::common::error::code`. `ERR_isError(x)` == `x > error(MAXCODE)`.
- Memory: use these from `crate::common::allocations`:
  `malloc, calloc, free, memcpy, memmove, memset, qsort` (libc), and
  `ZSTD_customMem`, `zstd_custom_malloc/calloc/free`.
- Unaligned LE reads/writes: use helpers in `crate::common::mem`
  (`mem_read_le16/32/64`, `mem_read16/32/64`, `mem_write*`, etc.). Or just use
  `core::ptr::copy_nonoverlapping`. On LE64 native reads are fine.
- For structs shared across the ABI boundary, use `#[repr(C)]`.
- Put `#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused_mut, unused_assignments, unused_parens)]` concerns at crate root (already set) — you can still add `#[allow(...)]` locally if needed.

## Legacy files (zstd_vNN.c)
These are self-contained: they define their OWN internal FSE/HUF/ZBUFF code.
Translate the WHOLE file into `src/legacy/zstd_vNN.rs`. Internal helper
functions/types stay private to the module. Only the symbols in the provided
list get `#[unsafe(no_mangle)]`. Note some exported names are prefixed
`FSEv05_`, `HUFv05_`, `ZBUFFv06_`, etc. — export those exactly as listed.

## Verification
Your file must compile as part of `cargo build --release`. After writing it,
add `pub mod zstd_vNN;` is handled by the coordinator — you only write the .rs.
Do NOT edit files outside your assigned .rs unless told.
Do NOT modify anything under c_src/.
