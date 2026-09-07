# SYMBOLS.md — Phase A symbol surface

C `.so`: `c_src/build/libharvest-work-OzhDkb.so`
Rust `.so`: `translation/target/release/libinreftree_lib.so`

Produced with `nm -D --defined-only <so> | sort`.

The C build (`CMakeLists.txt`) compiles the single TU `src/lib.c` into one shared
object with default visibility, so **every** non-`static` definition in `lib.c`
is part of the exported ABI — including the two global data objects.

## Symbol table

| # | symbol | kind | C | Rust | notes |
|---|--------|------|---|------|-------|
| 1 | `add_op`            | T (func) | ✅ | ✅ | `int(int,int,int,int)` |
| 2 | `multiply_op`       | T (func) | ✅ | ✅ | `int(int,int,int,int)` |
| 3 | `subtract_op`       | T (func) | ✅ | ✅ | `int(int,int,int,int)` |
| 4 | `divide_op`         | T (func) | ✅ | ✅ | `int(int,int,int,int)` |
| 5 | `modulo_op`         | T (func) | ✅ | ✅ | `int(int,int,int,int)` |
| 6 | `find_node_by_id`   | T (func) | ✅ | ✅ | `TreeNode*(int)` |
| 7 | `add_tree_node`     | T (func) | ✅ | ✅ | `int(int,int,int,const char*)` |
| 8 | `calculate_tree_sum`| T (func) | ✅ | ✅ | `int(int)` — recursive |
| 9 | `parse_operation`   | T (func) | ✅ | ✅ | `Operation(const char*)` |
|10 | `get_operation_func`| T (func) | ✅ | ✅ | `OperationFunc(Operation)` |
|11 | `inreftree`         | T (func) | ✅ | ✅ | `int(int,int,int,int)` — only decl in `lib.h` |
|12 | `node_table`        | B (data) | ✅ | ✅ | `TreeNode[50]`, 2600 bytes |
|13 | `node_count`        | B (data) | ✅ | ✅ | `int`, 4 bytes |

## Diff result

```
comm -23 c_syms r_syms   ->   (empty)
```

**0 symbols missing from the Rust `.so`. 0 undefined non-libc symbols.**
No stubs / `unimplemented!()` were used; every symbol is a real translation of
the corresponding C definition.

## ABI-level object checks

* `sizeof(TreeNode)` = 5*4 + 32 = **52** (align 4) in both; verified at runtime
  by striding `node_table` through `dlsym` in
  `tests/differential.rs::node_table_abi_layout_matches`.
* `node_table` total size **2600** bytes in both.
* `node_count` is an `int` (4 bytes) in both, initialised to 0.
* `get_operation_func` returns a pointer that must resolve to that library's own
  `*_op` symbol; verified by comparing against `dlsym` of each library
  (`get_operation_func_returns_matching_symbol`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Enumerated mechanically:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # -> no match
```

Therefore `--no-default-features` and the default build are the same code, and
Phases B–C cover 100% of the feature space. Both are still run explicitly by
`run_all.sh`.
