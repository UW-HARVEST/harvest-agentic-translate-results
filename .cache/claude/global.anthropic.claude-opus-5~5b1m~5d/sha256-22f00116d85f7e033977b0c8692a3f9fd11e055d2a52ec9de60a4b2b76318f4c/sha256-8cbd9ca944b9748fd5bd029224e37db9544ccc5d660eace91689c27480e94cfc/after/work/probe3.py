import subprocess, shutil
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig3'
shutil.copy(SRC,BAK)
M = [
 ("skip_whitespace: <= 32 -> < 32", "while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer) <= 32) {", "while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer) < 32) {"),
 ("skip_whitespace: drop offset-- fixup", "    if (*buffer).offset == (*buffer).length {\n        (*buffer).offset -= 1;\n    }", "    if false {\n        (*buffer).offset -= 1;\n    }"),
 ("skip_utf8_bom: can_access 4 -> 3", "if can_access_at_index(buffer, 4)", "if can_access_at_index(buffer, 3)"),
 ("skip_utf8_bom: strncmp 3 -> 2", "            cs!(b\"\\xEF\\xBB\\xBF\\0\"),\n            3,", "            cs!(b\"\\xEF\\xBB\\xBF\\0\"),\n            2,"),
 ("print_array: closing ensure 2 -> 1", "    output_pointer = ensure(output_buffer, 2);\n    if output_pointer.is_null() {\n        return FALSE;\n    }\n    *output_pointer = b']';", "    output_pointer = ensure(output_buffer, 1);\n    if output_pointer.is_null() {\n        return FALSE;\n    }\n    *output_pointer = b']';"),
 ("print_array: comma length swap", "            length = if (*output_buffer).format != 0 { 2 } else { 1 };", "            length = if (*output_buffer).format != 0 { 1 } else { 2 };"),
 ("print_object: open length swap", "    length = if (*output_buffer).format != 0 { 2 } else { 1 }; /* fmt: {\\n */", "    length = if (*output_buffer).format != 0 { 1 } else { 2 }; /* fmt: {\\n */"),
 ("get_array_item: index > 0 -> >= 0", "    while !current_child.is_null() && (index > 0) {", "    while !current_child.is_null() && (index >= 0) {"),
 ("GetArrayItem: index < 0 -> <= 0", "    if index < 0 {\n        return ptr::null_mut();", "    if index <= 0 {\n        return ptr::null_mut();"),
 ("print_string_ptr: escape <32 -> <31", "                if other < 32 {", "                if other < 31 {"),
 ("print_string_ptr: escape count 5 -> 4", "                    escape_characters += 5;", "                    escape_characters += 4;"),
 ("print_string_ptr: empty ensure 3 -> 2", "let out = ensure(output_buffer, 3 /* sizeof(\"\\\"\\\"\") */);", "let out = ensure(output_buffer, 2 /* sizeof(\"\\\"\\\"\") */);"),
 ("parse_string: end check >= -> >", "            if (((input_end as usize).wrapping_sub((*input_buffer).content as usize))\n                >= (*input_buffer).length)", "            if (((input_end as usize).wrapping_sub((*input_buffer).content as usize))\n                > (*input_buffer).length)"),
 ("parse_string: \\b 8 -> 7", "                    b'b' => {\n                        *output_pointer = 8;", "                    b'b' => {\n                        *output_pointer = 7;"),
 ("print_string_ptr: output_length +3 -> +2", "output = ensure(output_buffer, output_length + 3 /* sizeof(\"\\\"\\\"\") */);", "output = ensure(output_buffer, output_length + 2 /* sizeof(\"\\\"\\\"\") */);"),
]
res=[]
for name, old, new in M:
    s=open(BAK).read()
    if old not in s:
        res.append((name,'PATTERN-NOT-FOUND')); continue
    open(SRC,'w').write(s.replace(old,new,1))
    p=subprocess.run(['cargo','test','--offline','-q'],capture_output=True,text=True)
    out=p.stdout+p.stderr
    if 'error[' in out:
        res.append((name,'BUILD-ERROR')); continue
    res.append((name,'detected' if 'FAILED' in out else 'NOT DETECTED'))
shutil.copy(BAK,SRC)
for n,r in res: print(f"{r:20s} {n}")
