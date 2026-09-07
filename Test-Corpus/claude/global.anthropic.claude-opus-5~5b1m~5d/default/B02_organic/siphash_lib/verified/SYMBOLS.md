# SYMBOLS.md — Public symbol surface

C shared object: `c_src/build/libharvest-work-ZXAbRc.so`
Rust shared object: `translation/target/release/libsiphash_lib.so`

Command used: `nm -D --defined-only <so>`

## C exported symbols (source of truth)

| # | symbol | type | C signature | exported by Rust `.so`? |
|---|--------|------|-------------|-------------------------|
| 1 | `siphash`          | T (global text) | `void siphash(int init)`                              | YES |
| 2 | `stbds_hash_bytes` | T (global text) | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | YES |

## Not exported (and must NOT be)

| symbol | reason |
|--------|--------|
| `stbds_siphash_bytes` | `static` in `c_src/src/lib.c` → internal linkage, absent from `nm -D`. Kept private (non-`pub`, no `#[no_mangle]`) in Rust. |

## Undefined / imported symbols

The Rust `.so` imports only libc symbols (`printf`, plus the standard
`memcpy`/unwind/`__cxa`-style runtime symbols the Rust toolchain always emits).
The C `.so` imports `printf` from libc.

Verified: `nm -D --defined-only` output of both objects is identical after
sorting by name.

## Test-only artifacts (not part of the library ABI)

`src/bin/siphash_driver.rs` builds a small `siphash_driver` executable used only
by the tests: it `dlopen`s a given `.so`, calls its `siphash`, and writes nothing
else to stdout, so the C and Rust stdout can be compared byte-for-byte in a
clean child process.  It has no dependencies and does not link into the cdylib.

## Reproducing

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# Rust  (NOTE: `cargo test` does NOT build a cdylib -- build it explicitly,
#        or tests silently load a stale .so; tests/phase_a_freshness.rs guards this)
cd translation && cargo build --release && cargo test --release
# diff
diff <(nm -D --defined-only ../c_src/build/lib*.so      | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/release/libsiphash_lib.so | awk '{print $3}' | sort)
```

Result of the above `diff`: **empty**.

## Result

- Missing symbols in Rust: **0**
- Extra non-libc symbols in Rust: **0**
- Undefined non-libc symbols in Rust: **0**

STATUS: **PASS**
