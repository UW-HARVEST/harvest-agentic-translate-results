#!/usr/bin/env python3
import bisect
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CROOT = ROOT.parent / "c_src"
CSO = CROOT / "build/libzstd.so"
RSO = ROOT / "target/release/libzstd.so"


def nm(path):
    out = subprocess.check_output(["nm", "-D", "--defined-only", str(path)], text=True)
    result = {}
    for line in out.splitlines():
        fields = line.split()
        if len(fields) >= 3:
            result[fields[2]] = fields[1]
    return result


csyms, rsyms = nm(CSO), nm(RSO)
with (ROOT / "SYMBOLS.md").open("w") as f:
    f.write("# Dynamic symbol parity\n\n")
    f.write(f"C exports: {len(csyms)}. Rust exports: {len(rsyms)}. "
            f"Missing from Rust: {len(set(csyms)-set(rsyms))}.\n\n")
    f.write("| # | symbol | C kind | Rust kind | status |\n|---:|---|:---:|:---:|:---:|\n")
    for i, name in enumerate(sorted(csyms), 1):
        rk = rsyms.get(name, "—")
        f.write(f"| {i} | `{name}` | {csyms[name]} | {rk} | "
                f"{'[x]' if rk == csyms[name] else '[ ]'} |\n")

sources = sorted(
    p for d in ("common", "compress", "decompress", "dictBuilder", "deprecated", "legacy")
    for p in (CROOT / "src" / d).glob("*.c")
)
tags = {}
tagout = subprocess.check_output(
    ["ctags", "-x", "--c-kinds=f", *map(str, sources)], text=True
)
for line in tagout.splitlines():
    m = re.match(r"(\S+)\s+function\s+(\d+)\s+(\S+)", line)
    if m:
        tags.setdefault(Path(m.group(3)).resolve(), []).append((int(m.group(2)), m.group(1)))
for values in tags.values():
    values.sort()

patterns = re.compile(
    r"\bRETURN_ERROR(?:_IF)?\s*\(|\bERROR\s*\(|\bassert\s*\("
    r"|return\s+(?:NULL|-1)\s*;|\bif\s*\([^;]*(?:NULL|==\s*0|!=\s*0|[<>]=?)"
)
rows = []
for path in sources:
    resolved = path.resolve()
    funcs = tags.get(resolved, [])
    starts = [x[0] for x in funcs]
    lines = path.read_text(errors="replace").splitlines()
    for lineno, raw in enumerate(lines, 1):
        text = raw.strip()
        if not text or text.startswith(("//", "*", "/*", "#")) or not patterns.search(text):
            continue
        idx = bisect.bisect_right(starts, lineno) - 1
        fn = funcs[idx][1] if idx >= 0 else "(file scope)"
        window = " ".join(x.strip() for x in lines[lineno-1:min(lineno+4, len(lines))])
        expected = "branch-specific rejection/error"
        m = re.search(r"RETURN_ERROR(?:_IF)?\s*\([^,)]*,?\s*([A-Za-z0-9_]+)", window)
        if m:
            expected = f"`ERROR({m.group(1)})`"
        else:
            m = re.search(r"\bERROR\s*\(\s*([A-Za-z0-9_]+)", window)
            if m:
                expected = f"`ERROR({m.group(1)})`"
            elif "return NULL" in window:
                expected = "`NULL`"
            elif "return -1" in window:
                expected = "`-1`"
            elif "assert" in text:
                expected = "assertion failure"
        trigger = text.replace("|", "\\|").replace("`", "'")
        rows.append((path.relative_to(CROOT), lineno, fn, trigger, expected))

with (ROOT / "ERRORS.md").open("w") as f:
    f.write("# Error surface\n\n")
    f.write("Mechanically extracted from every compiled C source file. Rows include explicit "
            "error macros/returns/assertions and null/range conditions; file and line preserve "
            "the exact source location.\n\n")
    f.write("| # | function | trigger (exact C condition/statement) | expected C result | tested |\n")
    f.write("|---:|---|---|---|:---:|\n")
    for i, (path, line, fn, trigger, expected) in enumerate(rows, 1):
        f.write(f"| {i} | `{fn}` ({path}:{line}) | `{trigger}` | {expected} | [ ] |\n")

axes = [
    ("ZSTD_compress / ZSTD_decompress", "sizes 0, 1, small, 127/128 KiB boundary, and many blocks; random and repetitive bytes"),
    ("ZSTD_compress / ZSTD_compressCCtx / ZSTD_compress2", "compression levels min, default, max, and negative fast levels"),
    ("ZSTD_compress2", "content-size flag on/off; checksum on/off; dictID on/off"),
    ("ZSTD_compressStream2", "continue, flush, end; empty/one/many input chunks; tiny/recommended output buffers"),
    ("ZSTD_decompressStream", "empty/partial/full frames; one-byte and randomized chunk boundaries"),
    ("ZSTD_*usingDict / loadDictionary / refPrefix", "empty, raw-content, and formatted dictionaries; by-copy and by-reference"),
    ("ZSTD_*usingCDict / *usingDDict", "prepared dictionaries with min/default/max levels"),
    ("ZSTD_getFrameHeader_advanced", "standard and magicless formats; partial and complete headers"),
    ("ZSTD_writeSkippableFrame / ZSTD_readSkippableFrame", "magic variants 0 and 15; empty and non-empty payloads"),
    ("ZSTD_CCtx_reset / ZSTD_DCtx_reset", "session-only, parameters-only, and session+parameters"),
    ("ZSTD_CCtx_setParameter", "every declared ZSTD_cParameter at lower bound, upper bound, and defaults"),
    ("ZSTD_DCtx_setParameter", "every declared ZSTD_dParameter at lower bound, upper bound, and defaults"),
    ("ZBUFF_*", "init, continue, flush/end; empty, one, and many chunks"),
    ("ZDICT_*", "legacy, cover, and fastCover training; small and multi-sample corpora"),
    ("legacy ZSTDv01..ZSTDv07 / ZBUFFv04..v07", "version-specific frame and streaming paths"),
    ("FSE_* / HUF_* / HIST_*", "RLE, raw, compressed tables; single-symbol and full-alphabet histograms"),
    ("ZSTD_XXH32 / ZSTD_XXH64", "one-shot and streaming; empty, aligned, unaligned, and long inputs"),
]
with (ROOT / "CONFIGS.md").open("w") as f:
    f.write("# Configuration surface\n\n")
    f.write("The first section mechanically enumerates every dynamically public entry point. "
            "The second records source/header-derived option and input-shape axes which are "
            "special-cased by the implementation.\n\n")
    f.write("| # | entry point(s) | configuration (options set + input shape) | status |\n")
    f.write("|---:|---|---|:---:|\n")
    n = 0
    for name in sorted(csyms):
        n += 1
        shape = "exported data object: external read/write" if csyms[name] != "T" else "direct FFI call with declaration-valid arguments and boundary-shaped inputs"
        f.write(f"| {n} | `{name}` | {shape} | [ ] |\n")
    for entry, shape in axes:
        n += 1
        f.write(f"| {n} | `{entry}` | {shape} | [ ] |\n")
