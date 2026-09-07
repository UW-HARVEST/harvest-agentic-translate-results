# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libhello.so`
* Rust: `translation/target/release/libhello.so`

## C source inventory (completeness check)

Every C translation unit compiled into the C `.so` (from `c_src/CMakeLists.txt`,
`add_library(hello SHARED src/hello.c)`):

| C source file | public header | translated to | status |
|---|---|---|---|
| `c_src/src/hello.c` | `c_src/include/hello.h` | `translation/src/hello.rs` | TRANSLATED |

No C source file in `c_src/` is untranslated; there is exactly one translation
unit and exactly one public function (`int helloworld();` in `hello.h`).

## Defined (exported) dynamic symbols

`nm -D --defined-only <lib> | awk '{print $2, $3}'`

| symbol | C `.so` | Rust `.so` | notes |
|---|---|---|---|
| `helloworld` | `T` (text, global) | `T` (text, global) | the single public API function; exported from Rust via `#[unsafe(no_mangle)] pub extern "C"` |

ELF/toolchain-generated entries that `nm -D` may additionally list are not part
of the library API surface and are excluded from the parity requirement:

* `_init`, `_fini`, `__bss_start`, `_edata`, `_end` (linker-provided)
* `__cxa_finalize@GLIBC_*`, `_ITM_*`, `__gmon_start__`,
  `__register_frame_info` / `__deregister_frame_info` (weak, undefined)
* `rust_eh_personality` and `_ZN*` Rust-internal mangled symbols (Rust-only,
  extra symbols in the Rust `.so` are permitted; only *missing* ones are a bug)

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` |
|---|---|---|
| `puts` / `printf` (libc stdio) | imported | imported (`printf` via `unsafe extern "C"`) |

The C compiler is free to rewrite `printf("Hello World!\n")` into `puts("Hello World!")`
(the standard `printf`→`puts` optimisation). Both spellings write the identical
byte sequence `Hello World!\n` to `stdout` through the same glibc `FILE` stream,
so the observable output is unchanged. The Rust side deliberately calls libc
`printf` (rather than Rust `println!`) so that the bytes go through the *same*
`stdout` FILE object and inherit the same buffering semantics.

## Parity result

```
comm -23 <(nm -D --defined-only c_src/build/libhello.so    | awk '$2=="T"{print $3}' | sort) \
         <(nm -D --defined-only translation/target/release/libhello.so | awk '$2=="T"{print $3}' | sort)
```

Output: *(empty)* — 0 symbols exported by the C `.so` are missing from the Rust
`.so`. No stubs, no `unimplemented!()`; the single symbol is a real translation.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
