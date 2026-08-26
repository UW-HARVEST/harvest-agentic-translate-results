# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C address | type | symbol | Rust export | status |
|-----------|------|--------|-------------|--------|
| `000000000000122d` | `T` | `ima_parse` | `ima_parse` | [x] |

The C library's remaining `nm -D` entries are undefined weak toolchain/libc
symbols: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. They are not library API
definitions.

Rust comparison command:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/deps/libima_parse_lib.so | awk '{print $3}' | sort -u)
```

Result: empty (zero missing C definitions).

`ldd -r target/debug/deps/libima_parse_lib.so` reports no unresolved symbols.
All undefined Rust entries are provided by its listed glibc or `libgcc_s`
runtime dependencies; none is an undefined library API symbol.
