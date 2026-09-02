# SYMBOLS.md — dynamic-symbol parity between the C and Rust shared objects

Derived mechanically from `nm -D` on both objects.

* C:    `c_src/build/libdriver.so`   (cmake, `CMAKE_BUILD_TYPE=""` → `-O0`, gcc 11.5.0)
* Rust: `translation/target/release/libdriver.so` (`cargo build --release`, `crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D c_src/build/libdriver.so            | awk '$2=="T"{print $3}' | sort > /tmp/c.sym
nm -D translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort > /tmp/r.sym
comm -23 /tmp/c.sym /tmp/r.sym   # must be empty
```

## Defined (`T`) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `printLine` | T | T | `void printLine(const char *)`; declared nowhere in `driver.h` but external linkage in `driver.c`, hence exported |
| 2 | `bad`       | T | T | `void bad(void)` |
| 3 | `good`      | T | T | `void good(void)` |
| 4 | `driver`    | T | T | `void driver(int)`; the only symbol declared in the public header |

**Missing from Rust: none.** `comm -23` output is empty.

## Deliberately absent (not a gap)

| C symbol | why it is not in `nm -D` of either object |
|----------|-------------------------------------------|
| `helperBad`    | `static` in `driver.c` → internal linkage. Private `fn helperBad()` in Rust. |
| `helperGood1`  | `static` in `driver.c` → internal linkage. Private `fn helperGood1()` in Rust. |

There is no untranslated C module: `c_src` contains exactly one translation unit
(`src/driver.c`, 68 lines) plus one header (`include/driver.h`), and all four
external-linkage functions in it are implemented and exported by the Rust crate.

## Undefined / weak entries (linker + libc artifacts, not API)

These are *not* required to match; they are toolchain-generated and differ
because the two objects use different libc entry points and different runtimes.

| symbol | C | Rust | comment |
|--------|---|------|---------|
| `puts@GLIBC_2.2.5` | U | – | gcc rewrites `printf("%s\n", x)` into `puts(x)` |
| `printf@GLIBC_2.2.5` | – | U | Rust calls `printf` directly; byte-identical output to `puts` |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__`, `__cxa_finalize` | w | w | present in both |
| `__cxa_thread_atexit_impl`, `gettid`, `statx` | – | w | pulled in by the Rust `std` runtime |

`printf` vs `puts` is the only intentional codegen difference; both write the
string followed by a single `\n` to the shared libc `stdout` FILE object, so the
observable byte stream is identical. This is asserted by the differential tests
rather than assumed.
