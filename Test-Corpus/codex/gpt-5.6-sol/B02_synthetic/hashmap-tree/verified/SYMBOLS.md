# Dynamic Symbol Surface

The C shared object is built from the library implementation files named by
`c_src/CMakeLists.txt` (excluding its test-driver `main.c`):

```sh
cc -fPIC -shared -Ic_src/include c_src/src/hashmap.c c_src/src/tree.c \
  -o c_src/build/libdriver_c.so
```

The symbol list is the mechanically sorted output of:

```sh
nm -D --defined-only c_src/build/libdriver_c.so
```

| # | C symbol | Declared in | Rust `.so` |
|---|----------|-------------|------------|
| 1 | `hashmap_clear` | `include/hashmap.h` | present |
| 2 | `hashmap_contains` | `include/hashmap.h` | present |
| 3 | `hashmap_create` | `include/hashmap.h` | present |
| 4 | `hashmap_destroy` | `include/hashmap.h` | present |
| 5 | `hashmap_get` | `include/hashmap.h` | present |
| 6 | `hashmap_put` | `include/hashmap.h` | present |
| 7 | `hashmap_remove` | `include/hashmap.h` | present |
| 8 | `hashmap_size` | `include/hashmap.h` | present |
| 9 | `tree_add_node` | `include/tree.h` | present |
| 10 | `tree_contains` | `include/tree.h` | present |
| 11 | `tree_count_descendants` | `include/tree.h` | present |
| 12 | `tree_create` | `include/tree.h` | present |
| 13 | `tree_delete` | `include/tree.h` | present |
| 14 | `tree_find_path` | `include/tree.h` | present |
| 15 | `tree_get_depth` | `include/tree.h` | present |
| 16 | `tree_get_height` | `include/tree.h` | present |
| 17 | `tree_get_node` | `include/tree.h` | present |
| 18 | `tree_print` | `include/tree.h` | present |
| 19 | `tree_remove_node` | `include/tree.h` | present |
| 20 | `tree_size` | `include/tree.h` | present |

Missing C symbols in the Rust shared object: **0**.

Enforced by integration test `dynamic_symbol_surface_matches`.
