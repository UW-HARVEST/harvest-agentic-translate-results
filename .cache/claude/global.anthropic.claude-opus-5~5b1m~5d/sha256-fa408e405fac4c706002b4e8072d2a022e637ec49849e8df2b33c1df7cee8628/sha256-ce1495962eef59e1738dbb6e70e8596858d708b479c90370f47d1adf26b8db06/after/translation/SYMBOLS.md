# SYMBOLS.md — Phase A: exported-symbol surface

Source of truth: `nm -D --defined-only` on the C shared object built by
`c_src/CMakeLists.txt`.

* C `.so`   : `c_src/build/lib<parent-dir-name>.so` (the CMake project name is derived
  from the *parent directory* of `c_src`, so the file name is environment specific;
  the test harness globs `c_src/build/*.so`).
* Rust `.so`: `translation/target/<profile>/libinreftree_lib.so`
  (`[lib] name = "inreftree_lib"`, `crate-type = ["cdylib"]`).

The C library is a single translation unit (`src/lib.c`). Every non-`static`
definition in it is exported, which is **11 functions + 2 data objects**.

## Symbol table

| # | symbol | kind (C `nm`) | C declaration | in C `.so` | in Rust `.so` | notes |
|---|--------|---------------|---------------|-----------|---------------|-------|
| 1 | `add_op`             | `T` text   | `int add_op(int,int,int,int)`                     | yes | yes | `#[no_mangle] extern "C"` |
| 2 | `multiply_op`        | `T` text   | `int multiply_op(int,int,int,int)`                 | yes | yes | |
| 3 | `subtract_op`        | `T` text   | `int subtract_op(int,int,int,int)`                 | yes | yes | |
| 4 | `divide_op`          | `T` text   | `int divide_op(int,int,int,int)`                   | yes | yes | |
| 5 | `modulo_op`          | `T` text   | `int modulo_op(int,int,int,int)`                   | yes | yes | |
| 6 | `find_node_by_id`    | `T` text   | `TreeNode* find_node_by_id(int)`                  | yes | yes | returns interior pointer into `node_table` |
| 7 | `add_tree_node`      | `T` text   | `int add_tree_node(int,int,int,const char*)`       | yes | yes | |
| 8 | `calculate_tree_sum` | `T` text   | `int calculate_tree_sum(int)`                     | yes | yes | recursive |
| 9 | `parse_operation`    | `T` text   | `Operation parse_operation(const char*)`           | yes | yes | `Operation` is `int`-sized |
| 10 | `get_operation_func`| `T` text   | `OperationFunc get_operation_func(Operation)`      | yes | yes | returns fn pointer |
| 11 | `inreftree`         | `T` text   | `int inreftree(int,int,int,int)` (public header)    | yes | yes | only symbol in `include/lib.h` |
| 12 | `node_table`        | `B` bss, size 2600 | `TreeNode node_table[50]`                  | yes | yes | `#[no_mangle] pub static mut` |
| 13 | `node_count`        | `B` bss, size 4    | `int node_count`                           | yes | yes | `#[no_mangle] pub static mut` |

Nothing is `static` in `lib.c`, so there are no private symbols to account for.
`Operation`, `TreeNode` and `OperationFunc` are types, not symbols.

## ABI details that must match (verified with `readelf -sW`)

| object | C size | Rust size | note |
|--------|--------|-----------|------|
| `node_table` | 2600 bytes | 2600 bytes | `TreeNode` = 5×`int` + `char[32]` = 52 bytes, align 4, ×50 |
| `node_count` | 4 bytes    | 4 bytes    | `c_int` |

`find_node_by_id` hands out a pointer *into* `node_table`; the differential
tests therefore compare the **byte offset from each library's own `node_table`
base**, never the absolute address (the two libraries are mapped independently).

`get_operation_func` hands out a *function pointer*; the differential tests
resolve it against the addresses of that same library's `add_op` … `modulo_op`
via `dlsym` and compare the resulting **index**, never the absolute address.

## Result

`nm -D` diff between the two shared objects: **0 symbols missing from the Rust
`.so`**. No symbol needed a new export wrapper and no C module was missing from
the translation (`src/lib.c` is the whole library). This is asserted
mechanically by the test `phase_d_symbols::c_symbols_are_all_exported_by_rust`,
which shells out to `nm -D` on both objects at test time.

Undefined (imported) symbols in the Rust `.so` are libc/`std` runtime symbols
only — the test also asserts that no *non-libc* symbol is left undefined.
