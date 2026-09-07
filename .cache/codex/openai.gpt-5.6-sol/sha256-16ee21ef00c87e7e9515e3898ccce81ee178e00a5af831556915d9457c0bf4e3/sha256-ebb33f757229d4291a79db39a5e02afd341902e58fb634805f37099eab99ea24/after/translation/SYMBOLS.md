# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-2TNgXE.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `hdr_bitrate` | `T` | `hdr_bitrate` (`T`) | present |

The C shared object has one defined public dynamic symbol. The undefined weak
entries shown by plain `nm -D` (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`) are runtime
imports, not library API definitions.

Completion: [x] 0 C-defined dynamic symbols are missing from the Rust shared
object.

The final sorted symbol-set diff has 0 missing and 0 extra entries.

## Build Matrix

`Cargo.toml` declares no features and no binary target. `c_src/CMakeLists.txt`
declares only a shared-library target. The differential suite passes in release
mode for both Cargo configurations that exist:

- [x] default feature state;
- [x] `--no-default-features` (equivalent because no features are declared).
