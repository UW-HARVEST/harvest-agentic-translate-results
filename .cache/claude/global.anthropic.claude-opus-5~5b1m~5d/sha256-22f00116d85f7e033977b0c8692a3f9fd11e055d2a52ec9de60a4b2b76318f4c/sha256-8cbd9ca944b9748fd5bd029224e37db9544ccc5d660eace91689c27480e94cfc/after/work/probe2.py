import subprocess, shutil
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig2'
shutil.copy(SRC,BAK)
M = [
 ("ensure: needed+offset+1 -> +0", "needed = needed.wrapping_add((*p).offset).wrapping_add(1);", "needed = needed.wrapping_add((*p).offset);"),
 ("ensure: needed <= length -> <", "if needed <= (*p).length {\n        return (*p).buffer.wrapping_add((*p).offset);", "if needed < (*p).length {\n        return (*p).buffer.wrapping_add((*p).offset);"),
 ("ensure: offset >= length -> >", "if ((*p).length > 0) && ((*p).offset >= (*p).length) {", "if ((*p).length > 0) && ((*p).offset > (*p).length) {"),
 ("can_read: <= -> <", "!buffer.is_null() && ((*buffer).offset.wrapping_add(size) <= (*buffer).length)", "!buffer.is_null() && ((*buffer).offset.wrapping_add(size) < (*buffer).length)"),
 ("can_access_at_index: < -> <=", "!buffer.is_null() && ((*buffer).offset.wrapping_add(index) < (*buffer).length)", "!buffer.is_null() && ((*buffer).offset.wrapping_add(index) <= (*buffer).length)"),
 ("compare_double: <= -> <", "if (a - b).abs() <= max_val * f64::EPSILON {", "if (a - b).abs() < max_val * f64::EPSILON {"),
 ("compare_double: EPSILON -> 2*EPSILON", "if (a - b).abs() <= max_val * f64::EPSILON {", "if (a - b).abs() <= max_val * 2.0 * f64::EPSILON {"),
 ("print_number: length limit off by one", "if (length < 0) || (length > (core::mem::size_of::<[u8; 26]>() - 1) as c_int) {", "if (length < 0) || (length >= (core::mem::size_of::<[u8; 26]>() - 1) as c_int) {"),
 ("print_number: %1.17g -> %1.18g", 'length = sprintf(nb, cs!(b"%1.17g\\0"), d);', 'length = sprintf(nb, cs!(b"%1.18g\\0"), d);'),
 ("print_number: drop sscanf recheck", "if sscanf(nb as *const c_char, cs!(b\"%lg\\0\"), &mut test as *mut f64) != 1\n            || compare_double(test, d) == 0", "if false"),
 ("parse_number: >= INT_MAX -> >", "if number >= INT_MAX as f64 {\n        (*item).valueint = INT_MAX;", "if number > INT_MAX as f64 {\n        (*item).valueint = INT_MAX;"),
 ("parse_number: <= INT_MIN -> <", "} else if number <= INT_MIN as f64 {\n        (*item).valueint = INT_MIN;", "} else if number < INT_MIN as f64 {\n        (*item).valueint = INT_MIN;"),
 ("parse_number: drop 'E' from charset", "| b'e' | b'E' => {", "| b'e' => {"),
 ("parse_number: drop '+' from charset", "| b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'", "| b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'-'"),
]
res=[]
for name, old, new in M:
    s=open(BAK).read()
    if old not in s:
        res.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    out=p.stdout+p.stderr
    if 'error[' in out or 'error:' in out and 'test failed' not in out:
        res.append((name,'BUILD-ERROR')); continue
    res.append((name,'detected' if 'FAILED' in out else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in res: print(f"{r:20s} {n}")
