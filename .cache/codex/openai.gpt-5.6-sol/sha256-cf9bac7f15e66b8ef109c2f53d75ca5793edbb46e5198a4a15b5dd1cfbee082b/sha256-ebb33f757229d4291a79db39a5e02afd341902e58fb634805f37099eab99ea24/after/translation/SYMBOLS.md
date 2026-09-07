# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-NKkSCG.so
```

The C shared library exports exactly these library-defined public symbols:

| C symbol | C type | Rust export | implementation |
|---|---|---|---|
| `cleanup` | `T` | present | translated in `src/lib.rs` |
| `cleanup_resources` | `T` | present | translated in `src/lib.rs` |
| `print_result` | `T` | present | translated in `src/lib.rs` |

Missing from Rust: **none**.

Completion gate: [x] exact defined-symbol parity; [x] no unresolved non-libc
symbols (`ldd -r` clean).

The complete C `nm -D` output also contains undefined/weak runtime dependencies:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, `free`, `malloc`, `printf`, `puts`, `snprintf`, `strlen`, and
`strncmp`. These are libc/toolchain imports rather than library exports. The
Rust shared library resolves all C imports it uses through libc and has no
undefined non-libc application symbols.

There are no macro-generated public functions in the C source. `STRINGIZE` and
`TO_STRING` expand only the private string literal used by `cleanup`.
