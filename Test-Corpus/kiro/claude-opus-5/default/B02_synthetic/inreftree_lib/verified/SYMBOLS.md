# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-2K08Z7.so`
- Rust `.so`: `translation/target/release/libinreftree_lib.so`

The C build (`c_src/CMakeLists.txt`) compiles exactly one translation unit,
`src/lib.c`, into a shared library. No symbol visibility attributes are used, so
every non-`static` function and file-scope object becomes a dynamic symbol.
There are no `static` definitions in `lib.c`, therefore all 13 definitions are
exported. There are no `#ifdef` / feature toggles in either tree, and
`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration to verify.

## Symbol table (13 symbols: 11 functions `T`, 2 data objects `B`)

| # | symbol | kind | C decl | in C `.so` | in Rust `.so` | notes |
|---|--------|------|--------|-----------|--------------|-------|
| 1 | `add_op` | `T` func | `int add_op(int,int,int,int)` | yes | yes | — |
| 2 | `multiply_op` | `T` func | `int multiply_op(int,int,int,int)` | yes | yes | — |
| 3 | `subtract_op` | `T` func | `int subtract_op(int,int,int,int)` | yes | yes | — |
| 4 | `divide_op` | `T` func | `int divide_op(int,int,int,int)` | yes | yes | guards `b==0`; `INT_MIN/-1` faults |
| 5 | `modulo_op` | `T` func | `int modulo_op(int,int,int,int)` | yes | yes | guards `b==0`; `INT_MIN%-1` faults |
| 6 | `find_node_by_id` | `T` func | `TreeNode* find_node_by_id(int)` | yes | yes | returns interior pointer into `node_table` |
| 7 | `add_tree_node` | `T` func | `int add_tree_node(int,int,int,const char*)` | yes | yes | — |
| 8 | `calculate_tree_sum` | `T` func | `int calculate_tree_sum(int)` | yes | yes | recursive |
| 9 | `parse_operation` | `T` func | `Operation parse_operation(const char*)` | yes | yes | `Operation` is `int`-sized at the ABI |
| 10 | `get_operation_func` | `T` func | `OperationFunc get_operation_func(Operation)` | yes | yes | returns a function pointer |
| 11 | `inreftree` | `T` func | `int inreftree(int,int,int,int)` | yes | yes | the only symbol in `include/lib.h` |
| 12 | `node_table` | `B` data | `TreeNode node_table[50]` | yes | yes | 50 * 52 = 2600 bytes |
| 13 | `node_count` | `B` data | `int node_count` | yes | yes | 4 bytes |

## Symbol diff result

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-2K08Z7.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libinreftree_lib.so | awk '{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.** No `#[no_mangle]` wrapper had to be
added and no C source module was left untranslated — `lib.c` is the only C
translation unit and every one of its definitions has a real Rust
implementation (no stubs, no `unimplemented!()`).

The check is automated by `translation/check_symbols.sh` and asserted from the
test suite in `tests/phase_d_symbols.rs`.

Verified for the release `.so` (the build the task specifies) and for the debug
`.so`; both export the same 13 names.

## Undefined (imported) symbols

The Rust `.so` imports only libc/`std` runtime symbols
(`memcpy`, `__libc_start_main`-family, unwinder/`pthread` stubs, etc.). It
imports **no** symbol that the C library would have had to provide, i.e. there
are 0 missing/undefined non-libc symbols.

## ABI notes that the tests rely on

- `Operation` is a C enum with values 1..5, all representable in `int`, so it is
  passed and returned as `int`. `get_operation_func` therefore accepts *any*
  `int`, including values with no valid variant (see `ERRORS.md` rows 12–13).
- `TreeNode` is `5 * int + char[32]` = **52 bytes**, alignment 4. Confirmed from
  the C `.so`: `node_table` at `0x4060`, `node_count` at `0x4a88`;
  `0x4a88 - 0x4060 = 0xa28 = 2600 = 50 * 52`.
- Data-object *ordering* differs between the two libraries (C: `node_table` then
  `node_count`; Rust: `node_count` then `node_table`). This is only observable
  by reading `node_table[50]`, which is out of bounds in both languages; see
  `CONFIGS.md` row 34 for how the tests stay inside the defined range.
