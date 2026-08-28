# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Reproduce everything with:

```sh
cd translation && ./run_all.sh      # exits 0 only if every check passes
```

## Method

Both libraries are loaded as shared objects with `libloading` (`dlopen` with
`RTLD_LOCAL`, which matters because both export the same six names) and every
call goes through `dlsym`. **No Rust function is ever called directly**, so the
`#[no_mangle] extern "C"` export wrappers are themselves under test.

Three observable channels are diffed, not just return values:

1. the returned `int` / `char*` / `StringBuffer*`;
2. the `StringBuffer` heap state — `capacity`, `length`, and the `data` bytes
   (only bytes an implementation actually wrote are compared; the rest is
   uninitialised `malloc` memory and would be a false failure);
3. the **stdout byte stream** `buffapp` emits via `printf`, captured by
   redirecting fd 1 with `dup2`.

For the undefined-behaviour inputs the C accepts, the call is made in a forked
child and the **exact termination signal** is compared. "Both failed somehow" is
never accepted, and the near-miss inputs are asserted to exit 0 so a test cannot
pass by having everything crash.

## Gate

- [x] **`SYMBOLS.md`** — `nm -D` diff is EMPTY. All 6 symbols the C `.so`
      defines (`create_buffer`, `append_to_buffer`, `destroy_buffer`,
      `get_operation_name`, `perform_operation`, `buffapp`) are defined by the
      Rust `.so` under the exact same names; 0 unresolved non-libc imports.
      `src/lib.c` is the only C translation unit, and no symbol is stubbed —
      `grep -E 'unimplemented!|todo!'` over `src/lib.rs` is empty.
      Enforced by `check_symbols.sh` and `tests/phase_d_symbols.rs`.
- [x] **Phase B** — all **45** rows of `CONFIGS.md` pass, 0 unchecked, driven by
      the fixed-seed (`0x5EED_1234_ABCD_EF01`) xorshift64\* PRNG with thousands
      of draws per row plus the named boundary values. The low-level entry points
      are driven directly, not only through the `buffapp` wrapper.
- [x] **Phase C** — all **24** rows of `ERRORS.md` have a passing error-path
      differential test, 0 unchecked, plus the generic boundaries (NULL
      pointers, zero and oversized lengths, out-of-range enum ints one step past
      the valid range, capacity −1/0/1).
- [x] **All of the above under every configuration.** `Cargo.toml` declares no
      `[features]`, so the only build configuration is the default; both
      `cargo test` and `cargo test --no-default-features` are run. The suite is
      additionally run against the **release** and the **debug** cdylib, and
      against the C compiled at **-O0, -O1, -O2, -O3 and -Os**.

**76 tests, all passing** (25 + 16 + 5 Phase B, 26 Phase C, 4 Phase D)
× 2 feature combos × 2 Rust profiles × 5 C optimisation levels.

## Divergences found and fixed

Verification changed only `translation/`; `c_src/` was never modified.

1. **The debug cdylib aborted where the C segfaults.** `append_to_buffer(NULL, …)`
   killed the process with `SIGABRT` (6) from the Rust `.so` but `SIGSEGV` (11)
   from the C. Cause: Rust's `debug-assertions` insert a null-pointer-dereference
   check, whose panic cannot escape an `extern "C"` boundary and so aborts. Since
   this crate must reproduce the C's behaviour on the very inputs where the C has
   UB, `[profile.dev]` now sets `debug-assertions = false` and
   `overflow-checks = false`, matching the C build. The release cdylib was
   already correct. Re-verified: debug and release now behave identically.

## Notable findings about the C that the tests pin down

* `buffapp`'s final `result / intermediate3` **can** be `INT_MIN / -1`. It is
  reachable at `buffapp(0, 1073741823, 0, 1073741825)`: both halves take the
  `add` arm, so `result = i1 + i2 = INT_MIN` and `intermediate3 = i1 * i2 = -1`
  after wrapping. gcc emits a bare `idiv`, so the CPU raises `#DE` and the
  process dies with `SIGFPE`. The Rust translation reproduces this with inline
  `cdq`/`idiv` rather than Rust's `/` (which would panic). `ERRORS.md` #24 /
  `err24_buffapp_final_division_traps_identically` asserts both die with signal 8.
* `param1 % 4` uses C's truncating `%`, so **negative** parameters yield negative
  residues, fall through `get_operation_name`'s `default:` to `"unknown"`, and
  make `perform_operation` return 0. All 7×7 residue classes are covered.
  `INT_MIN % 4 == 0`, so `INT_MIN` takes the `"add"` arm, not the `"unknown"` one.
* `create_buffer(negative)` sign-extends to a huge `size_t` and returns `NULL`;
  `create_buffer(0)` **succeeds** on glibc and then writes one byte past a
  0-byte block.
* `append_to_buffer`'s `new_capacity = required_capacity * 2` overflows `int` to
  a negative value for large `length`, which sign-extends to a huge `size_t` and
  makes `realloc` fail → `-1`. If `required_capacity` *itself* wraps negative,
  the grow test is false, no reallocation happens, and `strcpy` writes at
  `data + INT_MAX`.
* Both libraries use the **same libc allocator**, verified directly: a buffer
  created by C can be grown and freed by Rust and vice versa
  (`row44_cross_library_buffer_handoff`).

## Notes on the environment

* `.cargo/config.toml` sets `offline = true` (the crates.io index is unreachable
  here; `libloading` and `libc` come from the local registry cache) and
  `RUST_TEST_THREADS = 1` (the stdout capture redirects the process-wide fd 1, so
  concurrent test threads would interleave libtest's own progress lines into the
  captured bytes).
* The C `.so` file name is derived from the parent directory by CMake
  (`libharvest-work-uATrTJ.so` here), so the harness globs for `c_src/build/lib*.so`
  instead of hard-coding it. Override with `C_SO=` / `RUST_SO=`.
* `tests/phase_c_errors.rs::err08` needs a 2 GiB `MAP_NORESERVE` reservation; if
  the kernel refuses it the test degrades to checking the branch decision and
  reports that on stderr rather than silently passing.
