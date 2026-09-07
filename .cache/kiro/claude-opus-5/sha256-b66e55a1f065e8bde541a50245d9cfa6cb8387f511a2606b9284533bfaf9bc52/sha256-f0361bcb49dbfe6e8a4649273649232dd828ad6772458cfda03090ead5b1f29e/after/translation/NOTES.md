# Verification notes

## How to reproduce

```sh
cd translation && ./verify.sh
```

That builds the C `.so`, builds the Rust `cdylib` in both profiles for every
feature combination, checks symbol parity against each object, and runs both
differential test files. Symbol parity alone: `./check_symbols.sh`.

Both objects are loaded with `libloading` and driven only through their exported
`hsl_to_rgb` symbol (`tests/common/mod.rs`); the Rust function is never called
directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

## What was wrong, and what was changed

Two defects were found in `translation/src/lib.rs`. Nothing in `c_src/` was
touched.

### 1. `addss` operand order in hue arms 3-6 (`dest[2]`)

The C compiles every `X + m` in the cascade to

```
movss  X, %xmm0        ; X is the DESTINATION operand
addss  m, %xmm0        ; m is always the SOURCE operand
```

so when both `X` and `m` are `NaN` the result carries `X`'s payload and sign.
The Rust had written `dest[2]` as `add(m, x)` / `add(m, c)` in arms 3, 4, 5 and
6 — destination and source transposed. Corrected to `add(x, m)` / `add(c, m)`,
matching the four `movss -0x18/-0x10 ; addss -0x14` sequences at `0x1343`,
`0x139d`, `0x13f7` and `0x1451` in the reference object.

Observable whenever `x` and `m` are both `NaN` with different sign bits, which
`l = NaN, s = 1.0` produces: `fabsf` clears the sign on the path to `x` but not
on the path to `m`. Caught by `CONFIGS.md` rows 18, 23, 25, 26, 27 and 28-31.

### 2. Null-pointer behaviour under `-C debug-assertions`

`*src.add(0)` and `*dest.add(0) = v` carry an `assert_unsafe_precondition!`
null/alignment check when debug assertions are on. With a null pointer that
turns into a non-unwinding Rust panic and `SIGABRT` (6), whereas the
uninstrumented C faults with `SIGSEGV` (11). The release build stripped the
check and matched, so the divergence only appeared in the `debug` profile.

Replaced with `core::ptr::read_volatile` / `core::ptr::write_volatile`, which
carry no precondition assertion and fault in both profiles. This is also the
closer analogue of the C: one load per `src` element and one store per `dest`
lane, in source order, exactly as the three `movss` reads and three `movss`
writes in the reference object. `ERRORS.md` row 14.

## Fidelity decision: the C's `NaN` payloads are `-O`-level dependent

The reference object is what the task's build command produces:
`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON` with **no `CMAKE_BUILD_TYPE`**,
so `CMAKE_C_FLAGS` carries no `-O` flag and gcc compiles at `-O0`. The Rust is
matched to that object.

`NaN` payload propagation when *both* operands of an SSE instruction are `NaN`
is not fixed by IEEE-754, and gcc's choice changes with optimisation level.
Compiling the same unmodified `c_src/src/lib.c` at four levels and probing eight
both-`NaN` inputs (out-of-tree, `c_src/` unmodified) gives three distinct
behaviours:

| input (`h`,`s`,`l` bits) | C `-O0` = reference | C `-O1` | C `-O2`/`-O3` | Rust |
|---|---|---|---|---|
| `ce5c19a5 3f800000 fff58651` | `fff58651 7ff58651 7ff58651` | `fff58651 fff58651 fff58651` | `fff58651 7ff58651 fff58651` | matches `-O0` |
| `ff800000 ffda005a 6ca23871` | `ffda005a ffda005a 7fc00000` | `ffda005a ffda005a ffda005a` | `ffda005a ffda005a ffda005a` | matches `-O0` |
| `438dcc90 3f800000 ff8dbe4b` | `7fcdbe4b ffcdbe4b 7fcdbe4b` | `ffcdbe4b ffcdbe4b ffcdbe4b` | `7fcdbe4b ffcdbe4b ffcdbe4b` | matches `-O0` |

No single implementation can match all three, so matching the object the task's
build command actually produces is the only well-defined target. Every other
input class — all finite values, both zeros, subnormals, infinities, and any
case where at most one operand of an instruction is `NaN` — is `-O`-independent
and matches at every level.

## Faithfully preserved C bugs

* Arm 3's predicate is `h < 120.0f && h < 180.0f`, not `h >= 120.0f`. Arms 1-2
  already consume `[0,120)`, so arm 3 is reachable only for `h < 0`, and hues in
  `[120,180)` fall through to the final `else` and come out as the flat grey
  `(m, m, m)`. `ERRORS.md` rows 3 and 7; `CONFIGS.md` rows 5 and 10.
* `h == 360.0f` is rejected (upper-exclusive) while `180/240/300` are accepted
  (lower-inclusive). `ERRORS.md` rows 10 and 11.
* No clamping of `s` or `l`, so `l > 1` yields `c < 0` and out-of-`[0,1]`
  output. `ERRORS.md` row 13.
* `s == -0.0f` takes the achromatic early return, because `-0.0 == 0` in C.
  `ERRORS.md` row 2.

## Suite has teeth (mutation check)

Two deliberate mutations were injected and the suite caught both, then the file
was restored and re-verified byte-identical:

| mutation | result |
|---|---|
| `dest[2]` in arm 3 back to `add(m, x)` (a `NaN`-payload-only difference) | 21 of 32 Phase B rows fail, 11 of 20 Phase C tests fail |
| arm 3's predicate "fixed" to `h >= 120.0f32` | same run; rows 5 and 10 among the failures |

## Scope facts

* One public symbol, `hsl_to_rgb`; symbol diff against the C `.so` is empty.
* No `[features]` in `Cargo.toml`, so `default` and `--no-default-features` are
  the same build; both are exercised anyway, in both profiles.
* No executable driver in either project (`add_library(... SHARED ...)` only;
  `crate-type = ["cdylib"]` only), so there is no stdout to compare.
* No `enum` and no length/count parameter anywhere in the API, so the
  invalid-enum and bad-length boundary classes do not exist here; documented as
  negative results in `phase_c_errors.rs` (`err18`, `err19`) with assertions that
  will fail if `lib.h` ever grows an `enum`.
