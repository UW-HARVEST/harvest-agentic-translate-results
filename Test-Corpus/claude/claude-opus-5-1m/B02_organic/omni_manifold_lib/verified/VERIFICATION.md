# VERIFICATION.md — results

Differential verification of the Rust translation in `src/` against the C ground
truth in `c_src/`. Both are built as shared objects, both are loaded with
`libloading`, and **every** call on both sides goes through `dlsym` — the Rust
functions are never called directly, so the `#[no_mangle] extern "C"` wrappers and the
SysV struct-passing ABI are part of what is tested.

## How to reproduce

```sh
# 1. build the C reference (gcc, -O0 -- CMakeLists.txt sets no CMAKE_BUILD_TYPE)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# 2. build the Rust cdylib and run everything
cargo build && cargo test
cargo build --release && cargo test --release

# or: enumerate every Cargo feature combination and run the suite under each
./run_all_features.sh
```

## Completion gate

| gate | status |
|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust | **PASS** — 46 / 46, diff empty |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | **PASS** — 79 / 79 |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | **PASS** — 96 / 96 |
| All of the above under every feature combination | **PASS** — the crate declares no `[features]`, so there is exactly 1 combination; additionally verified under both the `dev` and the `release` profile |

```
105 tests across 10 test binaries; 3 consecutive full debug runs and
2 full release runs, all green.
```

| test binary | tests | covers |
|---|---|---|
| `phase_b_primitives` | 16 | CONFIGS rows 1–20 |
| `phase_b_shapes`     | 9  | CONFIGS rows 21–29 |
| `phase_b_simplex`    | 7  | CONFIGS rows 30–38 |
| `phase_b_gjk`        | 14 | CONFIGS rows 39–56 |
| `phase_b_manifolds`  | 8  | CONFIGS rows 57–72 |
| `phase_b_api`        | 3  | CONFIGS rows 73–79 |
| `phase_c_errors`     | 34 | ERRORS rows 1–95 |
| `phase_d_symbols`    | 4  | symbol parity, undefined-symbol audit, ERRORS row 27 |
| `probe_uninit`       | 5 (+1 child) | characterises the C library's one piece of genuine UB |
| lib unit tests       | 4  | `src/fp.rs` NaN-propagation helpers |

Outputs are compared as **raw bytes** (`memcmp` on the object representation), so
`+0.0` vs `-0.0` and differing NaN payloads are divergences, not equalities. Output
structs are pre-filled with a poison pattern, so "the C code leaves this field
untouched on the reject path" is itself verified.

## Bugs found and fixed in the Rust translation

All 20 were found by the differential tests and all are in the same class: the
previous translation had *guessed* which SSE operand GCC uses as the `addss`/`mulss`
**destination**, and guessed from an `-O2`-style mental model. `c_src/CMakeLists.txt`
sets no `CMAKE_BUILD_TYPE`, so the reference is compiled at **`-O0`**, where the
destination falls out of the load order rather than following the source order. Since
the destination NaN wins when both operands are NaN, each wrong guess is a real,
reachable difference in the returned NaN payload.

The fix was to disassemble `c_src/build/libtranslated_rust.so` and read the operand
order off all 75 `addss`/`mulss` sites. Each corrected site now quotes the
instructions it came from.

| # | function | was | is (from `objdump -d`) |
|---|---|---|---|
| 1 | `c2Mulvs` | `mul(b, a.x)` — assumed a vectorised broadcast | `mul(a.x, b)` |
| 2 | `c2Add` | `add(a.x, b.x)` | `add(b.x, a.x)` |
| 3 | `c2Dot` | `add(mul(a.x,b.x), mul(a.y,b.y))` | `add(mul(b.y,a.y), mul(a.x,b.x))` |
| 4 | `c2Det2` | `mul(a.x,b.y) - mul(a.y,b.x)` | `mul(b.y,a.x) - mul(b.x,a.y)` |
| 5 | `c2Mulrv` | 4 muls + 1 add, all guessed | x: `mul(b.x,a.c) - mul(b.y,a.s)`, y: `add(mul(a.s,b.x), mul(b.y,a.c))` |
| 6 | `c2MulrvT` | y lane folded into a `subss` | GCC keeps the `xorps` negate: `add(mul(-a.s,b.x), mul(b.y,a.c))` |
| 7 | `c23` edge-BC | `add(vBC, uBC)` | `add(uBC, vBC)` |
| 8 | `c23` interior | `add(wABC, add(uABC,vABC))` | `add(add(uABC,vABC), wABC)` |
| 9–14 | `c2Witness` ×6 | `mul(den, u)` | `mul(u, den)` |
| 15 | `c2L` | `mul(den, b.u)` | `mul(b.u, den)` |
| 16–18 | `c2CircletoCircleManifold`, `c2CircletoCapsuleManifold`, `c2CapsuletoCapsuleManifold` | `add(A.r, B.r)` | `add(B.r, A.r)` |
| 19 | `c2AABBtoAABBManifold` | `add(eB.x, eA.x)`, `add(eB.y, eA.y)` | `add(eA.x, eB.x)`, `add(eA.y, eB.y)` |
| 20 | `c2CapsuletoPolyManifold` | `add(depths[i], A.r)` | `add(A.r, depths[i])` |
| 21 | `c2Clip` | plain `d0 * d1` | `fp::mul(d0, d1)` — pins `d0` as the destination |

And one non-floating-point bug, of a completely different kind:

| # | function | problem | fix |
|---|---|---|---|
| 22 | `c2AABBtoCapsuleManifold` | `c2CapsuletoPolyManifold` reads `B->verts[-1]` when every face distance is NaN (a degenerate AABB makes `c2Norms` emit NaN normals, so `index` keeps its `~0` initialiser). Here the `c2Poly` is a *local of the C function*, so that read lands inside GCC's own frame — where `A` sits immediately before `p`, making `verts[-1] == (A.max.y, p.count)`. The Rust locals were laid out differently, so the read returned different bytes and the whole manifold diverged. | The two locals are now one `#[repr(C)] struct AABBtoCapsuleFrame { a_local: c2AABB, p: c2Poly }`, pinning GCC's adjacency so the out-of-bounds read observes identical bytes. |

## Where the C library is not a function of its inputs

`c2GJK` declares `c2Proxy pA, pB;` without an initialiser, and `c2MakeProxy` has no
`C2_TYPE_POLY` case and no `default:`, so it writes nothing for a polygon.
**Every polygon path therefore reads uninitialised stack.** Measured:

* From a debug test harness, `pB.verts[0]` reads back the two halves of a stack
  address — e.g. `0x00007f89_3affe180` — so `c2GJK` returns a nonsense distance that
  moves with ASLR.
* From a minimal fresh-process C driver (`dlopen` + one `omni_manifold(AABB, CAPSULE)`
  call), and from a release-profile harness, the garbage `pB.count` makes `c2Support`
  walk off the end of the array and the process dies with **SIGSEGV, exit 139,
  reproducibly**.

Affected entry points: `c2GJK` with a POLY / out-of-range type,
`c2CapsuletoPolyManifold` and `c2AABBtoCapsuleManifold` (always), and `c2Collide` /
`omni_manifold` for the `(AABB, CAPSULE)` and `(CAPSULE, AABB)` pairs.

No translation can reproduce that, so `src/gjk.rs` zero-initialises both proxies. This
reproduces the "virgin, zero-filled stack page" case exactly — a POLY operand behaves
as a single point at the origin with radius 0 — which is deterministic and never
crashes. `tests/probe_uninit.rs` documents the UB, running the crashing call in a
**child process** so a SIGSEGV cannot take the suite down.

Crucially, this did **not** mean giving up on those paths. `common::zero_stack()`
zeroes 8 KiB of stack immediately below the caller's frame (~6× the deepest C call
chain) as the last statement before the FFI call, which forces the C side into the same
all-zero state. With that in place the polygon paths agree byte-for-byte and are fully
differentially tested — and they are the *only* route to the five `static` helpers
`c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident`.

## Remaining known divergence

One, and it is unobservable:

`ptr_from_parts(C2_TYPE_POLY, …)` — the C `switch` has no POLY case and no `default`,
so control falls off the end of a non-`void` function and GCC returns a stale
callee-saved register (observed: `0x7fd757fec39f`, a stack address). The Rust returns
`NULL`. This cannot be reproduced and cannot be observed: `c2Collide` has no POLY case
either, so the pointer is never dereferenced — which `ERRORS.md` rows 5–9 verify
byte-for-byte for all 16 type pairs plus 7 out-of-range enum values.

## Notes on coverage quality

* **Branch coverage is asserted, not hoped for.** Tests that depend on reaching a
  particular `switch`/`if` arm count the arms they hit and fail if any is missed.
  `c23`'s seven Voronoi regions are additionally each targeted by a hand-built
  simplex whose landing region is then verified.
* **Out-of-range enum values** (`4`, `5`, `255`, `256`, `0x7fffffff`, `0x80000000`,
  `0xffffffff`) are passed across the FFI boundary for every `C2_TYPE` parameter, since
  a C enum accepts any `int`.
* **Out-of-bounds reads the C makes are compared, not avoided.** `c2PlaneAt` with
  `i ∈ [-4, 11]` and `c2CapsuletoPolyManifold` with an empty polygon (`verts[-1]`) are
  driven with the `c2Poly` embedded in a `#[repr(C)]` struct with known padding, so
  both libraries read identical bytes at the same address.
* `ERRORS.md` row 27 (`while (iter < 20)`): `*iterations` is compared on every single
  `c2GJK` call, and a dedicated ~108 K-call search over all type pairs, transform
  combinations and hand-built caches produced the histogram
  `{0: 58101, 1: 33550, 2: 13455, 3: 2852, 4: 38, 5: 4}` — the cap is never reached, so
  the uninitialised `s.b.u` read it would enable is unreachable.
* `sqrtf` is the one libc call the C imports that the Rust does not: `f32::sqrt` lowers
  to the `sqrtss` instruction, and glibc's `sqrtf` does the same. Both are the IEEE-754
  exact square root, including the `0xFFC00000` indefinite for negative inputs, and
  this is covered by the `c2Len` / `c2Norm` / `c2Circleto*` differential tests.
