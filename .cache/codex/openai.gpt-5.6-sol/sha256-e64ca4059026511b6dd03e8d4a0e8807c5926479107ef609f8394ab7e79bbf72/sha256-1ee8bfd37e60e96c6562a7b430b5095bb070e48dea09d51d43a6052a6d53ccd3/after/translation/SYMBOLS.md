# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-TP6lE1.so`

The defined public surface was extracted with:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-TP6lE1.so
```

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `bin2hex` | `T` | `bin2hex` (`T`) | [x] |

The complete C `nm -D` output also contains the undefined libc symbol
`abort@GLIBC_2.2.5` and the weak toolchain/loader symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. These are not library API
definitions. There are zero missing C-defined symbols and zero undefined
non-libc API symbols in the Rust library.
