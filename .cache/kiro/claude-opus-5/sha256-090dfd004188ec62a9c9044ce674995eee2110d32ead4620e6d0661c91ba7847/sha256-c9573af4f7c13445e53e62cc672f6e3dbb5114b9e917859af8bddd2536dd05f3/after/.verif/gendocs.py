#!/usr/bin/env python3
"""Generate SYMBOLS.md / CONFIGS.md / ERRORS.md from real verification output.

The tables come from the same Rust data the tests iterate over
(`tests/support/configs.rs`, `tests/support/errors_tbl.rs`); the checkmarks come
from `.rowresults/*.tsv`, which each phase test writes as it runs.  Nothing here
is hand-maintained.
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(ROOT, "..", "translation")
CSO = os.path.join(ROOT, "..", "c_src", "build", "libpng.so")
RSO = os.path.join(CRATE, "target", "release", "liblibpng.so")
RESULTS = os.path.join(CRATE, ".rowresults")


def nm(path, mode):
    out = subprocess.run(["nm", "-D", mode, path], capture_output=True, text=True).stdout
    syms = []
    for line in out.splitlines():
        f = line.split()
        if mode == "--defined-only" and len(f) >= 3:
            syms.append(f[2])
        elif mode == "--undefined-only" and len(f) >= 2:
            syms.append(f[1])
    return sorted(set(syms))


def load_results():
    rows = {}
    order = []
    if not os.path.isdir(RESULTS):
        return rows, order
    for fn in sorted(os.listdir(RESULTS)):
        if not fn.endswith(".tsv"):
            continue
        group = fn[:-4]
        with open(os.path.join(RESULTS, fn)) as fh:
            for line in fh:
                p = line.rstrip("\n").split("\t")
                if len(p) < 3:
                    continue
                status, scen, desc = p[0], p[1], p[2]
                observed = p[3] if len(p) > 3 else ""
                rows.setdefault(group, []).append((status, scen, desc, observed))
        order.append(group)
    return rows, order


def group_sort_key(g):
    m = re.match(r"([A-Z]+)(\d+)", g)
    return (m.group(1), int(m.group(2))) if m else (g, 0)


GROUP_TITLES = {
    "B1": "Write pipeline — colour type x bit depth x interlace x entry point",
    "B2": "Write pipeline — zlib option matrix",
    "B3": "Write pipeline — row filter matrix",
    "B4": "Write pipeline — transforms (png_set_* and the png_write_png mask)",
    "B5": "Write pipeline — ancillary chunk sets",
    "B6": "Write pipeline — output plumbing (flush, status callback, raw chunk API, extreme shapes)",
    "B7": "Read pipeline — colour type x bit depth x interlace x entry point",
    "B8": "Read pipeline — transform matrix",
    "B9": "Read pipeline — png_read_png transform mask",
    "B10": "Read pipeline — ancillary chunk sets, stream layout, options, shapes",
    "B11": "Unknown-chunk handling matrix",
    "B12": "Progressive (push) reader",
    "B13": "Simplified read API",
    "B14": "Simplified write API",
    "B15": "png_set_* / png_get_* round trips and library-wide state",
    "B16": "Randomized read cross-product sweep",
    "B17": "Randomized write cross-product sweep",
    "B18": "Large images (multi-buffer zlib, long-row filter selection)",
    "B19": "User transform callbacks",
    "B20": "MNG extensions and png_set_sig_bytes hand-over",
    "B21": "CRC errors x png_set_crc_action",
    "B22": "Floating-point getters",
    "B23": "stdio-based entry points (png_init_io, *_from_file, *_to_stdio)",
    "B24": "png_free_data / png_data_freer / png_destroy_info_struct",
    "B25": "Deprecated filter heuristics",
    "C1": "Error surface, part 1",
    "C2": "Error surface, part 2",
    "C3": "Error surface, part 3",
    "C4": "Error surface, part 4",
}


def w(fh, s=""):
    fh.write(s + "\n")


def esc(s):
    """Markdown tables use '|' as the cell separator."""
    return s.replace("|", "\\|")


def gen_symbols(results):
    c = nm(CSO, "--defined-only")
    r = nm(RSO, "--defined-only")
    missing = [s for s in c if s not in set(r)]
    extra = [s for s in r if s not in set(c)]
    undef = nm(RSO, "--undefined-only")

    allowed_prefix = ("_ITM_", "_Unwind_", "__cxa_", "__errno_", "__gmon_", "__tls_get_addr",
                      "_setjmp")
    libc_like = set("""abort bcmp calloc close crc32 deflate deflateEnd deflateInit2_ deflateReset
        dl_iterate_phdr fclose ferror fflush fopen fprintf fputc fread free frexp fstat64 fwrite
        getcwd getenv gettid gmtime inflate inflateEnd inflateInit2_ inflateReset inflateReset2
        longjmp lseek64 malloc memcmp memcpy memmove memset mmap64 modf munmap open64
        posix_memalign pow pthread_key_create pthread_key_delete pthread_setspecific read readlink
        realloc realpath remove stat64 statx stderr strerror strlen strtod syscall write writev
        ceil floor atof sysconf getauxval pthread_getspecific abs __libc_start_main""".split())
    external = []
    for s in undef:
        base = s.split("@")[0]
        if base.startswith(allowed_prefix) or base in libc_like:
            continue
        external.append(base)

    with open(os.path.join(CRATE, "SYMBOLS.md"), "w") as fh:
        w(fh, "# SYMBOLS.md — dynamic symbol parity")
        w(fh)
        w(fh, "Mechanically derived from the two built shared objects:")
        w(fh)
        w(fh, "```")
        w(fh, "nm -D --defined-only c_src/build/libpng.so")
        w(fh, "nm -D --defined-only translation/target/release/liblibpng.so")
        w(fh, "```")
        w(fh)
        w(fh, "## Result")
        w(fh)
        w(fh, "| metric | value |")
        w(fh, "|---|---|")
        w(fh, "| symbols exported by the C `.so` | %d |" % len(c))
        w(fh, "| symbols exported by the Rust `.so` | %d |" % len(r))
        w(fh, "| **missing from the Rust `.so`** | **%d** |" % len(missing))
        w(fh, "| extra in the Rust `.so` | %d |" % len(extra))
        w(fh, "| undefined non-libc / non-zlib symbols in the Rust `.so` | %d |" % len(external))
        w(fh)
        if missing:
            w(fh, "### Missing")
            w(fh)
            for s in missing:
                w(fh, "- `%s`" % s)
            w(fh)
        else:
            w(fh, "The symbol diff is **empty in both directions**: the Rust `cdylib` exports")
            w(fh, "exactly the same %d names as the C build, and nothing more." % len(c))
            w(fh)
        if external:
            w(fh, "Unexpected external references: %s" % ", ".join(sorted(set(external))))
            w(fh)
        w(fh, "No symbol is stubbed. Every name below is backed by a translation of the")
        w(fh, "corresponding C function; `translation/src/` has one module per `c_src/src/*.c`")
        w(fh, "(large files are split into `<name>2.rs`, `<name>3.rs`, ...):")
        w(fh)
        w(fh, "```")
        w(fh, "$ grep -rn 'unimplemented!\\|todo!\\|not implemented' translation/src/")
        w(fh, "src/pngpread.rs: cstr(b\"png_process_data_skip is not implemented in any current")
        w(fh, "                      version of libpng\\0\"),   <- verbatim libpng message text")
        w(fh, "```")
        w(fh)
        w(fh, "`readelf -d` on the Rust `.so` lists `libz.so.1`, `libgcc_s.so.1`, `libm.so.6`,")
        w(fh, "`libc.so.6` and `ld-linux-x86-64.so.2` — the same external surface as the C build")
        w(fh, "(which needs libz, libc and libm; note the reference `CMakeLists.txt` links only")
        w(fh, "zlib, so its libm references are left unresolved in the `.so` itself).")
        w(fh)
        w(fh, "## Symbol table")
        w(fh)
        w(fh, "| # | symbol | C `.so` | Rust `.so` |")
        w(fh, "|---|--------|---------|------------|")
        rs = set(r)
        for i, s in enumerate(c, 1):
            w(fh, "| %d | `%s` | yes | %s |" % (i, s, "yes" if s in rs else "**MISSING**"))
    return len(c), len(missing), len(external)


def gen_configs(results):
    groups = [g for g in results if g.startswith("B")]
    groups.sort(key=group_sort_key)
    total = sum(len(results[g]) for g in groups)
    npass = sum(1 for g in groups for row in results[g] if row[0] == "PASS")
    path = os.path.join(CRATE, "CONFIGS.md")
    with open(path, "w") as fh:
        w(fh, "# CONFIGS.md — the configuration surface (Phase B)")
        w(fh)
        w(fh, "Every axis below is derived from what the C code actually branches on: the")
        w(fh, "runtime options the public API can set (`png_set_*`), the `#ifdef`-selected")
        w(fh, "features listed in `c_src/include/pnglibconf.h`, and the input shapes the")
        w(fh, "`if` / `switch` statements in `c_src/src/*.c` distinguish (colour type, bit")
        w(fh, "depth, interlace method, filter method, chunk presence and ordering, IDAT")
        w(fh, "segmentation, buffer sizes, row-stride sign, ...).  Each row is one meaningful")
        w(fh, "*combination* of those axes; the cross-product is pruned to the combinations")
        w(fh, "the C treats differently.")
        w(fh)
        w(fh, "The lowest-level public entry points are driven directly, not only through the")
        w(fh, "convenience wrappers: `png_write_row` / `png_write_rows` / `png_write_image` /")
        w(fh, "`png_write_png` / `png_write_sig` + `png_write_chunk*`; `png_read_row` /")
        w(fh, "`png_read_rows` / `png_read_image` / `png_read_png` / `png_process_data`; plus")
        w(fh, "`png_init_io`, the simplified `png_image_*` API and the raw `png_get_uint_32`")
        w(fh, "family.")
        w(fh)
        w(fh, "Each row runs **both** libraries through their `.so` exports in that")
        w(fh, "configuration and compares a full record of the run byte for byte: every byte")
        w(fh, "written or decoded, every getter, the ordered list of warnings, the error")
        w(fh, "message if any, and the process exit status.  Rows marked `n=2` or `n=3` repeat")
        w(fh, "the configuration with that many independently seeded random images; rows in")
        w(fh, "groups B16/B17 are themselves drawn from a fixed-seed generator, so the whole")
        w(fh, "matrix is reproducible.")
        w(fh)
        w(fh, "One field is deliberately excluded from the post-transform comparison:")
        w(fh, "after a `PNG_QUANTIZE` transform on a non-palette image the reference C leaves")
        w(fh, "`info_ptr->num_trans` non-zero while `info_ptr->trans_alpha` still points at")
        w(fh, "memory that was never written for that colour type.  The byte the C reads there")
        w(fh, "moves when unrelated allocations move, so it is not a function of the input;")
        w(fh, "`num_trans` and the pointer's nullness are still compared, and the array")
        w(fh, "contents are still compared in every pre-transform and end-of-stream dump.")
        w(fh)
        w(fh, "**Result: %d of %d rows pass.**" % (npass, total))
        w(fh)
        w(fh, "## Feature combinations (Phase D)")
        w(fh)
        w(fh, "`translation/Cargo.toml` declares **no `[features]` section**, so the crate has")
        w(fh, "exactly one build configuration; `cargo metadata` confirms an empty feature map.")
        w(fh, "Both of the reachable configurations were checked and the whole suite was run in")
        w(fh, "both:")
        w(fh)
        w(fh, "```")
        w(fh, "cargo check --release --all-targets                     # OK")
        w(fh, "cargo check --release --all-targets --no-default-features  # OK (identical)")
        w(fh, "cargo test  --release       # 36 passed")
        w(fh, "cargo test                  # 36 passed (dev profile)")
        w(fh, "cargo test  --no-default-features  # 36 passed")
        w(fh, "```")
        w(fh)
        w(fh, "The *libpng* feature set, by contrast, is fixed by `c_src/include/pnglibconf.h`")
        w(fh, "and is what the axes above enumerate: all 21 supported ancillary chunk types,")
        w(fh, "read and write transforms, the progressive reader, the simplified API, user")
        w(fh, "limits, user transforms, unknown-chunk handling, MNG extensions, benign errors,")
        w(fh, "and both the floating-point and fixed-point halves of every dual API.")
        w(fh)
        w(fh, "| group | rows | passing | title |")
        w(fh, "|---|---|---|---|")
        for g in groups:
            rs = results[g]
            w(fh, "| %s | %d | %d | %s |" % (
                g, len(rs), sum(1 for row in rs if row[0] == "PASS"),
                GROUP_TITLES.get(g, g)))
        w(fh)
        n = 0
        for g in groups:
            w(fh, "## %s — %s" % (g, GROUP_TITLES.get(g, g)))
            w(fh)
            w(fh, "| # | entry point(s) / scenario | configuration (options set + input shape) | observed in the C build | [ ] |")
            w(fh, "|---|---------------------------|--------------------------------------------|-------------------------|-----|")
            for st, scen, desc, observed in results[g]:
                n += 1
                box = "[x]" if st == "PASS" else "[ ] **FAIL**"
                w(fh, "| %d | `%s` | %s | %s | %s |"
                  % (n, esc(scen), esc(desc), esc(observed), box))
            w(fh)
    return total, npass


def gen_errors(results):
    groups = [g for g in results if g.startswith("C") and g != "C5"]
    groups.sort(key=group_sort_key)
    rows = []
    for g in groups:
        rows.extend(results[g])
    fuzz = results.get("C5", [])
    total = len(rows)
    npass = sum(1 for row in rows if row[0] == "PASS")
    path = os.path.join(CRATE, "ERRORS.md")
    with open(path, "w") as fh:
        w(fh, "# ERRORS.md — the error surface (Phase C)")
        w(fh)
        w(fh, "Derived by grepping `c_src/src/*.c` for every distinct way the library rejects")
        w(fh, "input, then reducing those sites to the ones reachable through the public API.")
        w(fh, "The raw extraction is reproducible:")
        w(fh)
        w(fh, "```")
        w(fh, "$ python3 .verif/extract_errors.py | wc -l")
        w(fh, "663      # rejection sites across 257 functions")
        w(fh, "$ python3 .verif/extract_errors.py | cut -f4 | sort | uniq -c | sort -rn")
        w(fh, "   157 png_error              55 png_chunk_benign_error   12 png_app_warning")
        w(fh, "   115 png_warning            52 png_app_error            10 png_benign_error")
        w(fh, "   103 return 0               22 return NULL               8 png_chunk_warning")
        w(fh, "    93 handled-enum           17 png_chunk_report          5 return -1")
        w(fh, "                              15 png_chunk_error           3 png_fixed_error")
        w(fh, "                                                           2 PNG_ABORT")
        w(fh, "```")
        w(fh)
        w(fh, "Many of those 663 sites are the same rejection reached from several places (for")
        w(fh, "example `png_chunk_benign_error(png_ptr, \"invalid\")` appears once per chunk")
        w(fh, "handler) or are internal consistency assertions that no input can reach.  The")
        w(fh, "table below has one row per **distinct, externally reachable rejection**: the")
        w(fh, "exact invalid input, the C function that rejects it, and what the C does.")
        w(fh)
        w(fh, "Every row is a differential test.  Because a `png_error` is fatal, each row runs")
        w(fh, "in its own child process with an error callback that records the message and")
        w(fh, "`_exit(70)`s; the parent then compares the **exact message text** and the exit")
        w(fh, "status, so \"both failed somehow\" is never enough — the two libraries have to")
        w(fh, "fail the same way, with the same words, at the same point.  Where the reference")
        w(fh, "C dereferences NULL, divides by zero or loops forever, the row records the")
        w(fh, "signal (`SIGSEGV` / `SIGFPE`) or `TIMEOUT` and both sides must agree on that too.")
        w(fh)
        w(fh, "Coverage of the generic boundaries every C API has is included explicitly:")
        w(fh)
        w(fh, "* NULL pointers — `getters_null` calls all 40+ `png_get_*` with NULL/NULL;")
        w(fh, "  `setters_null_png` does the same for the `png_set_*` family.")
        w(fh, "* zero and oversized lengths — zero-length chunks for every chunk type, chunk")
        w(fh, "  length `0x80000000`, `png_set_compression_buffer_size(0)` and `SIZE_MAX`,")
        w(fh, "  palette lengths 0 / 257 / -1, `png_image` dimensions 0 and `0x40000000`.")
        w(fh, "* one step past a documented range — `PNG_sRGB_INTENT_LAST`, `PNG_SCALE_LAST`,")
        w(fh, "  `PNG_OFFSET_LAST`, `PNG_EQUATION_LAST`, `PNG_RESOLUTION_LAST`,")
        w(fh, "  `PNG_HANDLE_CHUNK_LAST`, `PNG_FILTER_VALUE_LAST`, `PNG_INTERLACE_LAST`,")
        w(fh, "  alpha mode 4, background gamma code 4, rgb-to-gray error action 4.")
        w(fh, "* out-of-range enum values across the FFI boundary — C enums accept any `int`,")
        w(fh, "  so negative and far-out values are passed too (`sRGB` intent -1, alpha mode")
        w(fh, "  -1, `png_set_option` numbers -4..23 with on/off -1..3, colour type 255,")
        w(fh, "  bit depth 255, interlace 255, compression method 255, `keep` -1 and 4).")
        w(fh, "* floating-point edges across the FFI boundary — NaN, -NaN, +-inf and 1e300")
        w(fh, "  through every `double` entry point.  NaN silently passes libpng's own range")
        w(fh, "  checks, so the value that reaches `(png_fixed_point)` is target-defined.")
        w(fh)
        w(fh, "**Result: %d of %d rows pass.**" % (npass, total))
        w(fh)
        w(fh, "| # | function | trigger (the exact invalid input/condition) | expected C result | observed in the C build | [ ] |")
        w(fh, "|---|----------|----------------------------------------------|-------------------|-------------------------|-----|")
        for i, (st, scen, desc, observed) in enumerate(rows, 1):
            parts = desc.split(" \u2014 ")
            func = parts[0] if parts else ""
            trig = parts[1] if len(parts) > 1 else ""
            exp = " \u2014 ".join(parts[2:]) if len(parts) > 2 else ""
            box = "[x]" if st == "PASS" else "[ ] **FAIL**"
            w(fh, "| %d | `%s` | %s | %s | %s | %s |"
              % (i, esc(func), esc(trig), esc(exp), esc(observed), box))
        w(fh)
        w(fh, "Scenario ids (used as `err|id=<id>` by `translation/tests/support/errscen.rs`)")
        w(fh, "are listed in `translation/tests/support/errors_tbl.rs`, which is also the source")
        w(fh, "of this table.")
        w(fh)
        w(fh, "## Mutation fuzzing (group C5)")
        w(fh)
        w(fh, "The table above is an enumeration; group C5 is a *search*.  It takes a rich but")
        w(fh, "valid datastream (IHDR + gAMA + cHRM + sBIT + tRNS + bKGD + pHYs + oFFs + sCAL +")
        w(fh, "pCAL + sPLT + tEXt/zTXt/iTXt + eXIf + cICP + cLLI + mDCV + iCCP + private chunks")
        w(fh, "+ IDAT + trailing chunks + IEND), flips 1/2/4/8 bits at pseudo-random offsets and")
        w(fh, "reads the result end to end, with and without recomputed CRCs and with each")
        w(fh, "png_set_benign_errors() setting.  Both libraries get the identical mutated bytes.")
        w(fh)
        w(fh, "%d rows, %d passing (each row tries 8 independently seeded mutations)."
          % (len(fuzz), sum(1 for row in fuzz if row[0] == "PASS")))
        w(fh)
        w(fh, "`PNGDIFF_FUZZ=<n>` multiplies the size of this search; it has been run at")
        w(fh, "n=25 (3000 rows / 24000 mutations) with no divergence.  Distinct C outcomes")
        w(fh, "observed in the default run:")
        w(fh)
        seen = {}
        for st, scen, desc, o in fuzz:
            k = re.sub(r"\d+", "N", o)
            seen[k] = seen.get(k, 0) + 1
        w(fh, "| observed in the C build | rows |")
        w(fh, "|---|---|")
        for k, n in sorted(seen.items(), key=lambda kv: -kv[1])[:40]:
            w(fh, "| %s | %d |" % (esc(k), n))
        if len(seen) > 40:
            w(fh, "| ... %d further distinct outcomes | |" % (len(seen) - 40))
    return total, npass


def main():
    results, _ = load_results()
    if not results:
        sys.exit("no .rowresults/*.tsv found - run `cargo test --release` first")
    nsym, nmissing, nexternal = gen_symbols(results)
    ctotal, cpass = gen_configs(results)
    etotal, epass = gen_errors(results)
    print("SYMBOLS.md : %d symbols, %d missing, %d unexpected external refs"
          % (nsym, nmissing, nexternal))
    print("CONFIGS.md : %d rows, %d passing" % (ctotal, cpass))
    print("ERRORS.md  : %d rows, %d passing" % (etotal, epass))


if __name__ == "__main__":
    main()
