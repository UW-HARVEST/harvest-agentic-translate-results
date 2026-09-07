# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands:

```
nm -D c_src/build/libdriver.so | grep -v ' U '
nm -D translation/target/release/libdriver.so | grep -v ' U '
```

## C `.so` exported (defined) symbols

| symbol | type | in Rust `.so`? |
|--------|------|----------------|
| `UTIL_createLinePointers` | `T` (global text) | YES (`T`) |
| `_ITM_deregisterTMCloneTable` | `w` (weak, toolchain-generated) | YES (`w`) |
| `_ITM_registerTMCloneTable` | `w` (weak, toolchain-generated) | YES (`w`) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` (weak, libc) | YES (`w`) |
| `__gmon_start__` | `w` (weak, toolchain-generated) | YES (`w`) |

## Missing symbols

**NONE.** The symbol diff is empty: every symbol defined/exported by the C
`.so` is also exported by the Rust `.so` under the exact same name.

The Rust `.so` additionally exports these weak libc-provided stubs, which come
from the Rust standard library's own use of libc and are *not* part of the
library API surface (they are weak undefined/defined libc references, allowed
extras, not missing symbols):

* `__cxa_thread_atexit_impl@GLIBC_2.18` (w)
* `gettid@GLIBC_2.30` (w)
* `statx@GLIBC_2.28` (w)

## Public API (from `c_src/include/lib.h`)

```c
#include <stdlib.h>
const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
```

Exactly one public entry point. There is no driver binary in
`c_src/CMakeLists.txt` (only `add_library(driver SHARED src/lib.c)`), so the
"compare binary stdout" clause of the completion gate is not applicable.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
feature combination is the default (empty) one. Verified with:

```
cargo check                       # default
cargo check --no-default-features # identical, no features exist
```
