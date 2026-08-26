# SYMBOLS.md — exported-surface parity (Phase A / Phase D)

## How this was produced

```sh
# ground truth: the C translation unit compiled as a shared object
gcc -shared -fPIC -O0 -o libc_driver_O0.so c_src/src/main.c
gcc -shared -fPIC -O2 -o libc_driver_O2.so c_src/src/main.c   # same symbols
nm -D --defined-only libc_driver_O2.so

# translation
cargo build --release          # produces target/release/libdriver.so (cdylib)
nm -D --defined-only target/release/libdriver.so
```

`c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver src/main.c)`),
so the "public API" of this project is `main(argc, argv)` plus the two file-scope
objects the translation unit gives external linkage to. Compiling the same
translation unit with `-shared -fPIC` exposes exactly that surface, which is what
the differential tests dlopen.

`tests/symbols.rs` re-runs this comparison automatically (`nm -D`), so the parity
claim below is machine-checked on every `cargo test`, not just recorded here.

## C `.so` dynamic symbols

Defined (`nm -D --defined-only`), excluding the linker-generated
`_init`/`_fini`/`__bss_start`/`_edata`/`_end` and weak toolchain hooks
(`_ITM_*`, `__gmon_start__`, `__cxa_finalize`):

| # | C symbol | type | size | Rust `.so` exports it? | Rust definition |
|---|----------|------|------|------------------------|-----------------|
| 1 | `array` | `B` (bss object) | `0x100000` (1 048 576 B = 262 144 × `int`) | ✅ `B array`, size `0x100000` | `src/program.rs` — `#[no_mangle] pub static mut array: [i32; ARRAY_SIZE]` |
| 2 | `main` | `T` (function) | — | ✅ `T main` | `src/lib.rs` — `#[no_mangle] pub unsafe extern "C" fn main(c_int, *const *const c_char) -> c_int` |
| 3 | `perform_expensive_operations` | `T` (function) | — | ✅ `T perform_expensive_operations` | `src/program.rs` — `#[no_mangle] pub extern "C" fn perform_expensive_operations()` |

**Missing from the Rust `.so`: none.** No symbol needed stubbing: `array` and
`perform_expensive_operations` are the real translated objects (shared verbatim
with the `driver` binary through `src/program.rs`), and `main` is the real
translated control flow (`src/lib.rs` splits the `argc != 2` / `argc == 2`
branches so it dereferences exactly the same `argv` slots the C does).

### Undefined (imported) symbols in the C `.so`

These are libc imports, not part of the surface to re-export. The Rust port
reimplements the behaviour of the two non-trivial ones instead of calling them,
which is why they are differentially tested against real glibc
(`tests/rng.rs`, `tests/parse.rs`):

| C import | Rust replacement | tested against glibc in |
|----------|------------------|-------------------------|
| `srand`, `rand` | `src/rng.rs` (`GlibcRand`: TYPE_3 additive feedback, deg 31 / sep 3, Schrage seeding, 310 discarded outputs) | `tests/rng.rs` |
| `strtoul` | `src/strtoul.rs` (`strtoul_base10`) | `tests/parse.rs` |
| `printf`, `fprintf`, `stderr` | `std::io::{stdout, stderr}` byte writes | `tests/errors.rs`, `tests/pipeline.rs` |
| `__errno_location` | `StrtoulResult::erange` | `tests/parse.rs` |

## Extra symbols exported by the Rust `.so` (not in C)

Parity requires C ⊆ Rust; these are the additional test hooks. They exist
because the C program's RNG and seed-validation stages are only reachable
through `main`, and `main` runs 5.2 × 10¹⁰ arithmetic operations
(`ITERATIONS × ARRAY_SIZE × 100`) before it produces observable output —
roughly 5 minutes per call. The hooks call **the same** functions the program
uses (`rng::GlibcRand`, `program::parse_seed`), so they cannot mask a
divergence in program behaviour; they only make the stages observable in
milliseconds.

| Rust symbol | purpose |
|-------------|---------|
| `harness_srand` / `harness_rand` | drive `rng::GlibcRand` (the port of `srand`/`rand`) for the differential RNG test |
| `harness_parse_seed` | expose the accept/reject decision + resulting `unsigned int seed` of `program::parse_seed` |
| `harness_array_size` / `harness_iterations` | expose the compiled-in `ARRAY_SIZE` / `ITERATIONS` so the tests can assert they equal the C `#define`s |

## Verified

```
$ nm -D -S --defined-only $OUT_DIR/libc_driver_O2.so | grep -v ' [Vw] '
0000000000004060 0000000000100000 B array
00000000000010b0 00000000000000f0 T main
0000000000001260 0000000000000073 T perform_expensive_operations

$ nm -D -S --defined-only target/release/libdriver.so | grep -v ' [Vw] '
0000000000052a68 0000000000100000 B array          <-- same 1 MB footprint
0000000000012eb0 0000000000000006 T harness_array_size
0000000000012ec0 0000000000000006 T harness_iterations
0000000000012ed0 000000000000012d T harness_parse_seed
0000000000013000 000000000000008c T harness_rand
0000000000013090 0000000000000063 T harness_srand
0000000000013100 00000000000004bc T main
00000000000135c0 000000000000007e T perform_expensive_operations

$ comm -23 <(nm -D --defined-only libc_driver_O2.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only libdriver.so     | awk '{print $NF}' | sort -u)
        (no output)
```

* C symbols missing from the Rust `.so`: **0** (for the `-O0` build too)
* Symbol kinds agree (`B` object vs `T` function) and `array` has an identical
  size of `0x100000` bytes in both.
* Undefined non-libc symbols in the Rust `.so`: **0** — `nm -D -u` lists only
  `@GLIBC_*` / `@GCC_*` versioned imports plus the usual weak toolchain hooks.

All four statements are re-checked mechanically by `tests/symbols.rs`
(`every_c_symbol_is_exported_by_rust`, `rust_so_has_no_unresolved_non_libc_symbols`,
`array_symbol_size_matches`, `compile_time_constants_match`,
`c_exports_no_symbol_the_suite_forgot_to_exercise`).

## The binary target

`c_src/CMakeLists.txt` builds an executable, and so does this crate
(`[[bin]] driver`). `build.rs` additionally compiles the C source into
`c_driver_O0` / `c_driver_O2` executables, and `tests/binary_cli.rs` compares the
two *programs* (exit status, stdout, stderr, `argv[0]` handling, signal
disposition) — the part of the surface that dlopen'ing a `.so` cannot reach.
