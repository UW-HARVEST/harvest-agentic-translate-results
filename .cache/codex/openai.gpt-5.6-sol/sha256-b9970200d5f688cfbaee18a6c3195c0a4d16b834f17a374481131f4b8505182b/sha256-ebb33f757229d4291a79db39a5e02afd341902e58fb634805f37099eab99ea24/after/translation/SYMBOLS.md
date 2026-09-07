# Dynamic symbol surface

Source: `nm -D --defined-only ../c_src/build/libharvest-work-KE36At.so`.

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `half2float` | `half2float` | Present |

There are no missing C exports in the Rust shared library.

Final audit: the sorted `nm -D --defined-only` symbol sets are identical, and
`ldd -r` reports no unresolved dynamic symbols. The Rust library's dynamic
imports are standard libc/libgcc runtime symbols, not missing project symbols.
