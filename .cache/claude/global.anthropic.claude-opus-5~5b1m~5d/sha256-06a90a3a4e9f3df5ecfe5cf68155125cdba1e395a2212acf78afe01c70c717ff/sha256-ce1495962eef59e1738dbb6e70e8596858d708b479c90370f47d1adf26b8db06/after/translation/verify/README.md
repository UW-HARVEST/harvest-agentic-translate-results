# Differential verification

`difftest.c` dlopens the C shared library built from `c_src/` and the Rust
`cdylib` from this crate, calls all 22 exported symbols with identical inputs
(random bit patterns, specials: +-0, denormals, +-inf, NaN, huge values) and
compares the raw bytes of every return value and every `out` structure.

`specfuzz.c` does the same for `spec_ray`, the only function declared in the
public header, with 4M NaN/inf-heavy cases.

    # build the C reference
    cmake -S ../../c_src -B /tmp/cref -DCMAKE_BUILD_TYPE=Release && cmake --build /tmp/cref
    # build the Rust library
    (cd .. && cargo build --release)
    # compare
    gcc -O2 -o /tmp/difftest difftest.c -ldl -lm
    /tmp/difftest /tmp/cref/lib*.so ../target/release/libspec_ray_lib.so
    gcc -O2 -o /tmp/specfuzz specfuzz.c -ldl -lm
    /tmp/specfuzz /tmp/cref/lib*.so ../target/release/libspec_ray_lib.so

Result: 0 mismatches for every non-NaN input, for both the default (`-O0`) and
the `Release` (`-O2`) C build.  See the module documentation in `src/lib.rs`
for the NaN-payload caveat.

These two C programs are the *original* verification scaffolding.  The
authoritative suite is now the Rust integration-test harness in `../tests/`,
which loads both `.so` files with `libloading` and is driven by `../verify.sh`;
see `../VERIFICATION.md`, `../SYMBOLS.md`, `../CONFIGS.md` and `../ERRORS.md`.
