# Verification report — C ↔ Rust differential testing

The C in `c_src/` is the ground truth. Everything below was produced by loading
**both** shared objects with `libloading` and calling them only through their
exported C symbols (`dlsym`), so the `#[unsafe(no_mangle)] extern "C"` wrappers
are part of what is tested. No Rust function is ever called directly.

```
c_src/build/libharvest-work-61Wh7J.so     <- gcc 11.5, -fPIC, no -O (cmake default)
translation/target/release/libmaxnmin_lib.so
```

## How to reproduce

```sh
# 1. the C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. the Rust cdylib + the whole differential suite
cd translation
cargo build --offline --release
cargo test  --offline --release          # 85 tests

# 3. the phase-specific gates
./check_symbols.sh      # Phase D: nm -D diff must be empty
./check_features.sh     # Phase D: whole suite under every feature combination
./check_coverage.sh     # Phase B/C completeness: gcov of the C driven by the suite
./mutation_check.sh     # sanity: the suite must FAIL when the Rust is broken
```

Both `.so` paths can be overridden with `HARVEST_C_SO` / `HARVEST_RUST_SO`.

## Test inventory (85 tests)

| file | tests | what |
|---|---|---|
| `tests/common/mod.rs` | — | harness: dual `dlopen`, `Node` mirror, fixed-seed RNG, bit-exact comparators |
| `tests/smoke.rs` | 3 | harness self-check, pristine-state isolation, `Node` layout vs gcc |
| `tests/valid_paths.rs` | 41 | Phase B — one test per `CONFIGS.md` row (C1..C41) |
| `tests/error_paths.rs` | 36 | Phase C — one test per `ERRORS.md` row (E1..E38 minus E6/E21) |
| `tests/null_pointer.rs` | 2 (+1 ignored payload) | Phase C — E6/E21, out-of-process signal comparison |
| `tests/symbol_parity.rs` | 3 | Phase D — `nm -D` parity, no non-libc imports, `dlsym` of all 7 |

Each `Pair::fresh()` copies both `.so`s to unique temporary files before
`dlopen`, so every test case starts from **pristine** library state
(`node_count == 0`, `node_storage` zeroed) — which is the only way to reach the
`MAX_NODES` boundary and the empty-store branches, since the C exposes no reset.

## Divergences found and fixed

### 1. NaN payload/sign lost in `calculate_subtree_sum` (real bug)

`c_src/src/lib.c:92` — `sum += calculate_subtree_sum(node_storage[i].id);`

gcc emits the recursive result as the **destination** operand:

```asm
call   calculate_subtree_sum   ; xmm0 = child sum
movsd  -0x8(%rbp),%xmm1        ; xmm1 = sum
addsd  %xmm1,%xmm0             ; xmm0 = child + sum
```

LLVM emitted the operands the other way round (`addsd %xmm0,%xmm1`, dst = the
accumulator). `ADDSD` returns its *first* operand when both addends are NaN, so
whenever a subtree produced one NaN and the accumulator held another, the two
libraries returned different NaN bit patterns:

```
calculate_subtree_sum(1): C = 0xfff8000000000000, Rust = 0x7ff8000000000000
```

Fixed by pinning the operand order (`add_c_order`, inline `addsd` on x86-64) and
locked down with `CONFIGS.md` row C41 (all ordered pairs of 8 NaN patterns at
depth 3 + randomized fan-out).

### 2. Non-deterministic `Node` padding bytes

`find_node_by_id` returns a `Node *` into the static array, so a consumer can
read the 6 padding bytes at offsets 58..63. gcc zero-fills the whole 80-byte
object for `Node new_node = {.id = ..., ...}`; the Rust struct literal left
padding formally uninitialised, and the dev-profile build indeed wrote garbage
there. `add_node` now stages the node in a `MaybeUninit::<Node>::zeroed()` buffer
and copies all `size_of::<Node>()` bytes, so the full 80-byte image matches in
both profiles (row C40). The staging buffer also preserves the C's ordering — the
name is read *before* the destination slot is written, so a `name` argument that
aliases the destination slot behaves as in C.

### 3. Dev-profile null-dereference check (build configuration)

`add_node(.., NULL, ..)` and `process_string(NULL)` are unchecked dereferences in
the C and die with `SIGSEGV`. Rust's debug assertions turn them into a panic →
`SIGABRT`. `[profile.dev] debug-assertions = false` restores the C's failure
mode; the suite now passes with `HARVEST_RUST_SO` pointing at *either* the
release or the dev `cdylib`.

### 4. Smaller fidelity fixes

* `add_node`'s capacity test is a signed `int` comparison in C
  (`node_count >= MAX_NODES`); it was casting the counter to `usize`.
* the slot write uses signed `offset`, like C's indexing.

## Completeness evidence

* **Symbols** — `nm -D --defined-only`: C exports exactly 7 symbols
  (`add_node`, `find_node_by_id`, `get_children_count`,
  `calculate_subtree_sum`, `process_string`, `safe_double_to_int`, `maxnmin`);
  the Rust `.so` exports all 7 under the same names. Diff empty, 0 non-libc
  undefined symbols. Nothing stubbed, no untranslated module.
* **C coverage driven by the suite** (`./check_coverage.sh`):
  lines 100.00 % (75/75), branches executed 100.00 % (38/38), branch directions
  taken 97.37 % (37/38), calls 100.00 % (16/16). The single untaken direction is
  `lib.c:145 if (*name_ptr)` FALSE, which is unreachable by construction
  (`maxnmin` re-seeds six builtins whose names are all non-empty) and is covered
  as far as the API allows by `ERRORS.md` row E29.
* **Mutation sensitivity** (`./mutation_check.sh`): 10 deliberate breakages of
  the Rust (capacity off-by-one, `strncpy` length off-by-one, clamp value,
  `active == 1` instead of `!= 0`, NaN operand order, `rem_euclid` instead of
  C's truncating `%`, non-zero padding, `-0.0` instead of `+0.0`, dropped sign
  extension in `process_string`, inactive nodes counted as children) — **all 10
  caught**. Documented non-mutation: `>` → `>=` in the `safe_double_to_int`
  clamps is behaviour-preserving, because `d == (double)INT_MAX` falls through to
  `(int)d` and yields `INT_MAX` anyway.
* **Feature combinations** (`./check_features.sh`): `Cargo.toml` has no
  `[features]`, so `<default>`, `--no-default-features` and `--all-features` are
  the complete set; 85/85 tests pass and the symbol diff is empty in all three.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so`.
- [x] Phase B: all 41 `CONFIGS.md` rows pass across randomized inputs.
- [x] Phase C: all 38 executable `ERRORS.md` rows have a passing differential
      test (E39/E40 are identical, non-observable UB and are documented).
- [x] All of the above hold under every feature combination, and with the Rust
      `.so` built in either the release or the dev profile.
