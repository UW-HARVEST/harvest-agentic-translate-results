# Mutation testing — does the differential suite actually bite?

"All tests pass" only means something if the tests can fail.  29 deliberate bugs
were injected into `src/lib.rs`, one at a time, the cdylib rebuilt, and the whole
suite re-run.

```sh
mutation-testing/run.sh                 # all mutants
mutation-testing/run.sh M18_step_growth # just one
```

`raw-runs.txt` holds the captured output of the runs described below.

## Score: 24 caught / 29, and all 5 survivors are provably equivalent mutants

| mutant | what it changes | verdict |
|--------|-----------------|---------|
| `M1_hash_string_const` | `hash * 21` → `* 22` | CAUGHT |
| `M2_used_count_threshold` | `slot_count - (slot_count>>2)` → `>>1` | CAUGHT |
| `M3_siphash_tail_signext` | drops the `int` sign-extension of `d[3] << 24` in `case 4:` | CAUGHT |
| `M4_wrap_loop_tempkey` | *adds* the `stbds_temp_key` store the C omits in the wrap-around duplicate branch | CAUGHT |
| `M5_arrgrowf_min4` | `else if (min_cap < 4)` → `< 5` | survived — **equivalent** |
| `M6_final_index` | `arrlen-1-1` → `arrlen-1` | CAUGHT |
| `M7_strkey_prefix` | `"test_"` → `"tesT_"` | CAUGHT |
| `M8_stralloc_blocksize` | `512 << (block>>1)` → `512 << block` | CAUGHT |
| `M9_probe_position` | `hash & (n-1)` → `(hash>>1) & (n-1)` | CAUGHT |
| `M10_hash_string_rot` | `ROTATE_LEFT(hash,9)` → `,8` | CAUGHT |
| `M11_tombstone_thresh` | `(n>>3)+(n>>4)` → `(n>>3)+(n>>5)` | CAUGHT |
| `M12_hash_seed_mult` | one hex digit of the LCG multiplier | CAUGHT |
| `M13_shmode_cast` | `(unsigned char)mode` → `mode & 0x7f` | CAUGHT |
| `M15_sipround_rot` | SipRound `ROTL(v3,21)` → `,20` | CAUGHT |
| `M16_shrink_threshold` | `slot_count>>2` → `>>3` | CAUGHT |
| `M17_hash_lt2` | `if (hash < 2)` → `< 3` | survived — **equivalent** |
| `M18_step_growth` | `step += 8` → `+= 16` in `stbds_hm_find_slot` | CAUGHT *(only after `tests/probe_paths.rs` + the 16/24-byte key shapes were added — see below)* |
| `M19_strdup_len` | `strlen(str)+1` → `+2` in `stbds_strdup` | survived — **equivalent** |
| `M20_stralloc_ptr` | a `.wrapping_sub(0)` no-op | survived — **no-op** (author error) |
| `M21_align_fwd` | `(n+a-1) & ~(a-1)` → `(n+a) & ~(a-1)` | survived — **equivalent** |
| `M22_hmput_default_cond` | drops the `length == 0` half of the condition | CAUGHT |
| `M23_hmdel_temp` | `stbds_temp = 1` → `2` on a successful delete | CAUGHT |
| `M24_siphash_c_rounds` | `STBDS_SIPHASH_C_ROUNDS` 2 → 3 | CAUGHT |
| `M25_hmput_key_memcpy` | `memcpy(..., keysize)` → `keysize-1` | CAUGHT |
| `M26_hmput_step_growth` | `step += 8` → `+= 16` in `stbds_hmput_key` | CAUGHT (the crafted-table probe loop never terminates; `tests/probe_paths.rs`'s watchdog aborts) |
| `M27_rehash_step_growth` | `step += 8` → `+= 16` in `stbds_make_hash_index` | CAUGHT |
| `M28_find_slot_wrap_limit` | `while i < limit` → `i + 1 < limit` in the wrap-around loop | CAUGHT |
| `M29_tombstone_pos` | `if (tombstone >= 0)` → `> 0` | CAUGHT |
| `M30_rehash_index` | rehash wrap-around `limit` off by one | CAUGHT |

### Why the five survivors cannot be detected

* **`M5`** — the branch is only reached when `min_cap >= 2*arrcap`; the original
  and the mutant differ only for `min_cap == 4`, and both leave the capacity at
  4.
* **`M17`** — differs only when the full 64-bit hash is exactly `2`
  (probability ≈ 2⁻⁶⁴).  Both implementations contain the identical statement.
* **`M19`** — allocates and copies one byte more; the resulting NUL-terminated
  string, and therefore every observable, is unchanged.
* **`M21`** — the argument is always `table + sizeof(stbds_hash_index)` =
  `table + 104` with `table` 16-byte aligned, i.e. `≡ 8 (mod 16)`, so it is
  never 64-byte aligned and the two expressions always agree.
* **`M20`** — syntactically different, semantically identical (a badly generated
  mutant, kept for honesty).

## The one real gap this found

`M18` / `M26` / `M27` all mutate the same multi-bucket probe walk

```c
pos += step;  step += STBDS_BUCKET_LENGTH;  pos &= (table->slot_count-1);
```

which appears three times (`stbds_hm_find_slot`, `stbds_hmput_key`,
`stbds_make_hash_index`).  The `step` growth is only observable from the **second**
hop onwards, which needs two *consecutive* completely-full 8-slot buckets — and
the table never exceeds 75 % load, so random/property data essentially never
produces that.  In the first run `M18` survived.

`tests/probe_paths.rs` was added to close it: it builds the bucket array by hand
(byte-identically in the C map and the Rust map) and then drives the real
exported entry points over it, forcing 2+ hops through all three copies of the
walk, plus the tombstone-reuse and wrap-around-duplicate variants
(`CONFIGS.md` rows 71–77).  The wider key shapes (`keysize` 16 and 24) added to
`tests/maps_binary.rs` at the same time make the natural tables dense enough
that `M18` is now caught by `r25_get_present_and_absent` as well.
