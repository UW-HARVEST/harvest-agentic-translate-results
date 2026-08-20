# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so` (cmake default config, no `-O`)
* Rust `.so`: `target/{debug,release}/libima_parse_lib.so`

Regenerate with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/c.syms
nm -D --defined-only target/debug/libima_parse_lib.so  | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # MUST be empty
```

## Exported (dynamic, global) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `ima_parse` | `T` | `T` | `int ima_parse(struct ima_info *, const void *)`; declared in `include/lib.h`. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn ima_parse` in `src/parse.rs`. |

**Missing from Rust `.so`: 0.**

## Non-exported (`static`) C symbols — intentionally NOT in the Rust dynamic table

These are `static` in `c_src/src/lib.c`, so `nm` reports them as local (`t`) and they
are absent from `nm -D`. They are not part of the ABI and an external caller cannot
reach them, so the Rust `.so` must **not** export them either (it does not). They are
still fully translated in Rust (`src/endian.rs`) and are covered *indirectly* by the
differential tests, which give each one an exact oracle:

| C local symbol | Rust counterpart | how the differential tests exercise it |
|---|---|---|
| `ima_bswap16` / `ima_btoh16` | `endian::ima_bswap16` / `ima_btoh16` | `header.version` check — all 65536 values swept (`sweep_all_65536_version_values`) |
| `ima_bswap32` / `ima_btoh32` | `endian::ima_bswap32` / `ima_btoh32` | `header.type`, `chunk.type`, `desc.format_id` compares **and** `info.channel_count = ima_btoh32(desc->channels_per_frame)`, which is a direct 32-bit oracle over random inputs |
| `ima_bswap64` / `ima_btoh64` | `endian::ima_bswap64` / `ima_btoh64` | `info.frame_count = ima_btoh64(pakt->frame_count)` and `chunk_size = ima_btoh64(chunk->size)` — direct 64-bit oracles over random inputs |

## Linker-provided / toolchain symbols (not part of the library ABI)

`_init`, `_fini`, `_DYNAMIC`, `_GLOBAL_OFFSET_TABLE_`, `__dso_handle`,
`frame_dummy`, `register_tm_clones`, `deregister_tm_clones`,
`__do_global_dtors_aux`, `completed.0`, `__TMC_END__`, `__FRAME_END__`,
`__GNU_EH_FRAME_HDR`, and the weak `_ITM_*` / `__gmon_start__` /
`__cxa_finalize` imports. Emitted by the toolchain, not by the source; excluded
from the parity requirement.

The Rust `.so` additionally *imports* (`U`) libc and `_Unwind_*` symbols because it
links the Rust standard library. These are undefined-imports satisfied by the system
libraries, not missing definitions, and they are all libc/libgcc — so
"0 missing/undefined non-libc symbols" holds.

## Verified ABI facts (from `objdump -d` of the C `.so`)

The Rust translation hard-codes C struct offsets. Each was confirmed against the
actual C codegen rather than assumed:

| fact | C instruction | value |
|---|---|---|
| `sizeof(struct caf_header)` | `add $0x8,%rax` | 8 |
| `header->type` @ | `mov (%rax),%eax` | 0 |
| `header->version` @ | `movzwl 0x4(%rax),%eax` | 4 |
| `chunk->type` @ | `mov (%rax),%eax` | 0 |
| `chunk->size` @ | `mov 0x8(%rax),%rax` | **8** (4 bytes tail padding after `type`) |
| `sizeof(struct caf_chunk)` | `add $0x10,%rax` | **16** |
| `desc = &chunk[1]` | `add $0x10,%rax` | chunk + 16 |
| `pakt = &chunk[1]` | `add $0x10,%rax` | chunk + 16 |
| `blocks = &((caf_data*)&chunk[1])[1]` | `add $0x14,%rax` | chunk + **20** |
| chunk walk | `add $0x10,%rax; add %rax,-0x8(%rbp)` | chunk += size + 16 |
| `desc->format_id` @ | `mov 0x8(%rax),%eax` | 8 |
| `desc->channels_per_frame` @ | `mov 0x18(%rax),%eax` | 24 |
| `pakt->frame_count` @ | `mov 0x8(%rax),%rax` | 8 |
| `info->blocks` @ | `mov %rdx,(%rax)` | 0 |
| `info->size` @ | `mov %rdx,0x8(%rax)` | 8 |
| `info->sample_rate` @ | `movsd %xmm0,0x10(%rax)` | 16 |
| `info->frame_count` @ | `mov %rax,0x18(%rdx)` | 24 |
| `info->channel_count` @ | `mov %eax,0x20(%rdx)` | 32 (4-byte store; bytes 36..40 left untouched) |
| `'caff'` | `cmp $0x63616666,%eax` | 0x63616666 |
| `'desc'` | `cmpl $0x64657363` | 0x64657363 |
| `'pakt'` | `cmpl $0x70616b74` | 0x70616b74 |
| `'data'` | `cmpl $0x64617461` | 0x64617461 |
| `'ima4'` | `cmp $0x696d6134,%eax` | 0x696d6134 |
| `2^63` constant | `.rodata: 00000000 0000e043` | `0x43e0000000000000` |
| `double`->`u64` lowering | `comisd; jae; cvttsd2si; / subsd; cvttsd2si; xor $1<<63` | see `src/parse.rs::f64_to_u64` |
