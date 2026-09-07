import subprocess, shutil
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig5'
shutil.copy(SRC,BAK)
M = [
 ("print_object: indent tab -> space", "            while i < (*output_buffer).depth {\n                *output_pointer = b'\\t';", "            while i < (*output_buffer).depth {\n                *output_pointer = b' ';"),
 ("print_object: indent depth-1", "            output_pointer = ensure(output_buffer, (*output_buffer).depth);", "            output_pointer = ensure(output_buffer, (*output_buffer).depth - 1);"),
 ("print_object: colon ensure length -> length+1", "        *output_pointer = b':';\n        output_pointer = output_pointer.wrapping_add(1);\n        if (*output_buffer).format != 0 {\n            *output_pointer = b'\\t';", "        *output_pointer = b':';\n        output_pointer = output_pointer.wrapping_add(1);\n        if (*output_buffer).format != 0 {\n            *output_pointer = b' ';"),
 ("print_object: trailing comma length off by one", "        length = (if (*output_buffer).format != 0 { 1usize } else { 0usize })\n            + (if !(*current_item).next.is_null() {\n                1usize\n            } else {\n                0usize\n            });", "        length = (if (*output_buffer).format != 0 { 1usize } else { 0usize })\n            + (if !(*current_item).next.is_null() {\n                0usize\n            } else {\n                1usize\n            });"),
 ("minify_string: drop escaped-quote handling", "} else if (**input == b'\\\\' as c_char) && (*(*input).wrapping_add(1) == b'\\\"' as c_char) {", "} else if false {"),
 ("minify: '/' fallthrough consumes 2", "                } else {\n                    json = json.wrapping_add(1);\n                }\n            }\n\n            b'\\\"' => {", "                } else {\n                    json = json.wrapping_add(2);\n                }\n            }\n\n            b'\\\"' => {"),
 ("minify: treat \\r as content", "            b' ' | b'\\t' | b'\\r' | b'\\n' => {", "            b' ' | b'\\t' | b'\\n' => {"),
 ("IsInvalid: 0xFF mask -> 0x7F", "    (((*item).type_ & 0xFF) == cJSON_Invalid) as cJSON_bool", "    (((*item).type_ & 0x7F) == cJSON_Invalid) as cJSON_bool"),
 ("IsFalse: 0xFF mask -> 0xFE", "    (((*item).type_ & 0xFF) == cJSON_False) as cJSON_bool", "    (((*item).type_ & 0xFE) == cJSON_False) as cJSON_bool"),
]
res=[]
for name, old, new in M:
    s=open(BAK).read()
    if old not in s:
        res.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    if 'error[' in p.stdout+p.stderr:
        res.append((name,'BUILD-ERROR')); continue
    res.append((name,'detected' if p.returncode!=0 else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in res: print(f"{r:20s} {n}")
