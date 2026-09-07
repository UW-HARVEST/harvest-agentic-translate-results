# SYMBOLS.md — Phase A symbol surface

C `.so`: `c_src/build/libharvest-work-YYxvDp.so`
Rust `.so`: `translation/target/release/libmaxnmin_lib.so`

Derived mechanically with:

```sh
nm -D --defined-only <so> | awk '{print $2, $3}' | sort
```

## Exported (defined) symbols

| # | symbol | C type | in C `.so` | in Rust `.so` | notes |
|---|--------|--------|-----------|---------------|-------|
| 1 | `add_node`              | T | yes | yes | `int add_node(int, int, const char*, double)` |
| 2 | `calculate_subtree_sum` | T | yes | yes | `double calculate_subtree_sum(int)` |
| 3 | `find_node_by_id`       | T | yes | yes | `Node* find_node_by_id(int)` |
| 4 | `get_children_count`    | T | yes | yes | `int get_children_count(int)` |
| 5 | `maxnmin`               | T | yes | yes | `int maxnmin(int,int,int,int)` — the only symbol in `include/lib.h` |
| 6 | `process_string`        | T | yes | yes | `int process_string(char*)` |
| 7 | `safe_double_to_int`    | T | yes | yes | `int safe_double_to_int(double)` |

## Non-exported C state (internal, `static`)

| C declaration | Rust counterpart | exported? |
|---|---|---|
| `static Node node_storage[MAX_NODES]` | `static mut NODE_STORAGE: [Node; 100]` | no (correct — `static` in C has internal linkage) |
| `static int node_count` | `static mut NODE_COUNT: c_int` | no (correct) |

These are deliberately *not* exported by either library. All observation of the
shared mutable state happens indirectly through the 7 exported functions, so the
differential tests drive that state via `add_node` / `maxnmin`.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-YYxvDp.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libmaxnmin_lib.so | awk '{print $3}' | sort)
<empty>
```

**Result: 0 missing symbols. No macro-generated symbols exist in the C source
(`MAX_NODES` / `MAX_NAME_LEN` are object-like constant macros only).**

Undefined (imported) symbols in the Rust `.so` are limited to libc/runtime
(`memcpy`, `__stack_chk_fail`-class, `_ITM_*`, `__cxa_*`, `rust_eh_personality`
is absent because `panic = "abort"`); there are no unresolved project symbols.
