#!/usr/bin/env python3
"""Mutation testing for the differential suite.

Injects a catalogue of realistic single-edit translation defects into
`src/cjson.rs` / `src/cshim.rs`, rebuilds and runs the whole differential suite,
and reports which mutants survive.  A surviving mutant is a blind spot in the
tests.  The pristine sources in `.pristine/` are always restored afterwards.

    python3 mutation_check.py [--only N] [--fast]
"""
import argparse
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PRISTINE = os.path.join(HERE, ".pristine")

# (name, file, old, new)
MUTANTS = [
    ("print_number: 17g -> 16g", "cjson.rs",
     r'length = sprintf(nb, cs!(b"%1.17g\0"), d);',
     r'length = sprintf(nb, cs!(b"%1.16g\0"), d);'),
    ("print_number: 15g -> 14g", "cjson.rs",
     r'length = sprintf(nb, cs!(b"%1.15g\0"), d);',
     r'length = sprintf(nb, cs!(b"%1.14g\0"), d);'),
    ("print_number: drop the round-trip check", "cjson.rs",
     'if sscanf(nb as *const c_char, cs!(b"%lg\\0"), &mut test as *mut f64) != 1\n            || compare_double(test, d) == 0\n        {',
     'if false {'),
    ("print_number: number_buffer length guard 25 -> 26", "cjson.rs",
     'if (length < 0) || (length > (core::mem::size_of::<[u8; 26]>() - 1) as c_int) {',
     'if (length < 0) || (length > (core::mem::size_of::<[u8; 26]>()) as c_int) {'),
    ("print_string_ptr: control-byte escape cost 5 -> 4", "cjson.rs",
     '                if other < 32 {\n                    /* UTF-16 escape sequence uXXXX */\n                    escape_characters += 5;',
     '                if other < 32 {\n                    /* UTF-16 escape sequence uXXXX */\n                    escape_characters += 4;'),
    ("print_string_ptr: u%04x -> u%04X", "cjson.rs",
     r'cs!(b"u%04x\0"),', r'cs!(b"u%04X\0"),'),
    ("print_string_ptr: > 31 -> >= 31", "cjson.rs",
     'if (*input_pointer > 31) && (*input_pointer != b\'\\"\') && (*input_pointer != b\'\\\\\') {',
     'if (*input_pointer >= 31) && (*input_pointer != b\'\\"\') && (*input_pointer != b\'\\\\\') {'),
    ("parse_number: >= INT_MAX -> > INT_MAX", "cjson.rs",
     '    if number >= INT_MAX as f64 {\n        (*item).valueint = INT_MAX;',
     '    if number > INT_MAX as f64 {\n        (*item).valueint = INT_MAX;'),
    ("cJSON_CreateNumber: <= INT_MIN -> < INT_MIN", "cjson.rs",
     '        } else if num <= INT_MIN as f64 {',
     '        } else if num < INT_MIN as f64 {'),
    ("double_to_int: saturating instead of INT_MIN", "cshim.rs",
     'if value.is_nan() || value >= 2147483648.0f64 || value < -2147483648.0f64 {\n        c_int::MIN',
     'if false {\n        c_int::MIN'),
    ("case_insensitive_strcmp: NULL == NULL is equal", "cjson.rs",
     '    if string1.is_null() || string2.is_null() {\n        return 1;\n    }',
     '    if string1.is_null() && string2.is_null() {\n        return 0;\n    }\n    if string1.is_null() || string2.is_null() {\n        return 1;\n    }'),
    ("ensure: > INT_MAX/2 -> >= INT_MAX/2", "cjson.rs",
     'if needed > (INT_MAX as usize / 2) {',
     'if needed >= (INT_MAX as usize / 2) {'),
    ("ensure: newsize needed*2 -> needed", "cjson.rs",
     '        newsize = needed * 2;', '        newsize = needed;'),
    ("ensure: drop the offset >= length guard", "cjson.rs",
     'if ((*p).length > 0) && ((*p).offset >= (*p).length) {\n        /* make sure that offset is valid */\n        return ptr::null_mut();\n    }',
     'if false {\n        return ptr::null_mut();\n    }'),
    ("buffer_skip_whitespace: <= 32 -> < 32", "cjson.rs",
     'while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer) <= 32) {',
     'while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer) < 32) {'),
    ("buffer_skip_whitespace: drop the offset==length fixup", "cjson.rs",
     '    if (*buffer).offset == (*buffer).length {\n        (*buffer).offset -= 1;\n    }',
     '    if false {\n        (*buffer).offset -= 1;\n    }'),
    ("skip_utf8_bom: can_access_at_index(4) -> (3)", "cjson.rs",
     'if can_access_at_index(buffer, 4)\n        && strncmp(',
     'if can_access_at_index(buffer, 3)\n        && strncmp('),
    ("utf16: input_end - first < 6 -> < 5", "cjson.rs",
     'if (input_end as isize - first_sequence as isize) < 6 {',
     'if (input_end as isize - first_sequence as isize) < 5 {'),
    ("utf16: low-surrogate range 0xDC00 -> 0xDC01", "cjson.rs",
     'if first_code >= 0xDC00 && first_code <= 0xDFFF {',
     'if first_code >= 0xDC01 && first_code <= 0xDFFF {'),
    ("utf16: codepoint < 0x800 -> <= 0x800", "cjson.rs",
     '    } else if codepoint < 0x800 {',
     '    } else if codepoint <= 0x800 {'),
    ("utf16: first_byte_mark 0xE0 -> 0xF0", "cjson.rs",
     '        utf8_length = 3;\n        first_byte_mark = 0xE0;',
     '        utf8_length = 3;\n        first_byte_mark = 0xF0;'),
    ("parse_hex4: 'A'-'F' offset 10 -> 9", "cjson.rs",
     "h = h.wrapping_add((10u32.wrapping_add(c as u32)).wrapping_sub(b'A' as u32));",
     "h = h.wrapping_add((9u32.wrapping_add(c as u32)).wrapping_sub(b'A' as u32));"),
    ("parse_string: allocation_length off by one", "cjson.rs",
     'output = (*input_buffer).hooks.alloc(allocation_length + 1) as *mut u8;',
     'output = (*input_buffer).hooks.alloc(allocation_length) as *mut u8;'),
    ("parse_string: \\b emits 'b' instead of 0x08", "cjson.rs",
     "                    b'b' => {\n                        *output_pointer = 8;",
     "                    b'b' => {\n                        *output_pointer = b'b';"),
    ("parse_string: fail path does not rewind offset", "cjson.rs",
     '    (*input_buffer).offset =\n        (input_pointer as usize).wrapping_sub((*input_buffer).content as usize);\n\n    FALSE',
     '    FALSE'),
    ("parse_value: can_read(4) -> can_read(3) for null", "cjson.rs",
     '    if can_read(input_buffer, 4)\n        && strncmp(\n            buffer_at_offset(input_buffer) as *const c_char,\n            cs!(b"null\\0"),\n            4,\n        ) == 0',
     '    if can_read(input_buffer, 3)\n        && strncmp(\n            buffer_at_offset(input_buffer) as *const c_char,\n            cs!(b"null\\0"),\n            4,\n        ) == 0'),
    ("parse_value: true does not set valueint", "cjson.rs",
     '        (*item).type_ = cJSON_True;\n        (*item).valueint = 1;',
     '        (*item).type_ = cJSON_True;'),
    ("parse_array: nesting limit 1000 -> 1001", "cjson.rs",
     'const CJSON_NESTING_LIMIT: usize = 1000;',
     'const CJSON_NESTING_LIMIT: usize = 1001;'),
    ("Duplicate: circular limit 10000 -> 20000", "cjson.rs",
     'const CJSON_CIRCULAR_LIMIT: usize = 10000;',
     'const CJSON_CIRCULAR_LIMIT: usize = 20000;'),
    ("Duplicate: keeps cJSON_IsReference", "cjson.rs",
     '(*newitem).type_ = (*item).type_ & !cJSON_IsReference;',
     '(*newitem).type_ = (*item).type_;'),
    ("Duplicate: strdup const keys instead of sharing", "cjson.rs",
     '            (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {\n                (*item).string\n            } else {',
     '            (*newitem).string = if false {\n                (*item).string\n            } else {'),
    ("add_item_to_array: drop the array == item check", "cjson.rs",
     'if item.is_null() || array.is_null() || (array == item) {\n        return FALSE;\n    }',
     'if item.is_null() || array.is_null() {\n        return FALSE;\n    }'),
    ("add_item_to_object: drop the StringIsConst free guard", "cjson.rs",
     'if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {\n        (*hooks).dealloc((*item).string as *mut c_void);\n    }',
     'if false {\n        (*hooks).dealloc((*item).string as *mut c_void);\n    }'),
    ("compare_double: <= -> <", "cjson.rs",
     'if (a - b).abs() <= max_val * f64::EPSILON {',
     'if (a - b).abs() < max_val * f64::EPSILON {'),
    ("cJSON_Compare: array length check inverted", "cjson.rs",
     '            if a_element != b_element {\n                return FALSE;\n            }',
     '            if false {\n                return FALSE;\n            }'),
    ("cJSON_Compare: skip the reverse object pass", "cjson.rs",
     '            b_element = if !b.is_null() {\n                (*b).child\n            } else {\n                ptr::null_mut()\n            };\n            while !b_element.is_null() {',
     '            b_element = ptr::null_mut();\n            while !b_element.is_null() {'),
    ("cJSON_IsTrue: mask 0xff -> 0x03", "cjson.rs",
     '    (((*item).type_ & 0xff) == cJSON_True) as cJSON_bool',
     '    (((*item).type_ & 0x03) == cJSON_True) as cJSON_bool'),
    ("cJSON_IsBool: != 0 -> == cJSON_True", "cjson.rs",
     '    (((*item).type_ & (cJSON_True | cJSON_False)) != 0) as cJSON_bool',
     '    (((*item).type_ & (cJSON_True | cJSON_False)) == cJSON_True) as cJSON_bool'),
    ("print_object: closing indent depth-1 -> depth", "cjson.rs",
     '        while i < ((*output_buffer).depth - 1) {',
     '        while i < (*output_buffer).depth {'),
    ("print_object: comma length arithmetic", "cjson.rs",
     '        length = (if (*output_buffer).format != 0 { 1usize } else { 0usize })\n            + (if !(*current_item).next.is_null() {\n                1usize\n            } else {\n                0usize\n            });',
     '        length = 1usize\n            + (if !(*current_item).next.is_null() {\n                1usize\n            } else {\n                0usize\n            });'),
    ("print_array: separator ', ' -> ','", "cjson.rs",
     '            length = if (*output_buffer).format != 0 { 2 } else { 1 };\n            output_pointer = ensure(output_buffer, length + 1);',
     '            length = 1;\n            output_pointer = ensure(output_buffer, length + 1);'),
    ("print(): drop the cjson_min clamp", "cjson.rs",
     'cjson_min((*buffer).length, (*buffer).offset + 1),',
     '(*buffer).offset + 1,'),
    ("cJSON_PrintBuffered: allow prebuffer < 0", "cjson.rs",
     '    if prebuffer < 0 {\n        return ptr::null_mut();\n    }\n\n    (*p).buffer = global_hooks().alloc(prebuffer as usize) as *mut u8;',
     '    if prebuffer < -1 {\n        return ptr::null_mut();\n    }\n\n    (*p).buffer = global_hooks().alloc(prebuffer.max(0) as usize) as *mut u8;'),
    ("cJSON_PrintPreallocated: noalloc = FALSE", "cjson.rs",
     '    (*p).noalloc = TRUE;\n    (*p).format = format;',
     '    (*p).noalloc = FALSE;\n    (*p).format = format;'),
    ("cJSON_SetValuestring: drop the overlap check", "cjson.rs",
     '        if !((valuestring as usize).wrapping_add(v1_len) < (*object).valuestring as usize\n            || ((*object).valuestring as usize).wrapping_add(v2_len) < valuestring as usize)\n        {\n            return ptr::null_mut();\n        }',
     '        if false {\n            return ptr::null_mut();\n        }'),
    ("cJSON_SetValuestring: v1 <= v2 -> v1 < v2", "cjson.rs",
     '    if v1_len <= v2_len {', '    if v1_len < v2_len {'),
    ("cJSON_GetErrorPtr: ignore position", "cjson.rs",
     '    e.json.wrapping_add(e.position) as *const c_char',
     '    e.json as *const c_char'),
    ("ParseWithLengthOpts: error position length-1 -> length", "cjson.rs",
     '        } else if (*buffer).length > 0 {\n            local_error.position = (*buffer).length - 1;',
     '        } else if (*buffer).length > 0 {\n            local_error.position = (*buffer).length;'),
    ("ParseWithOpts: buffer_length strlen+1 -> strlen", "cjson.rs",
     '    buffer_length = strlen(value) + 1;', '    buffer_length = strlen(value);'),
    ("cJSON_DetachItemViaPointer: drop the prev == NULL guard", "cjson.rs",
     'if parent.is_null()\n        || item.is_null()\n        || (item != (*parent).child && (*item).prev.is_null())\n    {',
     'if parent.is_null() || item.is_null() {'),
    ("cJSON_InsertItemInArray: drop the corrupted-item guard", "cjson.rs",
     'if after_inserted != (*array).child && (*after_inserted).prev.is_null() {\n        /* return false if after_inserted is a corrupted array item */\n        return FALSE;\n    }',
     'if false {\n        return FALSE;\n    }'),
    ("cJSON_ReplaceItemViaPointer: drop the self-replace shortcut", "cjson.rs",
     '    if replacement == item {\n        return TRUE;\n    }',
     '    if false {\n        return TRUE;\n    }'),
    ("replace_item_in_object: keep StringIsConst", "cjson.rs",
     '    (*replacement).type_ &= !cJSON_StringIsConst;',
     '    (*replacement).type_ |= 0;'),
    ("cJSON_Minify: oneline comment advance 2 -> 1", "cjson.rs",
     '    *input = (*input).wrapping_add(2); /* static_strlen("//") */',
     '    *input = (*input).wrapping_add(1); /* static_strlen("//") */'),
    ("cJSON_Minify: multiline comment closer advance", "cjson.rs",
     '            *input = (*input).wrapping_add(2); // static_strlen of the comment closer',
     '            *input = (*input).wrapping_add(1); // static_strlen of the comment closer'),
    ("minify_string: drop the escaped-quote handling", "cjson.rs",
     "        } else if (**input == b'\\\\' as c_char) && (*(*input).wrapping_add(1) == b'\\\"' as c_char) {",
     "        } else if false {"),
    ("cJSON_GetNumberValue: positive NaN", "cjson.rs",
     '        return C_NAN;', '        return f64::NAN;'),
    ("cJSON_CreateBool: invert", "cjson.rs",
     '(*item).type_ = if boolean != 0 { cJSON_True } else { cJSON_False };',
     '(*item).type_ = if boolean == 1 { cJSON_True } else { cJSON_False };'),
    ("cJSON_Version: %i.%i.%i -> %d.%d.%d with wrong patch", "cjson.rs",
     'CJSON_VERSION_PATCH,\n    );', 'CJSON_VERSION_PATCH + 1,\n    );'),
    ("driver: PrintPreallocated len instead of len_fail", "driver.rs",
     'if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {',
     'if cJSON_PrintPreallocated(root, buf_fail, len as c_int, 1) != 0 {'),
    ("cJSON_InitHooks: always provide realloc", "cjson.rs",
     '    if allocate_is_malloc && deallocate_is_free {',
     '    if true || (allocate_is_malloc && deallocate_is_free) {'),
    ("get_object_item: case-sensitive loop ignores NULL keys", "cjson.rs",
     '        while !current_element.is_null()\n            && !(*current_element).string.is_null()\n            && (strcmp(name, (*current_element).string) != 0)',
     '        while !current_element.is_null()\n            && ((*current_element).string.is_null()\n                || strcmp(name, (*current_element).string) != 0)'),
]


def restore():
    for f in ("cjson.rs", "cshim.rs", "driver.rs"):
        shutil.copyfile(os.path.join(PRISTINE, f), os.path.join(HERE, "src", f))


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=HERE, capture_output=True, text=True, **kw)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", type=int, default=None)
    ap.add_argument("--fast", action="store_true",
                    help="skip the slow bigalloc test binary")
    args = ap.parse_args()

    assert os.path.isdir(PRISTINE), "missing .pristine/ snapshot"
    restore()

    test_cmd = ["cargo", "test", "--release"]
    if args.fast:
        test_cmd = ["cargo", "test", "--release",
                    "--test", "create", "--test", "print", "--test", "parse",
                    "--test", "mutate", "--test", "errors", "--test", "hooks",
                    "--test", "driver", "--test", "smoke", "--test", "guarded"]

    survivors, killed, broken = [], [], []
    for i, (name, fname, old, new) in enumerate(MUTANTS):
        if args.only is not None and args.only != i:
            continue
        path = os.path.join(HERE, "src", fname)
        src = open(path).read()
        if old not in src:
            broken.append((i, name, "pattern not found"))
            print(f"[{i:2}] SKIP  {name}  (pattern not found in {fname})")
            continue
        open(path, "w").write(src.replace(old, new, 1))

        b = run(["cargo", "build", "--release"])
        if b.returncode != 0:
            broken.append((i, name, "build failed"))
            print(f"[{i:2}] SKIP  {name}  (does not compile)")
            restore()
            continue

        t = run(test_cmd, timeout=900)
        if t.returncode == 0:
            survivors.append((i, name))
            print(f"[{i:2}] SURVIVED  {name}")
        else:
            first = ""
            for line in (t.stdout + t.stderr).splitlines():
                if "FAILED" in line and line.startswith("test "):
                    first = line.strip()
                    break
            killed.append((i, name, first))
            print(f"[{i:2}] killed    {name}   <- {first}")
        restore()

    run(["cargo", "build", "--release"])
    print("\n==== mutation summary ====")
    print(f"killed:    {len(killed)}")
    print(f"survived:  {len(survivors)}")
    print(f"skipped:   {len(broken)}")
    for i, n in survivors:
        print(f"  SURVIVOR [{i}] {n}")
    for i, n, why in broken:
        print(f"  skipped  [{i}] {n}: {why}")
    return 1 if survivors else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        restore()
