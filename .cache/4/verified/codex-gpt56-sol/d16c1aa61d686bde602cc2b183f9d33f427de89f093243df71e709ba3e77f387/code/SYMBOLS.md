# Dynamic Symbol Surface

Source command:

```sh
nm -D --defined-only c_src/build/libdriver_c.so
```

The C shared library was built from every source listed by CMake:
`main.c`, `engine.c`, `a.c`, `b.c`, `util.c`, and `lib.c`.

| # | type | C symbol | Rust export |
|---|------|----------|-------------|
| 1 | T | `call_a_once` | [x] |
| 2 | T | `call_b_once` | [x] |
| 3 | T | `iv_free` | [x] |
| 4 | T | `iv_init` | [x] |
| 5 | T | `iv_peek` | [x] |
| 6 | T | `iv_pop` | [x] |
| 7 | T | `iv_push` | [x] |
| 8 | T | `iv_reserve` | [x] |
| 9 | T | `process_a_stream` | [x] |
| 10 | T | `process_b_stream` | [x] |
| 11 | T | `prog_fetch` | [x] |
| 12 | T | `prog_init` | [x] |
| 13 | T | `run_engine` | [x] |
| 14 | T | `target` | [x] |
| 15 | T | `vm_free` | [x] |
| 16 | T | `vm_init` | [x] |
| 17 | T | `vm_print` | [x] |
| 18 | T | `vm_trace` | [x] |
| 19 | T | `main` | [x] |

The C object's undefined dynamic references are libc functions (`fprintf`,
`fputc`, `free`, and `realloc`) plus toolchain weak symbols. They are runtime
dependencies, not C-defined library exports.
