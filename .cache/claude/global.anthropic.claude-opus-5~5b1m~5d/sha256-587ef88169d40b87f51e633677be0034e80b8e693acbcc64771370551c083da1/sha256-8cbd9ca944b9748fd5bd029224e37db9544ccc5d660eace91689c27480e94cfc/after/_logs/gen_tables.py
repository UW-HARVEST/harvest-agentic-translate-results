import re,os,glob
L="$HARVEST_WORKDIR/_logs"
T="$HARVEST_WORKDIR/translation"
groups=[("G1","sodium/utils.c, codecs.c, core.c, runtime.c, version.c; crypto_verify/"),
        ("G2","crypto_pwhash/ (argon2 + argon2-encoding + scryptsalsa208sha256 + pbkdf2)"),
        ("G3","crypto_generichash/, crypto_shorthash/, crypto_onetimeauth/, crypto_auth/, crypto_kdf/, crypto_hash/, crypto_xof/, crypto_core/keccak1600"),
        ("G4","crypto_aead/, crypto_secretbox/, crypto_secretstream/, crypto_stream/, crypto_core/{salsa,hsalsa20,hchacha20}"),
        ("G5","crypto_box/, crypto_kx/, crypto_scalarmult/, crypto_sign/, crypto_core/{ed25519,ristretto255}"),
        ("G6","randombytes/, crypto_kem/, crypto_ipcrypt/")]
def rows(path):
    return [l.strip() for l in open(path) if l.strip().startswith("|") and l.count("|")>=4]

# ---------- row -> tests attribution ----------
attr={"CONFIGS":{}, "ERRORS":{}}
def add(kind,rs,label):
    for r in rs: attr[kind].setdefault(r,set()).add(label)
def parse_rows(kind,text):
    out=set()
    for m in re.finditer(kind+r"(?:\.md)?\s*(?:rows?)?\s*((?:\d+\s*(?:-|–|to)\s*\d+|\d+)(?:\s*(?:,|/|and|\+)\s*(?:\d+\s*(?:-|–|to)\s*\d+|\d+))*)", text, re.I):
        for part in re.split(r"[,/]|\band\b|\+", m.group(1)):
            part=part.strip()
            mm=re.fullmatch(r"(\d+)\s*(?:-|–|to)\s*(\d+)", part)
            if mm:
                a,b=int(mm.group(1)),int(mm.group(2))
                if a<=b and b-a<1200: out.update(range(a,b+1))
            elif part.isdigit(): out.add(int(part))
    return out
FILE_RANGE = {
    "t01_g1_utils":        ("CONFIGS", 1, 163),
    "t13_g1_extra":        ("CONFIGS", 1, 163),
    "t02_g1_errors":       ("ERRORS",  1, 99),
    "t11_g2_pwhash":       ("CONFIGS", 164, 280),
    "t12_g2_pwhash_errors":("ERRORS",  100, 281),
    "t03_g3_hash":         ("CONFIGS", 281, 509),
    "t04_g3_hash_errors":  ("ERRORS",  282, 404),
    "t05_g4_aead":         ("CONFIGS", 510, 716),
    "t06_g4_aead_errors":  ("ERRORS",  405, 508),
    "t07_g5_asym":         ("CONFIGS", 717, 888),
    "t08_g5_asym_errors":  ("ERRORS",  509, 638),
    "t09_g6_rand":         ("CONFIGS", 889, 995),
    "t10_g6_rand_errors":  ("ERRORS",  639, 739),
}

def bare_rows(text):
    """rows given as a bare list like `| 46-48, 52-54 | ...` in a ROW MAP table"""
    out=set()
    for part in re.split(r"[,/]", text):
        part=part.strip()
        mm=re.fullmatch(r"(\d+)\s*(?:-|–)\s*(\d+)", part)
        if mm:
            a,b=int(mm.group(1)),int(mm.group(2))
            if a<=b and b-a<1200: out.update(range(a,b+1))
        elif part.isdigit(): out.add(int(part))
    return out

os.chdir(T)
for f in sorted(glob.glob("tests/*.rs")):
    base=os.path.basename(f)[:-3]
    text=open(f).read()
    lines=text.split("\n")
    # (a) explicit ROW MAP tables in the file header: `//! | <rows> | <labels> |`
    kind_hint = None
    for l in lines:
        if l.strip().startswith("//!") and "ROW MAP" in l:
            kind_hint = "CONFIGS" if "CONFIGS" in l else ("ERRORS" if "ERRORS" in l else None)
        m=re.match(r"\s*//!\s*\|\s*([0-9,\-– ]+?)\s*\|\s*(.+?)\s*\|\s*$", l)
        if m and kind_hint:
            rs=bare_rows(m.group(1))
            labels=re.findall(r"`([\w:.]+)`", m.group(2))
            if rs and labels:
                for lab in labels:
                    lab=lab.replace("t01::","t01_g1_utils::").replace("t02::","t02_g1_errors::").replace("t13::","t13_g1_extra::").replace("t00::","t00_smoke::")
                    add(kind_hint, rs, lab)
    # (b) per-#[test] comment/body scan
    tests=[]
    for i,l in enumerate(lines):
        if l.strip()=="#[test]":
            for j in range(i+1,min(i+6,len(lines))):
                mm=re.match(r"\s*fn (\w+)", lines[j])
                if mm: tests.append((i,j,mm.group(1))); break
    for k,(ti,fj,name) in enumerate(tests):
        st=ti
        while st>0 and (lines[st-1].lstrip().startswith("//") or lines[st-1].strip()==""):
            st-=1
        end = tests[k+1][0] if k+1<len(tests) else len(lines)
        blob="\n".join(lines[st:end])
        for kind in ("CONFIGS","ERRORS"):
            add(kind, parse_rows(kind,blob), f"{base}::{name}")
    # (c) row numbers embedded in the #[test] fn NAME (the convention several
    #     suites use, e.g. `g1_err_rows_2_5_97_99_comparison_family`,
    #     `e484_492_500_501_size_max`, `c714_716_core_hchacha20`). Only digits
    #     that fall inside this FILE's own row range are accepted, which rules
    #     out incidental numbers such as lengths or primitive names.
    rng = FILE_RANGE.get(base)
    if rng:
        kind, lo, hi = rng
        for (_ti,_fj,name) in tests:
            nums = {int(x) for x in re.findall(r"\d+", name)}
            hits = {x for x in nums if lo <= x <= hi}
            if hits:
                add(kind, hits, f"{base}::{name}")
    # (d) file-level fallback: rows named anywhere in the first 60 header lines
    head="\n".join(lines[:60])
    for kind in ("CONFIGS","ERRORS"):
        add(kind, parse_rows(kind,head), f"{base}.rs")

def cell(kind,n):
    ts=sorted(attr[kind].get(n,()))
    # prefer specific test fns over the whole-file fallback
    fns=[t for t in ts if "::" in t]
    ts = fns if fns else ts
    return ", ".join(f"`{t}`" for t in ts) if ts else "**UNCOVERED**"

# ---------------- ERRORS.md ----------------
hdr = open(L+"/errors_header.md").read()
n=0; body=[]; counts={}
for g,desc in groups:
    rs=rows(f"{L}/frag/{g}_errors.md"); counts[g]=len(rs)
    for r in rs:
        parts=[p.strip() for p in r.strip("|").split("|")]
        while len(parts)<4: parts.append("")
        n+=1
        body.append(f"| {n} | `{parts[1]}` | {parts[2]} | {parts[3]} | {cell('ERRORS',n)} |")
open(f"{T}/ERRORS.md","w").write(hdr+"\n".join(body)+
  "\n\n## Rows per module group\n\n| group | modules | rows | error-path test file |\n|---|---|---|---|\n"+
  "\n".join(f"| {g} | {d} | {counts[g]} | `{tf}` |" for (g,d),tf in zip(groups,
    ["tests/t02_g1_errors.rs","tests/t12_g2_pwhash_errors.rs","tests/t04_g3_hash_errors.rs",
     "tests/t06_g4_aead_errors.rs","tests/t08_g5_asym_errors.rs","tests/t10_g6_rand_errors.rs"]))+
  f"\n| **total** | | **{n}** | |\n")
print("ERRORS.md rows:",n,"uncovered:",sum(1 for i in range(1,n+1) if cell('ERRORS',i)=='**UNCOVERED**'))

# ---------------- CONFIGS.md ----------------
hdr2 = open(L+"/configs_header.md").read()
n2=0; body2=[]; counts2={}
for g,desc in groups:
    rs=rows(f"{L}/frag/{g}_configs.md"); counts2[g]=len(rs)
    for r in rs:
        parts=[p.strip() for p in r.strip("|").split("|")]
        while len(parts)<4: parts.append("[ ]")
        n2+=1
        c=cell('CONFIGS',n2)
        body2.append(f"| {n2} | {parts[1]} | {parts[2]} | [{'x' if c!='**UNCOVERED**' else ' '}] | {c} |")
open(f"{T}/CONFIGS.md","w").write(hdr2+"\n".join(body2)+
  "\n\n## Rows per module group\n\n| group | modules | rows | valid-path test file |\n|---|---|---|---|\n"+
  "\n".join(f"| {g} | {d} | {counts2[g]} | `{tf}` |" for (g,d),tf in zip(groups,
    ["tests/t01_g1_utils.rs + tests/t13_g1_extra.rs","tests/t11_g2_pwhash.rs","tests/t03_g3_hash.rs",
     "tests/t05_g4_aead.rs","tests/t07_g5_asym.rs","tests/t09_g6_rand.rs"]))+
  f"\n| **total** | | **{n2}** | |\n")
print("CONFIGS.md rows:",n2,"uncovered:",sum(1 for i in range(1,n2+1) if cell('CONFIGS',i)=='**UNCOVERED**'))
