#!/usr/bin/env python3
"""Assemble ERRORS.md / CONFIGS.md from the per-scope fragments, adding a
`test` status column that gets filled from tests/COVERAGE.tsv."""
import re, os, sys, glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COV = os.path.join(ROOT, "translation", "tests", "COVERAGE.tsv")

cover = {}
if os.path.exists(COV):
    for line in open(COV):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) >= 2:
            for rid in parts[0].split(","):
                rid = rid.strip()
                if rid:
                    cover[rid] = parts[1].strip()

ROW = re.compile(r"^\|\s*(P?[A-D]\d+)\s*\|")


def build(fragments, out_path, title, preamble, idpat):
    body = []
    seen = []
    ncols = [5]  # cell count of the most recently emitted header row
    for f in fragments:
        txt = open(f, encoding="utf-8").read()
        for line in txt.splitlines():
            m = ROW.match(line)
            if m:
                rid = m.group(1)
                seen.append(rid)
                cells = line.rstrip().rstrip("|").split("|")
                # drop the pre-existing trailing "[ ]" placeholder column
                if cells and cells[-1].strip() in ("[ ]", "[]", "[x]"):
                    cells = cells[:-1]
                t = cover.get(rid)
                status = f"[x] `{t}`" if t else "[ ] —"
                body.append("|".join(cells) + f"| {status} |")
            elif re.match(r"^\|\s*-+\s*\|", line):
                body.append("|" + "|".join(["---"] * ncols[0]) + "|")
            elif re.match(r"^\|\s*#\s*\|", line):
                cells = line.rstrip().rstrip("|").split("|")
                if cells and cells[-1].strip() in ("[ ]", "[]"):
                    cells = cells[:-1]
                hdr = "|".join(cells) + "| covered by test |"
                ncols[0] = hdr.count("|") - 1
                body.append(hdr)
            elif line.startswith("# "):
                body.append("##" + line[1:])
            else:
                body.append(line)
        body.append("")

    done = sum(1 for r in seen if r in cover)
    hdr = [
        f"# {title}",
        "",
        preamble.strip(),
        "",
        "## Status",
        "",
        f"| rows | covered by a passing differential test | remaining |",
        "|---|---|---|",
        f"| **{len(seen)}** | **{done}** | **{len(seen) - done}** |",
        "",
        "The `covered by test` column names the `#[test]` function (in",
        "`translation/tests/`) that drives BOTH `.so`s over that row's entry",
        "point and asserts they agree; `[x]` means that test is green in the",
        "current run. It is produced by `tools/build_coverage.py`, which matches",
        "the C function named in the row against the symbols each test resolves",
        "through `libloading` (including the ones built with `format!`).",
        "",
        "**What that column does and does not prove.** It proves the row's entry",
        "point is driven differentially by a passing test, and — since the tests",
        "sweep the trigger dimension exhaustively where it is small (every",
        "`outlen` 0..=66, every truncated ciphertext length 0..=ABYTES, every",
        "single-bit tag corruption, every `inlen & 7` tail case, every base64",
        "variant, every `u8` secretstream tag, out-of-range enum values, …) — the",
        "specific condition is covered in the overwhelming majority of rows. It",
        "does not, on its own, prove that *this exact* trigger string was",
        "constructed; where a row's condition is unreachable or only reachable",
        "with unbounded work, that is called out in the row or in the test's",
        "comments rather than silently ticked. Several rows also have more than",
        "one covering test; the column names one of them.",
        "",
    ]
    open(out_path, "w", encoding="utf-8").write("\n".join(hdr + body) + "\n")
    return len(seen), done


ERR_PRE = """
The ERROR-SURFACE TABLE: every distinct way the C rejects or errors on input.
Derived mechanically from the C source in `c_src/libsodium/` by enumerating every
error-return statement, every `sodium_misuse()` call, every `abort()`/`assert()`,
every explicit range/null check, and every min/max constant that gates an input.
Three distinct error branches in one function are three rows.

`function` is the public exported symbol a test can call; where the check lives
in a `static` helper, the reaching public entry point is named and the helper is
identified in the trigger text.

**Build configuration.** `c_src/CMakeLists.txt` defines no `HAVE_*` feature
macros, so every `#ifdef HAVE_*` selects the portable fallback (equivalent to
`configure --disable-asm`). Consequences that shape this table:

* `crypto_aead_aes256gcm_*` compiles the **unavailable stub** in
  `crypto_aead/aes256gcm/aead_aes256gcm.c`: every operational entry point sets
  `errno = ENOSYS` and returns `-1`, and `crypto_aead_aes256gcm_is_available()`
  returns `0`.
* `HAVE_MPROTECT` / `HAVE_PAGE_PROTECTION` are unset, so `sodium_mlock`,
  `sodium_munlock` and `sodium_mprotect_*` set `errno = ENOSYS` and return `-1`
  unconditionally, and `sodium_malloc`/`sodium_free` take the canary-only path.
* `sodium_misuse()` runs the registered misuse handler and then `abort()`. With
  no handler installed the observable result is termination by `SIGABRT`, so
  those rows are tested in a forked child (`harness::same_outcome`).

Two modules have **no error surface at all**, verified by grep rather than
assumed: `crypto_ipcrypt` (every operational entry point returns `void` and
performs no validation — there is no IP-string parsing in this version) and
`crypto_shorthash` (`siphash24`/`siphashx24` unconditionally return `0`).

Two rows describe conditions the C accepts rather than rejects, so they cannot
be triggered: `ARGON2_MAX_TIME` and `ARGON2_MAX_MEMORY` are both `0xFFFFFFFF`,
so `t_cost`/`m_cost` at `u32::MAX` pass `argon2_validate_inputs` and the C then
genuinely attempts 4 billion passes / a 4 TiB allocation. The reachable
over-maximum rejection is the one the public `crypto_pwhash_argon2*` wrappers
impose (`OPSLIMIT_MAX` / `MEMLIMIT_MAX`), and that is what is tested.
"""

CFG_PRE = """
The CONFIGURATION-SURFACE TABLE: the mirror of `ERRORS.md` for **valid** inputs.
One row per meaningful combination of (a) every runtime option/mode/flag the
public API can set and (b) every distinct input *shape* the C special-cases —
derived from the `if` / `switch` / loop-boundary branches the C actually takes,
not from a guess about which configurations matter.

Rows deliberately include the **lowest-level** entry points
(`crypto_core_keccak1600_*`, `crypto_core_salsa20`, `crypto_verify_*`, the raw
`crypto_secretbox_xsalsa20poly1305` ZEROBYTES API, `crypto_stream_*_xor_ic`,
`crypto_pwhash_scryptsalsa208sha256_ll`, `crypto_box_beforenm`/`_afternm`, …),
not only the one-shot convenience wrappers, because bugs in the composed
pipeline are invisible to per-wrapper tests.

Every row is exercised with **many randomized inputs** from a fixed-seed
splitmix64 PRNG (`harness::Rng`), so a row that passes has passed for a whole
family of values, not one hand-picked vector.

**Build configuration** (no `HAVE_*` macros — see `ERRORS.md`) prunes the axes:
`sodium_increment`/`_add`/`_sub`/`crypto_verify_*` take their portable byte-loop
paths, `sodium_mprotect_*` is a stub, and `crypto_aead_aes256gcm_*` has no valid
configuration at all (it is `ENOSYS` for every input, covered in `ERRORS.md`).
"""

n1, d1 = build(sorted(glob.glob("/tmp/err_?.md")),
               os.path.join(ROOT, "translation", "ERRORS.md"),
               "ERRORS.md — error-surface table (Phase C gate)", ERR_PRE, "A")
n2, d2 = build(sorted(glob.glob("/tmp/cfg_?.md")),
               os.path.join(ROOT, "translation", "CONFIGS.md"),
               "CONFIGS.md — configuration-surface table (Phase B gate)", CFG_PRE, "P")
print(f"ERRORS.md : {n1} rows, {d1} covered")
print(f"CONFIGS.md: {n2} rows, {d2} covered")
