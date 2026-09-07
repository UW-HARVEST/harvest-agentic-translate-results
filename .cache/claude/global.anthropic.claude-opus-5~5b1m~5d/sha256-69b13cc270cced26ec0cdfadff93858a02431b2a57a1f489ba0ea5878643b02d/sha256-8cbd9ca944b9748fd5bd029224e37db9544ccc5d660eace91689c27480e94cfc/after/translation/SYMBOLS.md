# SYMBOLS.md — Phase A symbol surface

C shared library: `c_src/build/libharvest-work-1waoDg.so`
Rust shared library: `translation/target/release/libcharinbuf_lib.so`

Command used:

```sh
nm -D --defined-only <so> | awk '{print $2, $3}' | sort
```

## Defined (exported) symbols

| # | symbol | C type | in C `.so` | in Rust `.so` | C signature (from `c_src/src/lib.c`) |
|---|--------|--------|-----------|---------------|--------------------------------------|
| 1 | `apply_operation`      | T | yes | yes | `int apply_operation(int (*op)(int), int value)` |
| 2 | `charinbuf`            | T | yes | yes | `int charinbuf(int mode, int value, int opt1, int opt2)` |
| 3 | `create_buffer`        | T | yes | yes | `char *create_buffer(const char *initial)` |
| 4 | `decrement_counter`    | T | yes | yes | `int decrement_counter(int value)` |
| 5 | `find_char_in_buffer`  | T | yes | yes | `char *find_char_in_buffer(const char *buffer, size_t size, char target)` |
| 6 | `increment_counter`    | T | yes | yes | `int increment_counter(int value)` |
| 7 | `is_string_empty`      | T | yes | yes | `int is_string_empty(const char *str)` |
| 8 | `multiply_counter`     | T | yes | yes | `int multiply_counter(int value)` |
| 9 | `reset_counter`        | T | yes | yes | `int reset_counter(int value)` |
| 10 | `validate_uint16_range` | T | yes | yes | `int validate_uint16_range(int value)` |

**Symbol diff (C minus Rust): EMPTY.** No missing symbols, so no export
wrappers to add and no untranslated C module.

Note: `c_src/include/lib.h` only declares `charinbuf`, but `lib.c` gives
external linkage to the nine helpers as well (only `counter` is `static`), so
all ten are part of the ABI surface and all ten are verified differentially.

## Non-exported / internal C state (not a symbol, but observable)

| name | kind | notes |
|------|------|-------|
| `counter` | `static int` (local symbol, not in `nm -D`) | Mutated by the four `*_counter` functions and reset to 0 at the top of `charinbuf`. Observable through return values and through mode 3's `Final static counter value:` line. Verified via call sequences. |
| `operation_func` | `typedef int (*)(int)` | Function-pointer type used by `apply_operation`; NULL is a valid input. |

## Undefined (imported) symbols

Rust `.so` undefined non-libc symbols: **0**. Verified by
`phase_d_rust_has_no_unresolved_imports`, which runs

```sh
ldd -r translation/target/release/libcharinbuf_lib.so
ldd -r c_src/build/libharvest-work-1waoDg.so
```

and asserts neither reports `undefined symbol` / `not found`.

The Rust `.so` imports the six libc primitives the C uses —
`printf`, `malloc`, `free`, `memchr`, `strlen`, `strcpy` — which is asserted by
`phase_d_rust_reuses_the_same_libc_primitives`. Reusing libc's own `printf`
guarantees byte-identical formatting/stdio buffering, and reusing
`malloc`/`free` keeps `create_buffer`'s result `free()`-able by the caller.
Beyond those, the Rust standard library legitimately imports a wider *libc*
surface (`mmap64`, `pthread_key_create`, `statx`, …); all of it resolves against
`libc.so.6`/`ld-linux`, so there are no dangling non-libc imports.

## Verification driver

`scripts/verify.sh` rebuilds both libraries and runs every phase against BOTH
the debug and the release Rust cdylib, for every cargo feature combination, then
diffs `nm -D` output. Symbol sets come out IDENTICAL.

The suite also passes when the C library is rebuilt with
`-DCMAKE_BUILD_TYPE=Release` (`-O3`), i.e. the agreement is not an artefact of
the default unoptimised C build (relevant because mode 3 exercises signed
integer overflow).
