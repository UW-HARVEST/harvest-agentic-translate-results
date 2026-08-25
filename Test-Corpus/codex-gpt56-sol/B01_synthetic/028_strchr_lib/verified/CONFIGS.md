# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table and CMake defines no options or
conditional compilation. There is exactly one valid feature combination:

| # | Cargo feature set | C configuration | [ ] |
|---|-------------------|-----------------|-----|
| F1 | empty (`--no-default-features --features ""`) | default | [x] |

## Runtime Matrix

The C API has no mutable state or runtime options. Its runtime axes are the
exported entry point, NUL-terminated byte-string shape, searched byte class,
and the zero/one/many match counts that select the loop path. `driver` fixes
the searched bytes to `A` and `x`, producing the pruned 3 by 3 cross-product
of their count classes. Each row is exercised with many fixed-seed randomized
inputs.

`foo(in, '\0')` is excluded from the valid surface: after finding the string
terminator, the C loop increments the pointer beyond the C string and searches
outside the object, which is undefined behavior.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `foo` | ASCII non-NUL search byte; zero matches; empty/one/many-byte strings | [x] |
| C2 | `foo` | ASCII non-NUL search byte; exactly one match; one/many-byte strings | [x] |
| C3 | `foo` | ASCII non-NUL search byte; multiple matches; many-byte strings | [x] |
| C4 | `foo` | high-bit search byte (`0x80..0xff`); zero matches; empty/one/many-byte strings | [x] |
| C5 | `foo` | high-bit search byte (`0x80..0xff`); exactly one match; one/many-byte strings | [x] |
| C6 | `foo` | high-bit search byte (`0x80..0xff`); multiple matches; many-byte strings | [x] |
| C7 | `foo`, `driver` | buffer has bytes after the first NUL; only the C-string prefix is observed | [x] |
| C8 | `driver` | `A` count zero; `x` count zero; empty/irrelevant-byte strings | [x] |
| C9 | `driver` | `A` count zero; `x` count one | [x] |
| C10 | `driver` | `A` count zero; `x` count many | [x] |
| C11 | `driver` | `A` count one; `x` count zero | [x] |
| C12 | `driver` | `A` count one; `x` count one | [x] |
| C13 | `driver` | `A` count one; `x` count many | [x] |
| C14 | `driver` | `A` count many; `x` count zero | [x] |
| C15 | `driver` | `A` count many; `x` count one | [x] |
| C16 | `driver` | `A` count many; `x` count many | [x] |
