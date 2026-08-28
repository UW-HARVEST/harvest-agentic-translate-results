# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|if\s*\(|<=|>=|<|>|sizeof|min|max|NULL' ../c_src/src ../c_src/include
```

The only public entry point is `int memchra2(int, int, int, int)`. It has no
pointer, length, enum, option, or rejection/error return contract: every
possible FFI input is four valid C `int` values. The checks below are all in
`static` functions, and `memchra2` invokes them only with fixed non-null,
non-empty local data. They are tested through test-only C and Rust shared
objects that include the unmodified implementations and export guard wrappers;
the integration test loads both objects with `libloading`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `process_buffer` (static) | `buffer == NULL` | `-1` | [x] |
| 2 | `process_buffer` (static) | `*buffer == '\0'` | `-1` | [x] |
| 3 | `process_strings` (static) | `strings == NULL` | `0` | [x] |
| 4 | `process_strings` (static) | `count <= 0` | `0` | [x] |
| 5 | `process_strings` (static) | current element `*i == NULL` | skip element; no increment | [x] |
| 6 | `process_strings` (static) | current string `**i == '\0'` | skip element; no increment | [x] |
| 7 | `safe_sum_array` (static) | `arr == NULL` | `0` | [x] |
| 8 | `safe_sum_array` (static) | `size == 0` | `0` | [x] |
| 9 | `interpret_as_int` (static) | `bytes == NULL` | `0` | [x] |
| 10 | `interpret_as_int` (static) | `len < sizeof(int)` | `0` | [x] |
| 11 | `count_occurrences` (static) | `text == NULL` | `0` | [x] |
| 12 | `count_occurrences` (static) | `*text == '\0'` | `0` | [x] |
| 13 | `complex_iteration` (static) | `data == NULL` | `-1` | [x] |
| 14 | `complex_iteration` (static) | `count == 0` | `-1` | [x] |

## Public FFI Boundary Coverage

- [x] No nullable arguments exist.
- [x] No length arguments exist, so zero/oversized lengths do not exist.
- [x] No enum arguments exist, so out-of-range enum representations do not
  exist.
- [x] All four arguments are tested at `INT_MIN`, `INT_MAX`, zero, and broad
  randomized values through both shared libraries.
- [x] Every internal guard above is unreachable from `memchra2` by construction:
  its buffer starts with `"test"`, arrays have four elements, and all local
  pointers are non-null.
