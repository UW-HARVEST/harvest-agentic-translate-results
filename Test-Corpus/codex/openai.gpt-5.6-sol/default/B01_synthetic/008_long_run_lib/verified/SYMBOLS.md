# Dynamic Symbol Surface

Reference library:
`../c_src/build/liblong.so`

Rust library:
`target/release/liblong.so`

The export inventory is generated with:

```sh
nm -D --defined-only ../c_src/build/liblong.so
nm -D --defined-only target/release/liblong.so
```

## C exports

| symbol | C type | C size | Rust export | status |
|---|---:|---:|---|---|
| `array` | `B` (object) | 1,048,576 bytes | `B` (object), 1,048,576 bytes | present |
| `long_exec` | `T` (function) | 174 bytes | `T` (function) | present |
| `perform_expensive_operations` | `T` (function) | 187 bytes | `T` (function) | present |

Raw C output:

```text
0000000000004060 B array
00000000000011f4 T long_exec
0000000000001139 T perform_expensive_operations
```

## Imported runtime symbols

The C library has the strong libc imports `printf`, `rand`, and `srand`.
It also has the weak toolchain imports `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.
These are dynamic imports rather than C library exports and therefore are not
part of the export-parity set.

## Parity

```sh
comm -23 \
  <(nm -D --defined-only ../c_src/build/liblong.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/liblong.so | awk '{print $3}' | sort -u)
```

The command produces no output: zero C exports are missing from Rust.
