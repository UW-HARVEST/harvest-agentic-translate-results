# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-ym5QOj.so

cd translation && cargo build --release
# -> translation/target/release/libbin2hex_lib.so
```

## C `.so` defined dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-ym5QOj.so`

| symbol | type | exported by Rust `.so`? |
|--------|------|-------------------------|
| `bin2hex` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn bin2hex` |

The C translation unit (`c_src/src/lib.c`) contains exactly one function
definition and no macro-generated / aliased / versioned symbols, no `static`
functions promoted to globals, and no data symbols. `include/lib.h` declares
only `bin2hex` and contains no renaming/namespacing macros, so the linker name
is plainly `bin2hex`.

## Rust `.so` defined dynamic symbols

`nm -D --defined-only translation/target/release/libbin2hex_lib.so`

| symbol | type |
|--------|------|
| `bin2hex` | `T` |

## Symbol diff

```
comm -23 <(C defined) <(Rust defined)   # symbols in C missing from Rust
<empty>
```

- Missing from Rust: **0**
- Extra in Rust (not an error, but none present beyond `bin2hex`): **0**

## Undefined (imported) symbols

C `.so` imports: `abort@GLIBC_2.2.5`, `__cxa_finalize@GLIBC_2.2.5`,
`__gmon_start__`, `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable` —
all libc/runtime. The Rust `.so` imports only libc/runtime symbols as well.

- Non-libc undefined symbols in Rust `.so`: **0**

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.

No module of the C source was skipped: `c_src/CMakeLists.txt` lists `src/lib.c`
as the sole source file, and it is fully translated in `translation/src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, therefore the only
feature combination is the default (empty) one. `default`, `--all-features`,
and `--no-default-features` are all identical builds. Verified by
`scripts/check_features.sh`.

## Build-configuration notes affecting these results

- `[lib] crate-type` is `["cdylib", "rlib"]`. The `rlib` is required only so the
  integration tests can be built against the crate; it does **not** change the
  `.so`, which still exports exactly `bin2hex` and nothing else.
- `cargo test` does **not** build the `cdylib` crate-type. A `cdylib`-only crate
  therefore lets `cargo test` run against a missing or stale `.so` and report a
  green result for code that no longer exists. `tests/common/mod.rs` guards
  against this: it builds the cdylib itself when absent or older than
  `src/lib.rs`, and hard-fails if the artifact is still stale afterwards.
- Run everything with `./scripts/verify_all.sh`, which rebuilds the C `.so`,
  enumerates the feature power set from `Cargo.toml`, and for every
  (feature-combination x profile) pair diffs `nm -D` and runs both test suites.

## Divergences found and fixed during verification

Both were profile-dependent behavioural differences in the null-pointer /
faulting path, found by Phase C rows E13–E16 and E18:

1. The translation formed `&[u8]` / `&mut [u8]` slices over the caller's raw
   pointers via `slice::from_raw_parts{,_mut}`. Those carry a debug-assertion
   precondition check that rejects null pointers by **aborting** (`SIGABRT`),
   whereas the C — which performs no null check — dies with `SIGSEGV`.
2. Replacing the slices with the plain `*p` deref operator was not sufficient:
   rustc emits its own null-pointer check for raw dereferences under
   `-C debug-assertions`, producing "null pointer dereference occurred" and
   again `SIGABRT` instead of `SIGSEGV`.

The fix is `core::ptr::read` / `core::ptr::write`, which are unchecked and
reproduce the C's faulting behaviour identically in both `dev` and `release`.
