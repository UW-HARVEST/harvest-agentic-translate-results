# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory

The whole C library is two files:

| file | contents |
|------|----------|
| `c_src/include/driver.h` | one declaration: `void driver(int x, int y);` |
| `c_src/src/driver.c` | one definition: `driver` |

Nothing else exists in the C tree, so there is no untranslated module.

Note: the C sources are written with **C digraphs** (`%:` = `#`, `<%` = `{`,
`%>` = `}`) and **`<iso646.h>` alternative operator spellings** (`bitor` = `|`,
`compl` = `~`). `gcc -E` output confirms `driver.c` line 30 expands to:

```
int result = x | ~ y;
```

## `nm -D --defined-only` — exported (dynamic) symbols

### C `libdriver.so`

| addr | type | symbol |
|------|------|--------|
| `0000000000001119` | `T` | `driver` |

Total exported: **1**

### Rust `libdriver.so`

| addr | type | symbol |
|------|------|--------|
| `0000000000011700` | `T` | `driver` |

Total exported: **1**

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

**Missing from Rust: none. Extra in Rust: none.** Symbol parity is exact,
including the fact that the Rust `.so` exports *no* extra `pub extern` helpers.

The export wrapper in Rust is:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int)
```

## Undefined (imported) symbols — informational only

Imports are *not* required to match: they are an artifact of the language
runtime, not of the library's public contract. Recorded here for completeness.

| symbol | C | Rust | note |
|--------|---|------|------|
| `printf@GLIBC_2.2.5` | U | U | both emit the number through C `printf("%d")` |
| `puts@GLIBC_2.2.5` | U | — | C calls `puts("")` |
| `putchar@GLIBC_2.2.5` | — | U | LLVM rewrote the Rust `puts("")` into `putchar('\n')` — an established, observably-equivalent libc optimization (GCC performs the same rewrite in other builds). Byte stream is identical: a single `'\n'` into the same `stdout` `FILE`. Verified by the differential tests. |
| `_Unwind_*`, `malloc`, `memcpy`, `dl_iterate_phdr`, `pthread_key_*`, … | — | U/w | Rust `std` runtime (panic machinery, allocator shims). Not reachable from `driver`. |

Both objects resolve `printf`/`puts`/`putchar` against the *same*
process-wide glibc, so they share one `stdout` `FILE` object — which is what
makes the interleaving tests in Phase B meaningful.

## Checklist

- [x] every symbol exported by the C `.so` is exported by the Rust `.so`, same name
- [x] `nm -D` diff of defined symbols is empty
- [x] no stubs / `unimplemented!()` anywhere in the Rust crate
- [x] no C source file left untranslated

## Harness note: a stale-artifact trap that made the suite vacuous

`cargo test` does **not** emit the `cdylib` artifact — a `cdylib` cannot be
linked by an integration test, so cargo skips producing `target/<profile>/libdriver.so`.
A harness that simply picks that path up therefore loads whatever `.so` a
*previous* `cargo build` left behind. This was caught with a deliberate
mutation (`x | !y` → `x ^ !y`): the entire suite still passed, because it was
comparing the C library against a stale Rust object.

`tests/common/mod.rs::rust_so_path()` now **builds the cdylib itself** (into
`target/ffi-so`, a separate target dir so it cannot contend with the parent
cargo's build lock) and asserts the artifact is newer than `src/lib.rs`. Two
further guards were added:

* `d5_harness_loads_two_distinct_objects` uses `dladdr` to prove the C and Rust
  `driver` pointers originate from *different* `.so` files, so the suite can
  never silently compare the C library against itself.
* `scripts/mutate.py` re-runs the sensitivity check on demand.

Confirmed origins at run time:

```
C   driver <- c_src/build/libdriver.so
Rust driver <- translation/target/ffi-so/release/libdriver.so
```
