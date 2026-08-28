# CONFIGS.md — Phase A: configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` (the only translation unit) and
`c_src/include/lib.h`.

**Cargo features:** `translation/Cargo.toml` declares **no `[features]` table**,
so the only feature configuration is the default (empty) one. There are no
`#ifdef`s in the C source either — there is exactly one compile-time
configuration. Verified by script (`check_features.sh`); Phases B/C are run for
`--no-default-features` and default, which are identical here.

**Runtime "options" / mode words** (there is no init/context object; the mode is
carried in `int` flag words):

* A1 `process_flags(flags)` — 4 independently tested bits:
  `FLAG_READ 0b0001`, `FLAG_WRITE 0b0010`, `FLAG_EXECUTE 0b0100`,
  `FLAG_DELETE 0b1000`. Branch-free (`!!`), but the value space is
  {16 low-nibble combos} × {reserved high bits clear / set} × {sign}.
* A2 `matrixsum(p1..p4)` — each parameter's *zero-ness* selects a flag bit
  (`!!paramN`), so 2^4 = 16 distinct `permissions` configurations, crossed with
  the *magnitude* of the parameters (which only feeds `sum`).
* A3 global mutable state: the exported `matrix[3][4]` data object. Default
  initializer vs. caller-mutated contents changes
  `calculate_matrix_checksum()` and hence `matrixsum`'s
  `(matrix_sum & 0xFFF)` term.

**Input shapes / sizes** (the `DynamicArray` allocator API):

* A4 `init_array(initial_capacity)` — `capacity == 0`, `1`, `2` (the value
  `matrixsum` uses), small, large, byte-count-wrapping (`2^62`), un-servicable.
* A5 `add_element` — `size < capacity` (no growth), `size == capacity`
  (exactly one growth), and many pushes (repeated doubling: 1→2→4→…).
* A6 element values — `0`, `±1`, `INT_MIN`, `INT_MAX`, random full-range
  (drives signed-overflow wrapping of `sum` in `matrixsum`).
* A7 call-order / lifecycle — `init` → `add*` → read `data[]` → `free`;
  `init` → `expand*` → `add*`; `expand` before any `add`; `free` twice-safe
  variants (NULL); caller-constructed struct.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised against BOTH `.so`s with **many** seeded-random inputs
(seed `0x5EED_1234_ABCD_0001`, xorshift64\*), not a single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C1  | `process_flags` | exhaustive `flags` = 0..=255 (all 16 low-nibble combos × 16 reserved-bit patterns) | [x] |
| C2  | `process_flags` | 4096 seeded-random full-range `i32` (negatives, high bits set) | [x] |
| C3  | `process_flags` | boundary words: `0`, `1`, `2`, `4`, `8`, `15`, `16`, `-1`, `i32::MIN`, `i32::MAX`, `i32::MIN+1`, `i32::MAX-1`, `0xF0`, `!0xF` | [x] |
| C4  | `calculate_matrix_checksum` | pristine `matrix` (default initializer) — must be 916 in both | [x] |
| C5  | `calculate_matrix_checksum` + `matrix` (data symbol) | `matrix` overwritten with 512 seeded-random `i32[12]` patterns in both `.so`s (proves the global is really loaded, not folded) | [x] |
| C6  | `calculate_matrix_checksum` + `matrix` | `matrix` filled with overflow-inducing extremes (`i32::MAX` ×12, `i32::MIN` ×12, mixed) → signed wrap of `sum` | [x] |
| C7  | `init_array` + field inspection | `capacity` ∈ {0,1,2,3,4,7,8,16,63,64,1000,65536,1<<20} — check returned struct layout (`data != NULL`, `size == 0`, `capacity` echoed) | [x] |
| C8  | `init_array` + `free_array` | round-trip lifecycle for each capacity of C7, no leak/crash, `free_array` on a live array | [x] |
| C9  | `init_array` + `add_element` (no growth) | `capacity = n`, push `k < n` seeded-random values, read back `data[0..k]`, compare buffers byte-for-byte + `size`/`capacity` | [x] |
| C10 | `init_array` + `add_element` (exactly one growth) | `capacity = n`, push exactly `n+1` values → one doubling; compare `size`, `capacity == 2n`, and all `n+1` elements | [x] |
| C11 | `init_array` + `add_element` (many growths) | `capacity = 1` or `2`, push 1..=257 values → repeated doubling chain; compare full buffer + final `capacity` | [x] |
| C12 | `init_array` + `expand_array` directly | `expand_array` called 0..=8 times on a fresh array before/between `add_element`s; compare return code + `capacity` sequence | [x] |
| C13 | `add_element` | element values from the extremes set {0, 1, -1, `i32::MIN`, `i32::MAX`} and random full-range, stored/read back exactly | [x] |
| C14 | `matrixsum` | all 16 zero/non-zero parameter combos (`permissions` = 0b0000..0b1111) with representative non-zero magnitudes | [x] |
| C15 | `matrixsum` | 8192 seeded-random full-range `(p1,p2,p3,p4)` — signed-overflow wrapping of `sum` and `sum * 0x10` | [x] |
| C16 | `matrixsum` | extreme quadruples from {0, ±1, `i32::MIN`, `i32::MAX`, `i32::MIN+1`, `i32::MAX-1`, `0x08000000`, `-0x08000000`} — full 8^4 = 4096 cross-product | [x] |
| C17 | `matrixsum` + `matrix` | `matrixsum` after mutating `matrix` in both `.so`s (256 random matrices × random params) — exercises the `(matrix_sum & 0xFFF)` masking with negative/large checksums | [x] |
| C18 | `matrixsum` (repeat / statefulness) | same call repeated 1000× to confirm no residual state and that the internal `init_array`/`free_array` cycle is clean | [x] |
| C19 | full low-level pipeline | `init_array(cap)` → interleaved `add_element`/`expand_array` per a seeded random script → read `data`, `size`, `capacity` → `free_array`; 512 random scripts | [x] |
| C20 | `matrix` (data symbol) | exported object identity: 48 bytes, same default contents in both `.so`s, writable, byte-for-byte comparison of the raw 48 bytes | [x] |
| C21 | `matrix` (data symbol) — **as-linked initializer** | the 48 initializer bytes compared BEFORE anything writes to the global, in a dedicated test process (`phase_b_pristine.rs`). Without this row a wrong initializer compiled into the Rust `.so` is masked by the test's own reset — `mutation_check.sh` proved it | [x] |
| C22 | `calculate_matrix_checksum`, `matrixsum` on untouched globals | first-touch state of the process: checksum == 916 and 8 representative `matrixsum` calls, with nothing ever having written `matrix` | [x] |
| C23 | `matrix` partial mutation | exactly one of the 12 slots overwritten (12 slots × 6 values) → proves all 12 are summed and in the right order | [x] |
| C24 | `expand_array` — byte-count wrap to small non-zero | `capacity` ∈ {2^61+1, 2^62+1, 2^61+2, 2^62+2, 2^63+1} → `realloc` succeeds on a wrapped tiny size; the absurd doubled capacity must be stored verbatim | [x] |
| C25 | `init_array` — byte-count wrap to small non-zero, then used | `capacity = 2^62+n` (n = 1..4) → real `4n`-byte buffer, huge recorded capacity, `n` `add_element`s written and read back | [x] |
| C26 | `calculate_matrix_checksum` via the unprototyped `int f()` ABI | called through `extern "C" fn(c_int,c_int,c_int,c_int)` with 256 random argument sets; result must equal the zero-arg form in both | [x] |
| C27 | `init_array` + `add_element` — long doubling chain | `capacity` ∈ {1,2,3,5}, 10 000 pushes each → 12–14 reallocations, full 10 000-element buffer compared | [x] |
| C28 | ALL seven entry points + `matrix`, interleaved | 20 000-step randomized script over `init_array`/`add_element`/`expand_array`/`free_array`/`process_flags`/`matrix` writes/`calculate_matrix_checksum`/`matrixsum` with up to 24 live arrays; every observable compared after every step | [x] |
| C29 | allocator accounting | `free_array`/`matrixsum`/`expand_array` churn measured with `mallinfo2()`; net footprint must match (see the L-rows in `ERRORS.md`) | [x] |

| C30 | caller-supplied pointer alignment | `DynamicArray *` at an odd address, and `data` 4-byte-misaligned, through `add_element`/`expand_array`/`free_array` — the C makes no alignment promise (see the Z-rows in `ERRORS.md`) | [x] |
| C31 | `matrixsum` call structure | the helper chain must be reached through real, interposable calls so that both `malloc`s, the `realloc` and both `free`s actually happen (see the T-rows in `ERRORS.md`) | [x] |

Every row is verified against BOTH cdylib profiles (`debug` and `release`) and
against the C compiled at every `CMAKE_BUILD_TYPE` (default/`-O0`, `Release`,
`RelWithDebInfo`, `MinSizeRel`, `Debug`) — 10 combinations, all passing.

## Suite self-validation

`mutation_check.sh` injects 35 deliberate bugs into a COPY of the Rust crate
(`c_src/` and `translation/src/` are never touched), builds that copy in BOTH
profiles, points the suite at each via `RUST_SO`, and requires the suite to FAIL.
Result: **33 killed, 2 survivors, both provably semantics-preserving** (`!!x`
removed where `x` is already 0/1, and `!!x` removed where the value is only
consumed as `if x != 0`). Real blind spots found and closed this way:

1. the `matrix` **initializer** was never compared (the harness reset the global
   to its own constant first) → added `phase_b_pristine.rs` (C21–C22);
2. a missing `free()` in `free_array` was invisible to every return-value
   comparison → added `phase_c_leak.rs` (C29 / L1–L3);
3. crash modes and side-effect ordering were invisible because the process dies
   → added `phase_c_crash.rs` (C30 / Z1–Z5);
4. the release inliner elided an allocation → added
   `phase_d_alloc_traffic.rs` (C31 / T1–T4).
