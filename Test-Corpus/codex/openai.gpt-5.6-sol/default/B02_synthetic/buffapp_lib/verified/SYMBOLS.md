# Dynamic symbol surface

Source library:
`../c_src/build/libharvest-work-s5WMbT.so`

Rust library:
`target/release/libbuffapp_lib.so`

Derived with:

```sh
nm -D --defined-only <library> | awk '{print $3}' | sort -u
```

| C symbol | C source definition | Rust export | Status |
|----------|---------------------|-------------|--------|
| `append_to_buffer` | `src/lib.c:53` | `append_to_buffer` | [x] |
| `buffapp` | `src/lib.c:111` | `buffapp` | [x] |
| `create_buffer` | `src/lib.c:34` | `create_buffer` | [x] |
| `destroy_buffer` | `src/lib.c:75` | `destroy_buffer` | [x] |
| `get_operation_name` | `src/lib.c:84` | `get_operation_name` | [x] |
| `perform_operation` | `src/lib.c:94` | `perform_operation` | [x] |

The sorted C-minus-Rust symbol difference is empty. The C library's undefined
symbols are libc/runtime imports and are not API definitions.
