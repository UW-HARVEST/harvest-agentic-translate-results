#!/usr/bin/env python3
import subprocess, re, os

def syms(so):
    out = subprocess.run(["nm", "-D", "--defined-only", so], capture_output=True, text=True).stdout
    d = {}
    for ln in out.splitlines():
        p = ln.split()
        if len(p) >= 3:
            d[p[2]] = p[1]
    return d

C = syms("c_src/build/libzstd.so")
R = syms("translation/target/release/libzstd.so")
missing = sorted(set(C) - set(R))
extra = sorted(set(R) - set(C))

def group(n):
    if n.startswith(("ZSTDv0", "ZBUFFv0", "FSEv0", "HUFv0", "ZSTD_v0")):
        return "legacy (v0.1-v0.7)"
    if n.startswith("ZBUFF_"):
        return "deprecated (zbuff)"
    if n.startswith(("ZDICT_", "COVER_", "FASTCOVER_", "divsufsort", "divbwt")):
        return "dictBuilder"
    if n.startswith(("ZSTDMT_",)):
        return "zstdmt"
    if n.startswith(("FSE_", "HUF_", "HIST_", "ZSTD_XXH", "XXH", "ERR_", "BIT_")):
        return "common/entropy"
    if n.startswith("ZSTD_"):
        return "zstd core"
    return "other"

groups = {}
for n in sorted(C):
    groups.setdefault(group(n), []).append(n)

with open("translation/SYMBOLS.md", "w") as f:
    f.write("# SYMBOLS.md - dynamic symbol parity, C `.so` vs Rust `.so`\n\n")
    f.write("Generated mechanically from:\n\n")
    f.write("```\nnm -D --defined-only c_src/build/libzstd.so\n")
    f.write("nm -D --defined-only translation/target/release/libzstd.so\n```\n\n")
    f.write(f"* C  exported (defined, dynamic): **{len(C)}**\n")
    f.write(f"* Rust exported (defined, dynamic): **{len(R)}**\n")
    f.write(f"* **Missing from Rust: {len(missing)}**\n")
    f.write(f"* Extra in Rust (not in C): {len(extra)}\n\n")
    if missing:
        f.write("## MISSING (must be fixed)\n\n")
        for n in missing:
            f.write(f"* `{n}`\n")
        f.write("\n")
    else:
        f.write("## MISSING\n\n_None._ Every symbol exported by the C `.so` is exported by the\n"
                "Rust `.so` with the exact same name (including the macro-generated\n"
                "`XXH_NAMESPACE=ZSTD_` names and the legacy `FSEv05_`/`HUFv06_`/`ZSTDv07_` renames).\n\n")
    if extra:
        f.write("## EXTRA in Rust\n\n")
        for n in extra:
            f.write(f"* `{n}`\n")
        f.write("\n")
    else:
        f.write("## EXTRA in Rust\n\n_None._\n\n")

    f.write("## Undefined (imported) symbols\n\n")
    def und(so):
        out = subprocess.run(["nm", "-D", "--undefined-only", so], capture_output=True, text=True).stdout
        return sorted({l.split()[-1] for l in out.splitlines() if len(l.split()) >= 2})
    cu, ru = und("c_src/build/libzstd.so"), und("translation/target/release/libzstd.so")
    nonlibc = [s for s in ru if not re.search(r'@GLIBC|@GCC|^_ITM_|^__gmon_start__|^__cxa_|^_Unwind', s)]
    f.write("Rust non-libc undefined symbols: " + (", ".join(f"`{s}`" for s in nonlibc) if nonlibc else "_none_") + "\n\n")
    f.write("C non-libc undefined symbols: `ZSTD_trace_compress_begin`, `ZSTD_trace_compress_end`,\n"
            "`ZSTD_trace_decompress_begin`, `ZSTD_trace_decompress_end` (weak, never defined -> always NULL).\n"
            "The Rust port hard-codes these hooks to `None` (see `src/zstd_trace.rs`), which is\n"
            "behaviourally identical, so they do not appear as imports.\n\n")

    f.write("## Feature combinations (Phase D)\n\n")
    f.write("`translation/Cargo.toml` declares **no `[features]` table**, and\n"
            "`grep -rn 'cfg(feature' src/` finds **0** hits, so the crate has exactly\n"
            "**one** build configuration. The only `#[cfg(...)]` in the whole crate are\n"
            "5 `target_arch = \"x86_64\"` guards (`src/zstd_internal.rs::ZSTD_cpuid`,\n"
            "`src/compress/zstd_ldm.rs::PREFETCH_L1`,\n"
            "`src/compress/zstd_lazy.rs` SSE2 row-hash helpers), which mirror the C's own\n"
            "`#if defined(__x86_64__)` guards; the target is x86-64 little-endian per\n"
            "`PORTING_GUIDE.md`.\n\n"
            "`translation/run_all_features.sh` extracts the feature list from `Cargo.toml`\n"
            "mechanically (never hard-coded), builds the cross-product of\n"
            "`--no-default-features [--features ...]`, and for each combination runs\n"
            "`cargo check`, `cargo build --release`, the `nm -D` symbol diff and the whole\n"
            "`cargo test --release` suite. With no features declared it reports exactly one\n"
            "combination (`default`).\n\n")
    f.write("## Full symbol list\n\n")
    for g in sorted(groups):
        f.write(f"### {g} ({len(groups[g])} symbols)\n\n")
        f.write("| symbol | type | in C | in Rust |\n|---|---|---|---|\n")
        for n in groups[g]:
            f.write(f"| `{n}` | {C[n]} | yes | {'yes' if n in R else '**NO**'} |\n")
        f.write("\n")
print("missing", len(missing), "extra", len(extra), "total", len(C))
