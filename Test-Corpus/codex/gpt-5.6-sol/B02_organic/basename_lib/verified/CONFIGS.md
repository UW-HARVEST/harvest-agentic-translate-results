# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no CMake
options or conditional compilation. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | [ ] |
|---|-------------------|-----------------|-----|
| F1 | empty set (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

The sole public entry point is the lowest-level API:

```c
char *tool_basename(char *path);
```

It has no runtime flags or modes. Rows below are derived from the
`strrchr(path, '/')`, `strrchr(path, '\\')`, `if`/`else if` branches, pointer
comparison, and C-string termination semantics in `c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tool_basename` | empty C string; neither separator is present | [x] |
| 2 | `tool_basename` | nonempty C string with neither separator present | [x] |
| 3 | `tool_basename` | one or many `/` separators and no `\\`; randomized leading, interior, and trailing last separator | [x] |
| 4 | `tool_basename` | one or many `\\` separators and no `/`; randomized leading, interior, and trailing last separator | [x] |
| 5 | `tool_basename` | both separator types present and the last `/` occurs after the last `\\` | [x] |
| 6 | `tool_basename` | both separator types present and the last `\\` occurs after the last `/` | [x] |
| 7 | `tool_basename` | backing buffer has bytes after the first NUL; only the C-string prefix participates | [x] |

For every row, parity includes the returned offset into the caller-owned
buffer, returned suffix bytes through the NUL terminator, and absence of input
mutation.
