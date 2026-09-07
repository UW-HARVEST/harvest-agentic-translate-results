# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-KDzdfj.so
nm -D --defined-only target/release/libjumpnode_lib.so
```

## Required public exports

| C symbol | C type | Rust symbol present | Notes |
|----------|--------|---------------------|-------|
| `jumpnode` | `T` | yes | Exact unmangled name and `extern "C"` ABI |

Missing required exports: **0**

## C dynamic imports (not library exports)

The unfiltered `nm -D`/`readelf -Ws` output also contains the undefined runtime
imports `strlen`, `sprintf`, and `sqrt`, plus ELF toolchain weak symbols. These
are implementation dependencies, not public symbols defined by the C library,
and therefore are not export-parity requirements.

## Final parity command

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libharvest-work-KDzdfj.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libjumpnode_lib.so | awk '{print $3}' | sort -u)
```

Final missing-symbol diff: **empty**

`ldd -r target/release/libjumpnode_lib.so` reports no unresolved symbols.
