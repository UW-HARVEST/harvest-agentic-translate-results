# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on the built C shared library, not from
assumptions about which functions look "important".

## How the lists were produced

```sh
# C side
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only libtranslated_rust.so

# Rust side
cargo build --offline --release
nm -D --defined-only target/release/libto_barycentric_lib.so
```

Note on the C `.so` name: `c_src/CMakeLists.txt` derives `project()` from the
*parent* directory name (`cmake_path(GET parent FILENAME project_name)`), so the
library is `libtranslated_rust.so`, not `libc_src.so`.

## C exports (`nm -D --defined-only`, C `.so`)

| addr | type | symbol |
|------|------|--------|
| `00000000000011a3` | `T` | `to_barycentric` |

That is the complete list — **1** exported symbol. The three helpers in
`c_src/src/lib.c` (`lm_v2`, `lm_sub2`, `lm_dot2`) are declared `static`, so they
have local (`t`) binding and are deliberately absent from the dynamic symbol
table. They must NOT be exported from Rust either.

```
$ nm libtranslated_rust.so | grep -E 'lm_(v2|sub2|dot2)'
00000000000010f9 t lm_v2
0000000000001126 t lm_sub2
0000000000001173 t lm_dot2
```

## Rust exports (`nm -D --defined-only`, Rust `.so`, non-`U`/`w`/`b`)

| addr | type | symbol |
|------|------|--------|
| `0000000000011c40` | `T` | `to_barycentric` |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only target/release/libto_barycentric_lib.so | awk '$2=="T"{print $3}' | sort)
(no output)
```

* Symbols in C but missing from Rust: **0**
* Undefined (`U`) non-libc symbols in the Rust `.so`: **0**
  (the Rust `.so`'s `U` entries are only libc/`libgcc` runtime imports such as
  `memcpy`, `__cxa_thread_atexit_impl`, `_Unwind_Resume`, `dl_iterate_phdr`,
  `pthread_*`, `getenv`, `abort`, `write` — no unresolved project symbols)
* Extra Rust exports beyond the C ABI: none of interest — the Rust `.so`
  additionally exposes the usual `rust_eh_personality`-class weak/`__rust_*`
  runtime symbols that every Rust cdylib carries, plus standard ELF
  `_init`/`_fini`/`__bss_start`/`_edata`/`_end`. These are runtime scaffolding,
  not API, and do not shadow or conflict with any C symbol.

## Completeness check

The C build (`add_library(... SHARED src/lib.c)`) compiles exactly one
translation unit, `c_src/src/lib.c` (29 lines), whose only non-`static`
definition is `to_barycentric`. `c_src/include/lib.h` declares exactly one
function and one type:

```c
typedef struct lm_vec2 { float x, y; } lm_vec2;
lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
```

No C source file in `c_src/` is untranslated; there is no module that was
skipped. `src/lib.rs` translates the whole translation unit, including the three
`static` helpers (kept private, matching their C linkage).

**Status: PASS — symbol diff is empty.**

## ABI notes verified alongside the symbol check

* `lm_vec2` is `repr(C)` with two `f32` → 8 bytes, SysV AMD64 class SSE, so it
  is passed and returned in the low half of one XMM register. `p1..p` therefore
  arrive in `xmm0..xmm3` and the result leaves in `xmm0`, in both libraries
  (confirmed by `objdump -d` on both `.so` files).
* Both libraries' `to_barycentric` were disassembled and compared
  instruction-for-instruction for x86 SSE *destination-operand* choice, which is
  what selects the surviving NaN when both operands are NaN. All **9 `subss`, 18 `mulss`,
  5 `addss` and 1 `divss`** pick the same destination operand in the Rust
  release build as in the reference C build (counts verified with `objdump`:
  the C spends 2 `subss` per `lm_sub2` x 3 calls + 3 at top level, and
  2 `mulss` + 1 `addss` per `lm_dot2` x 5 calls + 8 `mulss` at top level).
  See `CONFIGS.md` rows C21-C27d.

## Phase D re-verification (automated)

`./verify_all.sh` re-derives both symbol tables on every run and fails if the
diff is non-empty, for **both** the debug and release Rust `.so`:

```
== Phase D: symbol parity (nm -D) ==
  PASS no C symbol missing from debug/libto_barycentric_lib.so
  PASS no C symbol missing from release/libto_barycentric_lib.so
  PASS no unresolved non-libc symbols in the Rust .so
```
