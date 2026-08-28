# Differential test harness (development aid, not part of the cdylib)

These files were used to validate the Rust translation against the original C
shared library.  They are plain C / Python and are ignored by Cargo.

    # build the C reference
    cmake -S ../../c_src -B /tmp/cbuild -DCMAKE_BUILD_TYPE=Release && cmake --build /tmp/cbuild
    # build the Rust cdylib
    (cd .. && cargo build --release)
    # generate cases and compare (dlopens both .so, forks per case, compares
    # return value, the whole output buffer and cp_error_reason)
    python3 gencases.py cases.bin
    gcc -O1 -o difftest difftest.c -ldl
    ./difftest /tmp/cbuild/lib*.so ../target/release/libtranslation.so cases.bin

Result at the time of writing: 2348 cases, the only divergences are inputs on
which the C library itself dies (SIGSEGV from its own out-of-bounds stack
writes in `cp_build` when a corrupt stream yields code lengths >= 16, or
SIGABRT when the C library is compiled with `assert()` enabled).  Every input
the C library survives produces byte-identical results.
