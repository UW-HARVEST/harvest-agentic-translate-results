# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --lib --target-dir target/ffi-cdylib
nm -D --defined-only c_src/build/libharvest-work-tVEmXb.so
nm -D --defined-only translation/target/ffi-cdylib/debug/libarity_lib.so
```

`tests/symbols.rs` performs exactly this diff as an assertion, so the parity is
re-checked on every `cargo test` rather than being a one-off observation.

> **Build gotcha that silently invalidates this whole comparison.**
> `cargo test` does **not** build a `crate-type = ["cdylib"]` artifact — the
> integration tests do not link against the library, so Cargo has no reason to
> produce the `.so`. A test harness that merely *looks* for
> `target/<profile>/libarity_lib.so` therefore loads a **stale** `.so` left over
> from some earlier `cargo build`, or silently falls back to the other profile's
> copy, and every differential assertion then passes against old code. This was
> observed here: the first full green run was verifying a `.so` built before the
> test suite existed. `tests/common/mod.rs` now *builds* the `cdylib` itself
> (into `target/ffi-cdylib/`, a separate target dir so it cannot contend with the
> outer `cargo test`) and asserts the artifact is newer than `src/lib.rs`.
> `scripts/mutation_check.sh` is the regression test for this: with the stale
> path lookup, 0 of 23 injected bugs were detected; with the on-demand build,
> 23 of 23 are.

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | C signature (definition site) | Rust export |
|---|--------|-----------|---------------|-------------------------------|-------------|
| 1 | `shift_array`        | T | T | `void shift_array(int *arr, int size, int positions)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn shift_array(*mut c_int, c_int, c_int)` |
| 2 | `process_string`     | T | T | `int process_string(const char *str)` | `... fn process_string(*const c_char) -> c_int` |
| 3 | `apply_bitmask`      | T | T | `int apply_bitmask(int value, int operation)` | `... fn apply_bitmask(c_int, c_int) -> c_int` |
| 4 | `init_matrix`        | T | T | `void init_matrix(int matrix[3][4])` (decays to `int (*)[4]`) | `... fn init_matrix(*mut [c_int; 4])` |
| 5 | `compare_allocations`| T | T | `int compare_allocations(int val1, int val2)` | `... fn compare_allocations(c_int, c_int) -> c_int` |
| 6 | `arity4`             | T | T | `int arity4(int, int, int, int)` | `... fn arity4(c_int, c_int, c_int, c_int) -> c_int` |
| 7 | `arity2`             | T | T | `int arity2(int p1, int p2)` | `... fn arity2(c_int, c_int) -> c_int` |
| 8 | `arity3`             | T | T | `int arity3(int p1, int p2, int p3)` | `... fn arity3(c_int, c_int, c_int) -> c_int` |
| 9 | `arity`              | T | T | `int arity(unsigned char len, int *params)` — note the header declares `int arity(int len, int *params)` | `... fn arity(c_int, *const c_int) -> c_int` (masks to 8 bits) |

**Missing from Rust: none.** No `#[no_mangle]` wrapper had to be added and no C
source was left untranslated.

The C library is built from a single translation unit (`c_src/src/lib.c`, the only
source listed in `c_src/CMakeLists.txt`), so the surface above is complete: no C
module was skipped by the translation and nothing had to be stubbed. Verified
independently:

```
$ nm -D --defined-only c_src/build/*.so | awk '{print $3}' | sort > /tmp/c.syms
$ nm -D --defined-only translation/target/ffi-cdylib/debug/libarity_lib.so \
      | awk 'NF==3{print $3}' | sort > /tmp/r.syms
$ comm -23 /tmp/c.syms /tmp/r.syms      # in C but not in Rust
                                        # (empty)
$ wc -l /tmp/c.syms /tmp/r.syms
9 /tmp/c.syms
9 /tmp/r.syms
```

### `arity` signature mismatch inside the C itself (verified, not assumed)

`c_src/include/lib.h` declares `int arity(int len, int *params)` while
`c_src/src/lib.c` *defines* `int arity(unsigned char len, int *params)`. The
compiled callee is the source of truth; `objdump -d` on the C `.so` shows it
narrowing the incoming register to one byte and doing **unsigned** byte compares:

```
1634: mov  %edi,%eax
163a: mov  %al,-0x4(%rbp)      # keep only the low 8 bits of the int argument
163d: cmpb $0x1,-0x4(%rbp)
1641: ja   ...                 # unsigned compare  => len is unsigned char
```

Consequences reproduced by the Rust (`len as u32 & 0xFF`, then `u8` compares) and
covered by tests: `arity(256, p) == arity(0, p) == -1`, `arity(258, p) ==
arity(2, p)`, and `arity(-1, p) == arity(255, p)` which takes the `arity4` path.

This narrowing is not an artifact of `-O0`: `scripts/check_c_optimization_levels.sh`
rebuilds the C at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` and re-runs the whole
differential suite (including `err_arity_int_truncation`, which asserts
`arity(256) == arity(0) == -1`) against each. All five agree with the Rust.

## Undefined (imported) symbols

C imports only `malloc`, `free`, `memmove`, `strlen` (plus weak CRT hooks).
The Rust `.so` imports those same four plus the standard Rust runtime set
(`_Unwind_*`, `dl_iterate_phdr`, `memcpy`, `mmap64`, …). Every one resolves in
libc / libgcc: **0 missing or unresolvable non-libc symbols.**

Verify with:

```sh
nm -D --undefined-only translation/target/release/libarity_lib.so
```

## Cargo features

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default one (an empty feature set). There is
therefore exactly one feature combination to verify; `scripts/check_features.sh`
enumerates the features from `Cargo.toml` and re-runs the suite for each
combination it finds, which for this crate is `--no-default-features` plus the
default build.
