# Dynamic Symbol Surface

Ground truth: `nm -D --defined-only c_src/build/libdag_c.so`, built from
`c_src/src/lib.c`.

`c_src/CMakeLists.txt` has one executable target and no build options. The
shared object used for this inventory was compiled from the same source and
public include directory with position-independent code.

| # | C symbol | Rust export | Phase A disposition |
|---|----------|-------------|---------------------|
| 1 | `add_edge` | [x] | Translated and exported with the exact C symbol name. |
| 2 | `add_node` | [x] | Translated and exported with the exact C symbol name. |
| 3 | `create_graph` | [x] | Translated and exported with the exact C symbol name. |
| 4 | `delete_node` | [x] | Translated and exported with the exact C symbol name. |
| 5 | `find_shortest_path` | [x] | Translated and exported with the exact C symbol name. |
| 6 | `free_graph` | [x] | Translated and exported with the exact C symbol name. |
| 7 | `get_node_by_name` | [x] | Translated and exported with the exact C symbol name. |
| 8 | `print_graph` | [x] | Translated and exported with the exact C symbol name. |
| 9 | `print_node` | [x] | Translated and exported with the exact C symbol name. |
| 10 | `shallow_copy` | [x] | Translated and exported with the exact C symbol name. |

The C object has no undefined non-libc application symbols. The final C-to-Rust
required-symbol diff is empty, and `ldd -r target/debug/libdriver.so` reports
no unresolved relocations.
