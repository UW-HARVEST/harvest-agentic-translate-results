# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/luggage.c` with `cc -shared -fPIC -O2`.

Inventory command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| # | C symbol | Rust symbol | Status |
|---|----------|-------------|--------|
| 1 | `addRoutingDirectiveToList` | `addRoutingDirectiveToList` | present |
| 2 | `main` | `main` | present |
| 3 | `matches` | `matches` | present |
| 4 | `printMatchingDirectives` | `printMatchingDirectives` | present |
| 5 | `superseded` | `superseded` | present |
| 6 | `supersedes` | `supersedes` | present |

Defined C symbols missing from `target/debug/deps/libdriver.so`: **0**.
Undefined non-libc C symbols missing from Rust: **0**.
