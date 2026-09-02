# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C   : `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C `.so` exported (defined, dynamic) symbols

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001119 T UTIL_createLinePointers
```

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `UTIL_createLinePointers` | `T` (global text) | YES (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

There are no macro-generated, versioned, weak, or data (`D`/`B`/`R`) exports in
the C `.so`: the whole public surface of `c_src/include/lib.h` is that one
function. `c_src/src/lib.c` is the only translation unit in
`c_src/CMakeLists.txt` (`add_library(driver SHARED src/lib.c)`), so no C module
was skipped by the translation — nothing needed to be newly translated for
Phase A/D.

## Rust `.so` exported (defined, dynamic) symbols

```
$ nm -D --defined-only translation/target/release/libdriver.so
00000000000116c0 T UTIL_createLinePointers
```

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
<empty>
```

**Missing symbols in Rust: 0.** Extra symbols in Rust: 0.

## Undefined (imported) symbols

C `.so` imports: `malloc`, `free` (glibc) plus the usual weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` toolchain stubs.

Rust `.so` imports the same `malloc`/`free`, plus libc/`libgcc` symbols pulled
in by the Rust runtime (`_Unwind_*`, `memcpy`, `mmap64`, `dl_iterate_phdr`,
`pthread_key_*`, …). All of these resolve out of `libc`/`libgcc_s`, i.e.

**0 missing/undefined *non-libc* symbols in the Rust `.so`.**

Verified with:

```sh
nm -D --undefined-only translation/target/release/libdriver.so
ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Phase D re-verification under every feature combination

`Cargo.toml` declares **no `[features]` table**, confirmed mechanically:

```
$ grep -c '^\[features\]' translation/Cargo.toml
0
$ cargo metadata --no-deps --format-version 1 | jq '[.packages[].features]'
[{}]
```

So the feature power set has exactly one member (the default, which is empty).
`translation/phase_d_features.sh` extracts the feature list from `Cargo.toml`
rather than hard-coding it, builds the `cdylib` under each combination, runs the
full differential suite against **that** `.so`, and re-checks the `nm -D` diff
per combination. Current output:

```
declared features : <none declared>
combinations      : 1
PASS  [<default>]  test result: ok. 41 passed; 0 failed; ...
      symbols [<default>] parity OK (0 missing)
ALL COMBINATIONS PASS
```

Because there is no feature axis, the profile axis was swept instead — the same
suite was run against the release `.so` and against the **debug** `.so`
(overflow checks on, which would trap any `+`/`*` the C performs modulo 2^64):
both 41/41. And against the C compiled at `-O0`, `-O1`, `-O2`, `-O3` and
`-O2 -fno-strict-aliasing`: 39/39 each (the two interposer tests skip when
`LD_PRELOAD` is not set).
