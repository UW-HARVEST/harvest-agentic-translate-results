import subprocess, sys, shutil, os
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig'
shutil.copy(SRC,BAK)
mutations = [
 ("utf16 surrogate low bound", "if second_code < 0xDC00 || second_code > 0xDFFF {", "if second_code < 0xDC01 || second_code > 0xDFFF {"),
 ("utf16 codepoint base", "0x10000u64 + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);", "0x10000u64 + ((((first_code & 0x3FF) << 10) | (second_code & 0x1FF)) as u64);"),
 ("utf8 3-byte threshold", "} else if codepoint < 0x10000 {", "} else if codepoint < 0xFFFF {"),
 ("utf8 2-byte threshold", "} else if codepoint < 0x800 {", "} else if codepoint < 0x7FF {"),
 ("first_code high surrogate upper", "if first_code >= 0xD800 && first_code <= 0xDBFF {", "if first_code >= 0xD800 && first_code <= 0xDBFE {"),
 ("continuation byte mask", "((codepoint | 0x80) & 0xBF) as u8;", "((codepoint | 0x80) & 0x7F) as u8;"),
]
results=[]
for name, old, new in mutations:
    s=open(BAK).read()
    if old not in s:
        results.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    detected = 'FAILED' in p.stdout or 'FAILED' in p.stderr
    results.append((name,'detected' if detected else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in results: print(f"{r:20s} {n}")
