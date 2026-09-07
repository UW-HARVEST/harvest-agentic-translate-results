import subprocess, shutil, sys
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig4'
shutil.copy(SRC,BAK)
M = [
 ("skip_utf8_bom: can_access 4 -> 3", "if can_access_at_index(buffer, 4)", "if can_access_at_index(buffer, 3)"),
 ("print_string_ptr: empty ensure 3 -> 2", "let out = ensure(output_buffer, 3 /* sizeof(\"\\\"\\\"\") */);", "let out = ensure(output_buffer, 2 /* sizeof(\"\\\"\\\"\") */);"),
 ("ensure: offset >= length -> >", "if ((*p).length > 0) && ((*p).offset >= (*p).length) {", "if ((*p).length > 0) && ((*p).offset > (*p).length) {"),
 ("print_number: length limit off by one", "if (length < 0) || (length > (core::mem::size_of::<[u8; 26]>() - 1) as c_int) {", "if (length < 0) || (length >= (core::mem::size_of::<[u8; 26]>() - 1) as c_int) {"),
 ("parse_number: >= INT_MAX -> >", "if number >= INT_MAX as f64 {\n        (*item).valueint = INT_MAX;", "if number > INT_MAX as f64 {\n        (*item).valueint = INT_MAX;"),
 ("parse_number: <= INT_MIN -> <", "} else if number <= INT_MIN as f64 {\n        (*item).valueint = INT_MIN;", "} else if number < INT_MIN as f64 {\n        (*item).valueint = INT_MIN;"),
]
res=[]
for name, old, new in M:
    s=open(BAK).read()
    if old not in s:
        res.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    res.append((name,'detected' if p.returncode!=0 else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in res: print(f"{r:20s} {n}")
