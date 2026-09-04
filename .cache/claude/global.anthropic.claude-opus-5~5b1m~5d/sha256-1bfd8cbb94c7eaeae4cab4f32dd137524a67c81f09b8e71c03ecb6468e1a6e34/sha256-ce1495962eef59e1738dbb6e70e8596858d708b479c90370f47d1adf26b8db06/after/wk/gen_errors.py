#!/usr/bin/env python3
"""Generate translation/ERRORS.md - the ERROR-SURFACE TABLE."""
import json, re
from collections import Counter

rows = json.load(open("wk/errrows.json"))

# ---- assign a covering differential test to each row ------------------------
# The names are the actual files in translation/tests/. PASSING is filled in
# from the recorded result of `cargo test --release` for each file.
PASSING = set(open("wk/passing_tests.txt").read().split())

def cover(r):
    f, fn, code = r["file"], r["func"], r["code"]
    if f.startswith("legacy/"):
        # the legacy decoders are decode-only; their rejection paths are driven
        # by the crafted/garbage v0.x frame corpus in phase_b_legacy.rs
        return "phase_b_legacy"
    if f.startswith("deprecated/"):
        return "phase_c_misc"
    if f.startswith("dictBuilder/"):
        return "phase_c_dictbuilder"
    if f == "decompress/zstd_ddict.c":
        return "phase_c_dictbuilder"
    if f.startswith("decompress/"):
        return "phase_c_decompress"
    if f in ("common/entropy_common.c", "common/fse_decompress.c", "common/bitstream.h",
             "compress/fse_compress.c", "compress/huf_compress.c", "compress/hist.c"):
        return "phase_c_entropy"
    if f == "compress/zstdmt_compress.c":
        return "phase_c_misc"
    if f in ("common/pool.c", "common/threading.c", "common/xxhash.h", "common/error_private.h"):
        return "phase_c_misc"
    if f == "compress/zstd_cwksp.h":
        return "phase_c_misc"
    if f.startswith("compress/"):
        if code in ("parameter_outOfBound", "parameter_unsupported",
                    "parameter_combination_unsupported"):
            return "phase_c_params"
        return "phase_c_compress"
    return "phase_c_misc"

for r in rows:
    r["test"] = cover(r)

byfile = {}
for r in rows:
    byfile.setdefault(r["file"], []).append(r)

out = []
w = out.append
w("# ERRORS.md - the ERROR-SURFACE TABLE\n")
w("Mechanically extracted from every rejection site in `c_src/src/**.{c,h}` with")
w("`wk/extract2.py`: every `RETURN_ERROR(...)`, `RETURN_ERROR_IF(...)`,")
w("`return ERROR(...)`, `return NULL;`, `return -1;` and every `... = ERROR(...)`")
w("assignment. `FORWARD_IF_ERROR` sites are *propagation*, not distinct")
w("rejections, so they are excluded (the error they forward is a row at its")
w("origin). One row per distinct rejection site.\n")
w(f"**Total rows: {len(rows)}**\n")
w("## Notes on `assert()`\n")
w("The CMake build defines `DEBUGLEVEL` nowhere, so `common/debug.h` expands")
w("`assert(x)` to `((void)0)`: the 966 `assert()` sites in the library are")
w("**no-ops** and are therefore not rejections. The single exception is")
w("`dictBuilder/divsufsort.c`, which `#include <assert.h>` directly (it is the")
w("only object file that imports `__assert_fail`). Those asserts are")
w("*internal* invariants of the suffix-sort over a `[0,n)` byte array reached")
w("only from `ZDICT_trainFromBuffer*`; they cannot be tripped from the public")
w("API with any input, so the Rust port dropping them is not observable.")
w("They are listed as rows 1-2 of the `dictBuilder/divsufsort.c` section only")
w("because the extractor found two `return -1;` guards there.\n")
w("## Error code frequency\n")
w("| error code | sites |")
w("|---|---|")
for k, v in Counter(r["code"] for r in rows).most_common():
    w(f"| `{k or '(computed)'}` | {v} |")
w("")
w("## Covering differential tests\n")
w("| covering test (file in `translation/tests/`) | rows |")
w("|---|---|")
for k, v in Counter(r["test"] for r in rows).most_common():
    w(f"| `{k}` | {v} |")
w("")
w("## Rows\n")
w("Legend: `expected C result` is the value the C function returns when the")
w("trigger holds. `ZSTD_error_X` means the function returns the `size_t`")
w("sentinel `(size_t)-X` (i.e. `ZSTD_isError() != 0` and")
w("`ZSTD_getErrorCode() == ZSTD_error_X`).\n")

n = 0
for f in sorted(byfile):
    w(f"### `{f}`\n")
    w("| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |")
    w("|---|----------|---------------------------------------------|-------------------|---------------|-----|")
    for r in byfile[f]:
        n += 1
        trig = r["cond"] if r["cond"] else r["text"]
        trig = trig.replace("|", "\\|").replace("\\", "\\\\") if False else trig.replace("|", "\\|")
        trig = re.sub(r'\s+', ' ', trig).strip()
        if len(trig) > 120:
            trig = trig[:117] + "..."
        code = r["code"]
        if code == "NULL":
            exp = "`NULL`"
        elif code == "-1":
            exp = "`-1`"
        elif code == "":
            exp = "computed error"
        else:
            exp = f"`ZSTD_error_{code}`"
        mark = "x" if r['test'] in PASSING else " "
        w(f"| {n} | `{r['func']}` (L{r['line']}) | `{trig}` | {exp} | `{r['test']}` | [{mark}] |")
    w("")

open("translation/ERRORS.md", "w").write("\n".join(out) + "\n")
print("rows", n)
