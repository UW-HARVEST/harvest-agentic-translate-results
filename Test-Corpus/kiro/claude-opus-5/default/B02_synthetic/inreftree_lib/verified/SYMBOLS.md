# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-O7jLg0.so` (built with
  `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
- Rust `.so`: `translation/target/release/libinreftree_lib.so`
  (`cd translation && cargo build --release`)

The whole C library is a single translation unit (`c_src/src/lib.c`, 197 lines);
`CMakeLists.txt` compiles only that file, so there is no un-translated module.
Every non-`static` definition in `lib.c` therefore appears in the dynamic symbol
table, not just the one function declared in `include/lib.h` (`inreftree`).

## Symbol parity table

| # | symbol | type | C `.so` | Rust `.so` | notes |
|---|--------|------|---------|------------|-------|
| 1 | `add_op` | `T` func | yes | yes | `int(int,int,int,int)` |
| 2 | `multiply_op` | `T` func | yes | yes | `int(int,int,int,int)` |
| 3 | `subtract_op` | `T` func | yes | yes | `int(int,int,int,int)` |
| 4 | `divide_op` | `T` func | yes | yes | `int(int,int,int,int)` |
| 5 | `modulo_op` | `T` func | yes | yes | `int(int,int,int,int)` |
| 6 | `find_node_by_id` | `T` func | yes | yes | `TreeNode*(int)` |
| 7 | `add_tree_node` | `T` func | yes | yes | `int(int,int,int,const char*)` |
| 8 | `calculate_tree_sum` | `T` func | yes | yes | `int(int)`, recursive |
| 9 | `parse_operation` | `T` func | yes | yes | `Operation(const char*)`, ABI `int` |
| 10 | `get_operation_func` | `T` func | yes | yes | `OperationFunc(Operation)`, returns fn ptr |
| 11 | `inreftree` | `T` func | yes | yes | `int(int,int,int,int)`; only header-declared entry point |
| 12 | `node_table` | `B` data | yes | yes | `TreeNode[50]`, 2600 bytes (`0x4060..0x4a88`) |
| 13 | `node_count` | `B` data | yes | yes | `int` |

**Missing from Rust `.so`: none (0 of 13).**
**Extra non-libc symbols in Rust `.so`: none.**

## Undefined-symbol check

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports
(`malloc`, `memcpy`, `strlen`, `_Unwind_*`, `__cxa_finalize`, …). There are
**0 undefined non-libc symbols**, so nothing is referenced but un-translated.

## `TreeNode` layout agreement

The C struct is 5 × `int` + `char[32]` = 52 bytes, align 4. The C `.so` places
`node_table` at `0x4060` and `node_count` at `0x4a88`; the 2600-byte gap
confirms `50 * 52`. The Rust `#[repr(C)] TreeNode` reproduces this, so tests may
read/write `node_table` through the exported data symbol in either library with
the same offsets.

## `.rodata` string-literal layout (needed for a UB corner)

`inreftree` evaluates `op_string[tree_sum % 4]`. C's `%` truncates toward zero,
so a negative `tree_sum` yields a **negative index** that reads the bytes
preceding the `"+*-%"` literal. `objdump -s -j .rodata` on the C `.so` shows the
literal block, in emission order:

```
2018  root\0left\0right\0left-left\0+*-%\0
```

`"+*-%"` sits at offset **26** within that block. The Rust translation embeds the
same 31-byte block (`RODATA_LITERALS`) with `OP_STRING_OFFSET = 26`, so indices
`-3..=3` (byte offsets 23..=29) read identical bytes in both libraries:

| `tree_sum % 4` | byte offset | C byte | Rust byte |
|---|---|---|---|
| -3 | 23 | `'f'` | `'f'` |
| -2 | 24 | `'t'` | `'t'` |
| -1 | 25 | `'\0'` | `'\0'` |
| 0 | 26 | `'+'` | `'+'` |
| 1 | 27 | `'*'` | `'*'` |
| 2 | 28 | `'-'` | `'-'` |
| 3 | 29 | `'%'` | `'%'` |

Verified byte-for-byte by Phase B test `configs_row_16_negative_tree_sum_modulus`.
