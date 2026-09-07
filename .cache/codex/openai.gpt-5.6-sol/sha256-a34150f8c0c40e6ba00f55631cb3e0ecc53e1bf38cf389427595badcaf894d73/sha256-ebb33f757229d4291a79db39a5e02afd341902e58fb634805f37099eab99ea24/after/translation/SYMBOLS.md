# Dynamic Symbol Surface

Generated mechanically from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

Only defined public symbols are part of the implementation surface. Undefined
runtime/libc symbols are not library API symbols.

| C symbol | C type | Rust symbol present | Translation |
|----------|--------|---------------------|-------------|
| `bad` | `T` | [x] | `src/lib.rs` |
| `driver` | `T` | [x] | `src/lib.rs` |
| `good` | `T` | [x] | `src/lib.rs` |
| `printIntLine` | `T` | [x] | `src/lib.rs` |
| `printLine` | `T` | [x] | `src/lib.rs` |

## Raw C defined-symbol output

```text
00000000000011a2 T bad
00000000000012a1 T driver
000000000000127b T good
000000000000117b T printIntLine
0000000000001159 T printLine
```

## Phase D parity

- [x] All five C-defined dynamic symbols have exact-name Rust exports.
- [x] The C-minus-Rust defined-symbol diff is empty.
- [x] Rust has no undefined non-runtime implementation symbols.
