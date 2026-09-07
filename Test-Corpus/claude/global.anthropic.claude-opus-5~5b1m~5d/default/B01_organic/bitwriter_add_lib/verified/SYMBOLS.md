# SYMBOLS.md — Phase A symbol surface

C library:    `c_src/build/libharvest-work-VRjRwc.so`
Rust library: `translation/target/release/libbitwriter_add_lib.so`

## Defined symbols from `nm -D --defined-only` (excluding weak / libc / linker-generated)

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `bitwriter_add` | `T` | `T` | `int bitwriter_add(tflac_bitwriter*, tflac_u32, tflac_uint)` — exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

The C `.so` exports exactly one non-weak, non-linker-generated global text
symbol. Weak/auto-generated entries filtered out of both libraries:
`_init`, `_fini`, `__bss_start`, `_edata`, `_end`,
`__cxa_finalize@GLIBC_2.2.5` (undefined import), plus the Rust runtime's
`rust_eh_personality` / allocator shims (Rust-internal, not part of the C ABI
surface and therefore not required to match in the C→Rust direction).

## Header-only surface (no symbols, but ABI-relevant)

| item | C | Rust | match |
|------|---|------|-------|
| `tflac_u8` | `uint8_t` | `u8` | yes |
| `tflac_u32` | `uint32_t` | `u32` | yes |
| `tflac_u64` | `uint64_t` | `u64` | yes |
| `tflac_uint` | `tflac_u64` | `u64` | yes |
| `struct tflac_bitwriter` | `{u64 val; u32 bits, pos, len, tot; u8* buffer;}` | `#[repr(C)]` same order | size 32 / align 8 — verified by test `struct_layout_matches_c` |

## Missing-symbol remediation

None required: the symbol diff (C-defined → Rust-defined) is **empty**.
No C source file was left untranslated — `c_src/src/lib.c` (24 lines,
one function) is the entire implementation and it is fully translated in
`translation/src/lib.rs`. No stubs or `unimplemented!()` exist in the crate
(verified by grep).
