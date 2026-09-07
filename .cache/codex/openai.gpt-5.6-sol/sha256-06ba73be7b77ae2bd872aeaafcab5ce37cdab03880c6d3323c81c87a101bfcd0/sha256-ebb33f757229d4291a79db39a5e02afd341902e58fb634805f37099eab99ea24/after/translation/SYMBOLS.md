# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libSimpleList.so
```

| C address | type | symbol | Rust export | status |
|-----------|------|--------|-------------|--------|
| `00000000000010f9` | `T` | `smallestValue` | `smallestValue` | present |

The Rust export was checked with:

```text
nm -D --defined-only target/release/libSimpleList.so
```

Missing C symbols: **0**

