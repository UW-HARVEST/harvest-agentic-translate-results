# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver_c.so`.

The supplied CMake target is an executable. The shared object was built from the
same unmodified translation unit with:

```text
cc -shared -fPIC -O2 -o c_src/build/libdriver_c.so c_src/src/main.c
```

| # | C symbol | Kind | Rust export | Status |
|---|----------|------|-------------|--------|
| 1 | `main` | function | yes | [x] |
| 2 | `calculate_checksum` | function | yes | [x] |
| 3 | `validate_buffer` | function | yes | [x] |
| 4 | `init_buffer_array` | function | yes | [x] |
| 5 | `free_buffer_array` | function | yes | [x] |
| 6 | `buffer_copy` | function | yes | [x] |
| 7 | `buffer_reverse` | function | yes | [x] |
| 8 | `buffer_merge` | function | yes | [x] |
| 9 | `buffer_split` | function | yes | [x] |
| 10 | `buffer_interleave` | function | yes | [x] |
| 11 | `buffer_rotate` | function | yes | [x] |
| 12 | `buffer_conditional_copy` | function | yes | [x] |
| 13 | `buffer_copy_strided` | function | yes | [x] |
| 14 | `process_buffer_array` | function | yes | [x] |
| 15 | `read_buffer` | function | yes | [x] |
| 16 | `write_buffer` | function | yes | [x] |

Initial finding: none of these symbols were exported because the Rust package
only contained a binary. `init_buffer_array`, `free_buffer_array`,
`buffer_conditional_copy`, `buffer_copy_strided`, and
`process_buffer_array` were also absent from the translation. They require real
implementations, while the remaining functions require C ABI wrappers.

Final finding: all 16 symbols are implemented and exported by
`target/release/libdriver.so`; `comm` over the two defined dynamic-symbol lists
is empty.

Undefined C symbols are GLIBC/runtime imports only:
`__isoc99_scanf`, `fprintf`, `free`, `fwrite`, `malloc`, `memcpy`, `printf`,
`putchar`, and `stderr` (plus weak ELF runtime hooks).
