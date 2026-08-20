# SYMBOLS.md — exported symbol parity (Phase A / Phase D)

## Build-time configuration surface

| side | configurations |
|------|----------------|
| `Cargo.toml` | **no `[features]` section at all** → exactly one combination: `--no-default-features` == default. Verified with `cargo check --no-default-features` and `cargo check` (both clean, zero warnings). |
| `c_src/CMakeLists.txt` | `add_executable(driver src/main.c src/engine.c src/a.c src/b.c src/util.c src/lib.c)`. No `option()`, no `target_compile_definitions`, no build-type-specific sources. |
| C preprocessor | `grep -rn '#if\|#define' c_src/` → only include guards (`UTIL_H`, `API_H`) and three function-like macros (`A_MAC_CALL`, `B_MAC_CALL`, `MAC_CALL`). **No `#ifdef` feature switches anywhere.** |

So there is a single build configuration; every table row below and in
`CONFIGS.md` / `ERRORS.md` applies to it (see also `scripts/run_all.sh`,
which loops over the whole (single) combination set).

## How the shared objects are built

`CMakeLists.txt` only declares an executable, so the `.so` used for the
differential tests is produced from the *same* translation units with `-fPIC
-shared` and **no `-O` flag** (matching the cmake default, which sets no
`CMAKE_BUILD_TYPE` and therefore no optimisation level):

```
gcc -fPIC -shared -o c_src/build/libdriver_c_full.so c_src/src/*.c   # all 6 files, incl. main.c
cargo build --release                                                # -> target/release/libdriver.so
```

Both are built by `scripts/build_all.sh`.

Every non-`static` function in the C sources has external linkage, therefore
all of them (including `main`) are exported by `libdriver_c_full.so`.  The Rust
side is a `cdylib` + `rlib`; `src/main.rs` is a `#![no_main]` shim so that the
library's `#[no_mangle] pub unsafe extern "C" fn main` is *both* the exported
`main` symbol of the `.so` and the real entry point of the `driver` binary —
exactly like the C build.

## Symbol table

`nm -D --defined-only` on both objects (19 symbols on each side, all `T`):

| # | C symbol (`libdriver_c_full.so`) | C source | in Rust `.so`? | Rust item |
|---|----------------------------------|----------|----------------|-----------|
| 1 | `iv_init` | util.c | yes | `#[no_mangle] iv_init` |
| 2 | `iv_free` | util.c | yes | `#[no_mangle] iv_free` |
| 3 | `iv_reserve` | util.c | yes | `#[no_mangle] iv_reserve` |
| 4 | `iv_push` | util.c | yes | `#[no_mangle] iv_push` |
| 5 | `iv_pop` | util.c | yes | `#[no_mangle] iv_pop` |
| 6 | `iv_peek` | util.c | yes | `#[no_mangle] iv_peek` |
| 7 | `prog_init` | util.c | yes | `#[no_mangle] prog_init` |
| 8 | `prog_fetch` | util.c | yes | `#[no_mangle] prog_fetch` |
| 9 | `vm_init` | util.c | yes | `#[no_mangle] vm_init` |
| 10 | `vm_free` | util.c | yes | `#[no_mangle] vm_free` |
| 11 | `vm_trace` | util.c | yes | `#[no_mangle] vm_trace` |
| 12 | `vm_print` | util.c | yes | `#[no_mangle] vm_print` |
| 13 | `run_engine` | engine.c | yes | `#[no_mangle] run_engine` |
| 14 | `target` | lib.c | yes | `#[no_mangle] target` |
| 15 | `call_a_once` | a.c | yes | `#[no_mangle] call_a_once` |
| 16 | `process_a_stream` | a.c | yes | `#[no_mangle] process_a_stream` |
| 17 | `call_b_once` | b.c | yes | `#[no_mangle] call_b_once` |
| 18 | `process_b_stream` | b.c | yes | `#[no_mangle] process_b_stream` |
| 19 | `main` | main.c | yes | `#[no_mangle] main` (lib.rs; entry point of the `driver` bin via `#![no_main]`) |

`static` C functions have internal linkage and are exported by neither side;
they are translated as private Rust `fn`s and are exercised through their
public callers:

| C static | file | Rust counterpart | reached through |
|----------|------|------------------|-----------------|
| `target` (file-local) | a.c | `a_target` | `call_a_once`, `process_a_stream`, `run_engine(impl 0)` |
| `a_bias_call`, `wrap` | a.c | `a_bias_call`, `a_wrap` | `call_a_once` |
| `state_a` | a.c | `static STATE_A` | all of a.c's entry points |
| `target` (file-local) | b.c | `b_target` | `call_b_once`, `process_b_stream`, `run_engine(impl 1)` |
| `b_twist_call`, `w2` | b.c | `b_twist_call`, `b_w2` | `call_b_once` |
| `flipflop` | b.c | `static FLIPFLOP` | all of b.c's entry points |
| `inline_call`, `MAC_CALL`, `classify`, `process_stream` | engine.c | `classify`, `process_stream` | `run_engine` |
| `usage`, `read_stdin` | main.c | `usage`, `read_stdin` | `main` |

## Verification

`scripts/symbol_parity.sh` performs the mechanical check and is also asserted
by the integration test `symbol_parity` in `tests/differential.rs`:

* `comm -23 c_syms rust_syms` (symbols in C but not in Rust) → **empty**.
* `nm -D --undefined-only target/release/libdriver.so` → only libc/libgcc
  runtime imports (`malloc`, `realloc`, `free`, `strtol`, `strcmp`, `fgets`,
  `printf`, `fprintf`, `fputc`, `stdin`, `stdout`, `stderr`, `memcpy`, …,
  `_Unwind_*`), i.e. **0 missing/undefined non-libc symbols**.
* `ldd` on the Rust `.so` resolves everything (`libgcc_s.so.1`, `libc.so.6`).

Latest run:

```
$ nm -D --defined-only c_src/build/libdriver_c_full.so | awk '{print $3}' | sort > c.txt
$ nm -D --defined-only target/release/libdriver.so | awk '$2=="T"{print $3}' | sort > r.txt
$ comm -23 c.txt r.txt        # C symbols missing from Rust
(no output)
$ wc -l < c.txt ; wc -l < r.txt
19
19
```

No symbol required stubbing, and no C translation unit was missing from the
Rust side: all six C files (`main.c`, `engine.c`, `a.c`, `b.c`, `util.c`,
`lib.c`) are translated in `src/lib.rs`.
