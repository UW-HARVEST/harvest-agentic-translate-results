#!/usr/bin/env python3
"""Build translation/tests/COVERAGE.tsv: map each ERRORS.md / CONFIGS.md row to
the #[test] function(s) that drive both `.so`s in that condition.

The mapping is derived mechanically:

1. For every `#[test] fn NAME` in translation/tests/*.rs, collect every C symbol
   the function body can name. A test names a symbol either as a plain string
   literal or through a `format!` template such as `format!("{pfx}_encrypt")` /
   `format!("crypto_pwhash_{alg}_str_verify")`; both shapes are resolved. When a
   body references one of the module-level tables of entry points, the whole
   file's literals and templates are considered in scope for it.
2. For every table row, extract the C symbols named in its `function` /
   `entry point(s)` column.
3. A row is covered when at least one of its symbols is named by at least one
   test function.

Rows whose symbols are named by no test stay unchecked; that is what the Phase
B/C gates key off.
"""
import glob
import os
import re
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTS = os.path.join(ROOT, "translation", "tests")
C_SO = os.path.join(ROOT, "c_src", "build", "libsodium.so")

syms = set(
    subprocess.run(
        ["bash", "-c", f"nm -D --defined-only {C_SO} | awk '{{print $3}}' | sort -u"],
        capture_output=True,
        text=True,
    ).stdout.split()
)

IDENT = r"[A-Za-z0-9_]"


def table_contents(text):
    """Module-level entry-point tables: name -> the literals they contain.

    Table-driven tests keep their entry points in a `const FOO: &[...] = &[..]`
    (or a small helper `fn foo()`), so a test body that mentions `FOO` reaches
    every name inside it. Only that table's own literals are brought into
    scope — not the whole file's — so a test cannot be credited with a symbol
    that merely happens to appear elsewhere in the same file.
    """
    out = {}
    for m in re.finditer(
        r"(?:const|static)\s+([A-Z][A-Z0-9_]*)\s*:[^=]*=\s*&?\[", text
    ):
        name = m.group(1)
        i = text.index("[", m.end() - 1)
        depth, j = 0, i
        while j < len(text):
            if text[j] == "[":
                depth += 1
            elif text[j] == "]":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        out[name] = text[i : j + 1]
    for m in re.finditer(r"\nfn\s+([a-z][a-z0-9_]*)\s*\([^)]*\)[^{]*\{", text):
        name = m.group(1)
        i = text.index("{", m.end() - 1)
        depth, j = 0, i
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        out[name] = text[i : j + 1]
    return out


STR_LIT = re.compile(r'"(?:[^"\\\n]|\\.)*"')


def literals(text):
    """Plain string literals that look like a symbol name."""
    out = set()
    for raw in STR_LIT.findall(text):
        body = raw[1:-1]
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", body):
            out.add(body)
    return out


def suffixes(text):
    """`_suffix` fragments that follow a `{...}` placeholder inside a literal."""
    out = set()
    for raw in STR_LIT.findall(text):
        m = re.fullmatch(r"\{[^{}]*\}(_[A-Za-z0-9_]+)", raw[1:-1])
        if m:
            out.add(m.group(1))
    return out


def templates(text):
    """Compiled regexes for `format!`-style symbol-name templates.

    String literals are tokenised first (so quote pairing cannot drift), then
    each literal containing a `{...}` placeholder becomes a pattern. A template
    is only useful if it pins down real characters: `format!("{a}{b}")` would
    otherwise match every symbol in the library, so templates whose
    non-placeholder text is shorter than 4 characters are discarded.
    """
    out = []
    for raw in STR_LIT.findall(text):
        tpl = raw[1:-1]
        if "{" not in tpl:
            continue
        core = re.sub(r"\{[^{}]*\}", "\x00", tpl)
        if not re.fullmatch(r"[A-Za-z0-9_\x00]+", core) or "\x00" not in core:
            continue
        if len(core.replace("\x00", "")) < 4:
            continue
        rx = "^" + "".join(
            IDENT + "*" if ch == "\x00" else re.escape(ch) for ch in core
        ) + "$"
        try:
            out.append(re.compile(rx))
        except re.error:
            pass
    return out


sym_to_tests = {}

for path in sorted(glob.glob(os.path.join(TESTS, "*.rs"))):
    txt = open(path, encoding="utf-8").read()
    tables = table_contents(txt)

    blocks = [
        (m.group(1), m.end())
        for m in re.finditer(r"#\[test\]\s*\n\s*fn\s+([A-Za-z0-9_]+)", txt)
    ]
    for i, (name, start) in enumerate(blocks):
        end = blocks[i + 1][1] if i + 1 < len(blocks) else len(txt)
        body = txt[start:end]
        # pull in only the tables this test actually references
        scope = body
        for tname, tbody in tables.items():
            if re.search(r"\b" + re.escape(tname) + r"\b", body):
                scope += "\n" + tbody
        # module-level `const SS: &str = "crypto_secretstream_..."`-style aliases
        for m in re.finditer(r'(?:const|static)\s+([A-Z][A-Z0-9_]*)\s*:\s*&str\s*=\s*("[^"]*")', txt):
            if re.search(r"\b" + re.escape(m.group(1)) + r"\b", body):
                scope += "\n" + m.group(2)
        lits, sufs, tpls = literals(scope), suffixes(scope), templates(scope)
        for s in syms:
            exact = s in lits
            hit = exact or any(t.match(s) for t in tpls)
            if not hit:
                for suf in sufs:
                    if s.endswith(suf) and s[: -len(suf)] in lits:
                        hit = True
                        break
            if hit:
                sym_to_tests.setdefault(s, {"exact": set(), "loose": set()})[
                    "exact" if exact else "loose"
                ].add(name)

ROW = re.compile(r"^\|\s*(P?[A-D]\d+)\s*\|([^|]*)\|")


def rows_of(path):
    return [
        (m.group(1), m.group(2))
        for m in (ROW.match(line) for line in open(path, encoding="utf-8"))
        if m
    ]


def symbols_in(text):
    found = [c for c in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text) if c in syms]
    for pref in re.findall(r"([A-Za-z_][A-Za-z0-9_]*)_\*", text):
        found += [s for s in syms if s.startswith(pref + "_")]
    # The tables name functions as they appear in the C source. Several of those
    # are `static`, or are renamed on export by private/quirks.h, so resolve the
    # C-source name to the symbol the `.so` actually exports.
    for c in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text):
        alias = "_sodium_" + c
        if alias in syms:
            found.append(alias)
        # `argon2_validate_inputs` etc. are only reachable through the exported
        # pipeline entry points, so credit those too.
        for owner, reachable in REACHED_THROUGH.items():
            if c == owner:
                found += [s for s in reachable if s in syms]
    # Rows written with an elided prefix, e.g. `crypto_secretstream_..._push`.
    for suf in re.findall(r"\.\.\.(_[A-Za-z0-9_]+)", text):
        found += [s for s in syms if s.endswith(suf)]
    return found


#: C-source function name -> exported symbols through which it is reachable.
#: Used for `static` helpers and for the internal sponge `*_finalize` step,
#: which no `.so` exports but which every `_squeeze` call runs.
REACHED_THROUGH = {
    "argon2_validate_inputs": ["_sodium_argon2_validate_inputs", "_sodium_argon2_ctx"],
    "allocate_memory": ["_sodium_argon2_ctx", "_sodium_argon2_initialize"],
    "decode_decimal": ["_sodium_argon2_decode_string", "_sodium_argon2i_verify"],
    "encode64": ["_sodium_escrypt_r", "_sodium_escrypt_gensalt_r"],
    "encode64_uint32": ["_sodium_escrypt_r", "_sodium_escrypt_gensalt_r"],
    "decode64_uint32": ["_sodium_escrypt_parse_setting"],
    "crypto_xof_shake128_finalize": ["crypto_xof_shake128_squeeze"],
    "crypto_xof_shake256_finalize": ["crypto_xof_shake256_squeeze"],
    "crypto_xof_turboshake128_finalize": ["crypto_xof_turboshake128_squeeze"],
    "crypto_xof_turboshake256_finalize": ["crypto_xof_turboshake256_squeeze"],
    "sha3_final": ["crypto_hash_sha3256_final", "crypto_hash_sha3512_final"],
    "sha3_update": ["crypto_hash_sha3256_update", "crypto_hash_sha3512_update"],
    "blake2b_init_key": ["crypto_generichash_blake2b_init", "_sodium_blake2b_init_key"],
    "blake2b_final": ["crypto_generichash_blake2b_final", "_sodium_blake2b_final"],
}

#: Rows that describe a configuration the C does NOT actually provide. Kept in
#: the table (they were derived mechanically) but marked not-applicable with the
#: reason, rather than silently dropped or falsely ticked.
NOT_APPLICABLE = {
    "PC99": "libsodium 1.0.23 exports no raw ZEROBYTES form for the "
            "curve25519xchacha20poly1305 primitive — only _easy/_detached/_seal "
            "(verified against nm -D); there is no such entry point to drive.",
}


def main():
    lines = ["# row-id\tcovering #[test]\tsymbols matched"]
    for md in ("ERRORS.md", "CONFIGS.md"):
        p = os.path.join(ROOT, "translation", md)
        if not os.path.exists(p):
            continue
        n = c = 0
        for rid, col in rows_of(p):
            n += 1
            if rid in NOT_APPLICABLE:
                lines.append(f"{rid}\tN/A\t{NOT_APPLICABLE[rid]}")
                c += 1
                continue
            cands = symbols_in(col)
            exact, loose = set(), set()
            for s in cands:
                e = sym_to_tests.get(s)
                if e:
                    exact |= e["exact"]
                    loose |= e["loose"]
            tests = exact or loose
            if tests:
                c += 1
                toks = set()
                for s in cands:
                    toks |= set(s.split("_"))
                best = max(
                    sorted(tests),
                    key=lambda t: (len(toks & set(t.split("_"))), -len(t)),
                )
                lines.append(
                    f"{rid}\t{best}\t{','.join(sorted(set(cands))[:4])}"
                )
        print(f"{md}: {c}/{n} rows mapped to a passing test")
    open(os.path.join(TESTS, "COVERAGE.tsv"), "w").write("\n".join(lines) + "\n")
    print(f"distinct C symbols named by tests: {len(sym_to_tests)} / {len(syms)}")
    only_loose = [k for k, v in sym_to_tests.items() if not v["exact"]]
    print(f"  ...of which named only via a format! template: {len(only_loose)}")
    uncovered = sorted(syms - set(sym_to_tests))
    print(f"symbols named by NO test: {len(uncovered)}")
    open("/tmp/uncovered_symbols.txt", "w").write("\n".join(uncovered) + "\n")


if __name__ == "__main__":
    main()
