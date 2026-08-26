# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

The C project (`c_src/CMakeLists.txt`) builds an **executable** (`add_executable(driver src/main.c)`).
For symbol-level differential testing the same single translation unit is also
built as a shared object, exactly as the task prescribes:

```
gcc -shared -fPIC -o build_c/libdriver_c.so c_src/src/main.c
```

The Rust side is built as a `cdylib` from the same sources that produce the
`driver` binary (`src/lib.rs` + `src/driver_impl.rs`):

```
cargo build                     # -> target/debug/libdriver.so  (+ target/debug/driver)
cargo build --release           # -> target/release/libdriver.so (panic = "abort")

# exactly what tests/common/mod.rs builds and dlopens (RUST_SO_PROFILE=dev|release):
rustc --edition 2021 --crate-type cdylib --crate-name driver -A warnings \
      -C opt-level=0 -C debug-assertions=on  -C overflow-checks=on  \
      -o libdriver_rs_dev.so     src/lib.rs
rustc --edition 2021 --crate-type cdylib --crate-name driver -A warnings \
      -C opt-level=3 -C debug-assertions=off -C overflow-checks=off -C panic=abort \
      -o libdriver_rs_release.so src/lib.rs
```

## `nm -D --defined-only` — C shared object

| symbol | type | C declaration | exported by Rust `.so`? |
|--------|------|---------------|-------------------------|
| `driver` | `T` (global text) | `void driver(int x)` | YES — `#[no_mangle] pub extern "C" fn driver(x: c_int)` in `src/lib.rs` |
| `main`   | `T` (global text) | `int main(void)` | YES — `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

That is the complete list; the C `.so` defines exactly two global symbols.

### Deliberately **not** exported

| C entity | why not exported |
|----------|------------------|
| `static void print_hex(unsigned char *p, int len)` | `static` ⇒ internal linkage, absent from `nm -D` of the C `.so`. Translated as the private `imp::print_hex` and reached through `driver`. |

## Undefined symbols

`nm -D --undefined-only` on the C `.so`:
`__isoc99_scanf`, `printf`, `putchar` (glibc) plus the usual weak
`_ITM_*` / `__gmon_start__` / `__cxa_finalize` toolchain stubs.

On the Rust `.so`: glibc (`read`, `write`, `writev`, `malloc`, `memcpy`, …),
the `pthread_*`/`__tls_get_addr` TLS helpers and the `_Unwind_*` family from
`libgcc_s.so.1`. All are satisfied by `DT_NEEDED` entries
(`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`), i.e. **zero
missing/undefined non-libc(-runtime) symbols**.

## Result

```
$ nm -D --defined-only build_c/libdriver_c.so | awk '{print $3}' | sort   >  c.txt
$ nm -D --defined-only libdriver_rs.so        | awk '{print $3}' | sort   >  rs.txt
$ diff c.txt rs.txt      # empty
```

### Enforcing tests (`tests/symbols.rs`)

| test | what it enforces |
|------|------------------|
| `c_and_rust_export_identical_symbol_sets` | the two `nm -D --defined-only` sets are equal, and equal to `{driver, main}` |
| `c_symbols_are_static_where_the_source_says_static` | neither object exports `print_hex` |
| `rust_so_has_no_unresolved_non_runtime_symbols` | every undefined symbol in the Rust `.so` is libc/libgcc/TLS runtime (the same classifier is applied to the C `.so`, so it cannot quietly accept everything) |
| `cargo_built_shared_objects_match_too` | the artifact **cargo** produces (`target/{debug,release}/libdriver.so`) exports the same set; if neither exists it builds one into its own target directory rather than passing vacuously |

`tests/smoke.rs` additionally checks the harness itself: both objects `dlopen`,
both symbols resolve and are callable, and every capture mechanism (regular file,
pipe, forked child, real executables) agrees
(`both_shared_objects_load_and_export_driver_and_main`,
`capture_mechanisms_agree`, `forked_main_runner_works`,
`executables_agree_on_a_simple_input`).

Symbol diff is **empty** — every symbol exported by the C `.so` is exported by
the Rust `.so` under the exact same name, and the Rust `.so` exports nothing
extra. Enforced automatically by `tests/symbols.rs`
(`c_and_rust_export_identical_symbol_sets`), which re-runs `nm -D` on both
objects and also checks the cargo-built `target/{debug,release}/libdriver.so`
when present.
