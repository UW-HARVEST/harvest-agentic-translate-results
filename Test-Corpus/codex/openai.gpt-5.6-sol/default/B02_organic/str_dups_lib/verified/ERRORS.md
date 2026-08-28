# Error Surface

The C API is allocation-based and has no error enum. Its observable rejection
surface consists of sentinel/no-op returns, process-fatal invalid pointer
inputs, and assertions guarding internal invariants. Rows cite the source
condition mechanically.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `stbds_hmfree_func` | `a == NULL` (`lib.c:573`) | returns immediately (no-op) |
| [x] 2 | `stbds_hmget_key_ts` | map `a == NULL` (`lib.c:634`) | creates zero default entry; writes `temp = -1` |
| [x] 3 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` (`lib.c:644`) | returns map; writes `temp = -1` |
| [x] 4 | `stbds_hmget_key_ts` | key lookup returns `slot < 0` (`lib.c:648`) | returns map; writes `temp = -1` |
| [x] 5 | `stbds_hmget_key` | missing key (delegates to rows 2-4) | returns map; stores header `temp = -1` |
| [x] 6 | `stbds_hmdel_key` | map `a == NULL` (`lib.c:809`) | returns `NULL` |
| [x] 7 | `stbds_hmdel_key` | map exists but `hash_table == NULL` (`lib.c:816`) | returns same map; stores header `temp = 0` |
| [x] 8 | `stbds_hmdel_key` | key lookup returns `slot < 0` (`lib.c:821`) | returns same map; stores header `temp = 0` |
| [x] 9 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | accepted boundary; deterministic hash |
| [x] 10 | `stbds_hash_bytes` | `p == NULL`, `len > 0` | invalid dereference; process terminates |
| [x] 11 | `stbds_hash_string` | `str == NULL` | invalid dereference; process terminates |
| [x] 12 | `stbds_hmget_key_ts` | `temp == NULL` | invalid write; process terminates |
| [x] 13 | `stbds_stralloc` | `a == NULL` or `str == NULL` | invalid dereference; process terminates |
| [x] 14 | `stbds_strreset` | `a == NULL` | invalid dereference/write; process terminates |
| [x] 15 | hash-map APIs | `mode < 0` (one below `STBDS_HM_BINARY`) | C treats it as binary mode |
| [x] 16 | hash-map APIs | `mode > 1` (one above `STBDS_HM_STRING`) | C treats it as string mode (`mode >= 1`) |
| [x] 17 | `stbds_shmode_func` | mode outside enum `0..=3` | mode is truncated to `unsigned char`; later insert uses default switch arm |
| [x] 18 | `stbds_arrgrowf` | `addlen`/`min_cap` arithmetic wraps at `SIZE_MAX` | C `size_t` arithmetic wraps; capacity/result follows wrapped values |
| [x] 19 | `stbds_hash_bytes` | oversized length (`SIZE_MAX`) with null data | invalid dereference; process terminates |
| [x] 20 | `stbds_make_hash_index` (via put/mode) | `used_threshold + tombstone_threshold >= slot_count` (`lib.c:401`) | assertion failure; unreachable for generated power-of-two table sizes |
| [x] 21 | `stbds_hmput_key` | post-growth `i + 1 > capacity` (`lib.c:778`) | assertion failure; allocator invariant |
| [x] 22 | `stbds_hmdel_key` | found `slot >= table->slot_count` (`lib.c:828`) | assertion failure; table-corruption invariant |
| [x] 23 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`, `lib.c:846`) | assertion failure; table-corruption invariant |
| [x] 24 | `stbds_hmdel_key` | moved element bucket index differs from `final_index` (`lib.c:849`) | assertion failure; table-corruption invariant |
| [x] 25 | `stbds_stralloc` | after block allocation, `len > remaining` (`lib.c:913`) | assertion failure; arena-allocation invariant |
| [x] 26 | `str_dups` | duplicated entry key content is not `'a'` (`lib.c:960`) | assertion failure; composed string-map invariant |
| [x] 27 | `str_dups` | duplicated key aliases source key (`lib.c:961`) | assertion failure; `SH_STRDUP` invariant |
| [x] 28 | `str_dups` | stored value differs from input `num` (`lib.c:962`) | assertion failure; map-value invariant |

