import subprocess, shutil
SRC='src/cjson.rs'
BAK='$HARVEST_WORKDIR/work/cjson.rs.orig6'
shutil.copy(SRC,BAK)
M = [
 ("add_item_to_array: skip child->prev update", "            suffix_object((*child).prev, item);\n            (*(*array).child).prev = item;", "            suffix_object((*child).prev, item);"),
 ("add_item_to_array: empty list prev = null", "        (*array).child = item;\n        (*item).prev = item;", "        (*array).child = item;\n        (*item).prev = ptr::null_mut();"),
 ("add_item_to_array: allow array == item", "    if item.is_null() || array.is_null() || (array == item) {", "    if item.is_null() || array.is_null() {"),
 ("suffix_object: drop prev link", "unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {\n    (*prev).next = item;\n    (*item).prev = prev;", "unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {\n    (*prev).next = item;"),
 ("Compare string: strcmp -> strncmp 1", "            if strcmp((*a).valuestring, (*b).valuestring) == 0 {\n                return TRUE;", "            if strncmp((*a).valuestring, (*b).valuestring, 1) == 0 {\n                return TRUE;"),
 ("Compare array: ignore length mismatch", "            while !a_element.is_null() && !b_element.is_null() {\n                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {\n                    return FALSE;\n                }", "            while !a_element.is_null() && !b_element.is_null() {\n                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {\n                    return TRUE;\n                }"),
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
