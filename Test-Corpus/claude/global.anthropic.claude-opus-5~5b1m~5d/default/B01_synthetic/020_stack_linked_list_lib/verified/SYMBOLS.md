# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

- C:    `c_src/build/libSimpleList.so`
- Rust: `translation/target/release/libSimpleList.so`

## C exported symbols (`nm -D --defined-only`, non-libc)

| symbol | type | present in Rust `.so`? | notes |
|--------|------|------------------------|-------|
| `smallestValue` | `T` (global text) | YES (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn smallestValue(*mut ListNode) -> c_int` |

## Rust extra symbols

The Rust `cdylib` additionally exports the standard Rust runtime/allocator
housekeeping symbols (`rust_eh_personality`, `__rust_*`, etc. — none appear as
`T` in the filtered listing above for this crate). No C symbol is missing.

## C source files vs. Rust modules (completeness check)

| C source file | translated? | Rust location |
|---------------|-------------|---------------|
| `c_src/src/simplestruct.c` | YES | `translation/src/lib.rs` |
| `c_src/include/simplestruct.h` (type `struct ListNode`) | YES | `ListNode` in `translation/src/lib.rs` (`#[repr(C)]`) |

No C module was skipped. No stubs / `unimplemented!()` present.

## Result

`nm -D` diff of C-exported symbols against the Rust `.so`: **EMPTY** (0 missing,
0 undefined non-libc). Verified by:

```sh
diff <(nm -D --defined-only c_src/build/libSimpleList.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libSimpleList.so | awk '{print $3}' | sort)
```
