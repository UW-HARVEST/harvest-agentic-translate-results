// Translation of c_src/cJSON.c + c_src/test.c to Rust.
//
// The original C code is a JSON library plus a "driver" function that exercises
// a few of its features by printing the resulting JSON to stdout. There is no
// `main` in the C sources — the driver is invoked from a host that supplies
// canonical sample data. We replicate that here so this program is a runnable
// executable that produces byte-identical output to the C version.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::ptr;
use std::rc::Rc;

// cJSON type bit flags (matching c_src/cJSON.h)
const CJSON_INVALID: i32 = 0;
const CJSON_FALSE: i32 = 1 << 0;
const CJSON_TRUE: i32 = 1 << 1;
const CJSON_NULL: i32 = 1 << 2;
const CJSON_NUMBER: i32 = 1 << 3;
const CJSON_STRING: i32 = 1 << 4;
const CJSON_ARRAY: i32 = 1 << 5;
const CJSON_OBJECT: i32 = 1 << 6;
const CJSON_RAW: i32 = 1 << 7;
const CJSON_IS_REFERENCE: i32 = 256;
const CJSON_STRING_IS_CONST: i32 = 512;

const CJSON_VERSION_MAJOR: i32 = 1;
const CJSON_VERSION_MINOR: i32 = 7;
const CJSON_VERSION_PATCH: i32 = 19;

// ---------------------------------------------------------------------------
// cJSON node type (translated from struct cJSON)
//
// We use Rc<RefCell<...>> + raw next/prev pointers to faithfully replicate
// cJSON's intrusive doubly-linked-list semantics (the original uses a circular
// `prev` link in arrays, which we preserve so node ordering matches the C
// implementation byte-for-byte).
// ---------------------------------------------------------------------------

type NodePtr = *mut Node;

struct Node {
    next: NodePtr,
    prev: NodePtr,
    child: NodePtr,
    typ: i32,
    valuestring: Option<Vec<u8>>, // null-terminated bytes (without trailing NUL kept implicit)
    valueint: i32,
    valuedouble: f64,
    string: Option<Vec<u8>>, // key for object members
}

impl Node {
    fn new() -> *mut Node {
        Box::into_raw(Box::new(Node {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            child: ptr::null_mut(),
            typ: 0,
            valuestring: None,
            valueint: 0,
            valuedouble: 0.0,
            string: None,
        }))
    }
}

// Recursively delete a cJSON tree (mirrors cJSON_Delete).
unsafe fn cjson_delete(mut item: *mut Node) {
    while !item.is_null() {
        let next = (*item).next;
        if ((*item).typ & CJSON_IS_REFERENCE) == 0 && !(*item).child.is_null() {
            cjson_delete((*item).child);
        }
        // valuestring and string are owned Vecs — Box drop handles them.
        let _ = Box::from_raw(item);
        item = next;
    }
}

// ---------------------------------------------------------------------------
// printbuffer (mirrors the C printbuffer struct)
// ---------------------------------------------------------------------------

struct PrintBuffer {
    buffer: Vec<u8>, // logical buffer; len == "length"
    offset: usize,
    depth: usize,
    noalloc: bool,
    format: bool,
    // For preallocated mode we operate on an externally-supplied buffer.
    // We model that by holding the data in `buffer` and copying back at the
    // end if requested by the caller.
}

impl PrintBuffer {
    fn new(initial_size: usize, format: bool) -> Self {
        PrintBuffer {
            buffer: vec![0u8; initial_size],
            offset: 0,
            depth: 0,
            noalloc: false,
            format,
        }
    }

    fn new_preallocated(buf_len: usize, format: bool) -> Self {
        PrintBuffer {
            buffer: vec![0u8; buf_len],
            offset: 0,
            depth: 0,
            noalloc: true,
            format,
        }
    }
}

// Equivalent of C `ensure`. Returns Some(start_index) on success — caller
// writes into self.buffer[start_index..] via write_at helpers.
fn ensure(p: &mut PrintBuffer, mut needed: usize) -> Option<usize> {
    if p.buffer.is_empty() {
        return None;
    }
    if p.buffer.len() > 0 && p.offset >= p.buffer.len() {
        return None;
    }
    if needed > i32::MAX as usize {
        return None;
    }
    needed = needed
        .checked_add(p.offset)
        .and_then(|n| n.checked_add(1))
        .unwrap_or(usize::MAX);
    if needed <= p.buffer.len() {
        return Some(p.offset);
    }
    if p.noalloc {
        return None;
    }
    let newsize = if needed > (i32::MAX as usize / 2) {
        if needed <= i32::MAX as usize {
            i32::MAX as usize
        } else {
            return None;
        }
    } else {
        needed * 2
    };
    p.buffer.resize(newsize, 0);
    Some(p.offset)
}

// strlen-equivalent on the print buffer starting from offset.
fn buffer_strlen(p: &PrintBuffer) -> usize {
    let start = p.offset;
    let mut i = start;
    while i < p.buffer.len() && p.buffer[i] != 0 {
        i += 1;
    }
    i - start
}

fn update_offset(p: &mut PrintBuffer) {
    let n = buffer_strlen(p);
    p.offset += n;
}

// ---------------------------------------------------------------------------
// Number printing — uses libc::sprintf to guarantee byte-identical formatting
// of doubles compared to the original C code.
// ---------------------------------------------------------------------------

fn compare_double(a: f64, b: f64) -> bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    (a - b).abs() <= max_val * f64::EPSILON
}

fn print_number(item: &Node, output_buffer: &mut PrintBuffer) -> bool {
    let d = item.valuedouble;
    let mut number_buffer: [u8; 26] = [0; 26];
    let length: i32;

    unsafe {
        if d.is_nan() || d.is_infinite() {
            length = libc::sprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                b"null\0".as_ptr() as *const c_char,
            );
        } else if d == item.valueint as f64 {
            length = libc::sprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                b"%d\0".as_ptr() as *const c_char,
                item.valueint as c_int,
            );
        } else {
            length = libc::sprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                b"%1.15g\0".as_ptr() as *const c_char,
                d,
            );
            // Try to round-trip the value
            let mut test: c_double = 0.0;
            let scanned = libc::sscanf(
                number_buffer.as_ptr() as *const c_char,
                b"%lg\0".as_ptr() as *const c_char,
                &mut test as *mut c_double,
            );
            if scanned != 1 || !compare_double(test, d) {
                let length2 = libc::sprintf(
                    number_buffer.as_mut_ptr() as *mut c_char,
                    b"%1.17g\0".as_ptr() as *const c_char,
                    d,
                );
                if length2 < 0 || length2 > (number_buffer.len() - 1) as i32 {
                    return false;
                }
                let length = length2;
                let start = match ensure(output_buffer, length as usize + 1) {
                    Some(s) => s,
                    None => return false,
                };
                for i in 0..length as usize {
                    output_buffer.buffer[start + i] = number_buffer[i];
                }
                output_buffer.buffer[start + length as usize] = 0;
                output_buffer.offset += length as usize;
                return true;
            }
        }
    }

    if length < 0 || length > (number_buffer.len() - 1) as i32 {
        return false;
    }
    let start = match ensure(output_buffer, length as usize + 1) {
        Some(s) => s,
        None => return false,
    };
    for i in 0..length as usize {
        output_buffer.buffer[start + i] = number_buffer[i];
    }
    output_buffer.buffer[start + length as usize] = 0;
    output_buffer.offset += length as usize;
    true
}

// ---------------------------------------------------------------------------
// String printing
// ---------------------------------------------------------------------------

// Returns the length of the input C-string (NUL-terminated bytes).
fn cstr_len(input: &[u8]) -> usize {
    let mut i = 0;
    while i < input.len() && input[i] != 0 {
        i += 1;
    }
    i
}

fn print_string_ptr(input: Option<&[u8]>, output_buffer: &mut PrintBuffer) -> bool {
    // "empty" pointer case — original prints "\"\""
    let bytes = match input {
        Some(b) => b,
        None => {
            let start = match ensure(output_buffer, 3) {
                Some(s) => s,
                None => return false,
            };
            output_buffer.buffer[start] = b'"';
            output_buffer.buffer[start + 1] = b'"';
            output_buffer.buffer[start + 2] = 0;
            return true;
        }
    };

    let n = cstr_len(bytes);
    let mut escape_characters: usize = 0;
    for &c in &bytes[..n] {
        match c {
            b'"' | b'\\' | 0x08 | 0x0c | b'\n' | b'\r' | b'\t' => escape_characters += 1,
            _ => {
                if c < 32 {
                    escape_characters += 5;
                }
            }
        }
    }
    let output_length = n + escape_characters;
    let start = match ensure(output_buffer, output_length + 3) {
        Some(s) => s,
        None => return false,
    };

    if escape_characters == 0 {
        output_buffer.buffer[start] = b'"';
        for i in 0..output_length {
            output_buffer.buffer[start + 1 + i] = bytes[i];
        }
        output_buffer.buffer[start + output_length + 1] = b'"';
        output_buffer.buffer[start + output_length + 2] = 0;
        return true;
    }

    output_buffer.buffer[start] = b'"';
    let mut wp = start + 1;
    for &c in &bytes[..n] {
        if c > 31 && c != b'"' && c != b'\\' {
            output_buffer.buffer[wp] = c;
            wp += 1;
        } else {
            output_buffer.buffer[wp] = b'\\';
            wp += 1;
            match c {
                b'\\' => {
                    output_buffer.buffer[wp] = b'\\';
                    wp += 1;
                }
                b'"' => {
                    output_buffer.buffer[wp] = b'"';
                    wp += 1;
                }
                0x08 => {
                    output_buffer.buffer[wp] = b'b';
                    wp += 1;
                }
                0x0c => {
                    output_buffer.buffer[wp] = b'f';
                    wp += 1;
                }
                b'\n' => {
                    output_buffer.buffer[wp] = b'n';
                    wp += 1;
                }
                b'\r' => {
                    output_buffer.buffer[wp] = b'r';
                    wp += 1;
                }
                b'\t' => {
                    output_buffer.buffer[wp] = b't';
                    wp += 1;
                }
                _ => {
                    // Use libc sprintf to format "u%04x" — exactly mirrors C
                    let mut tmp = [0u8; 8];
                    unsafe {
                        libc::sprintf(
                            tmp.as_mut_ptr() as *mut c_char,
                            b"u%04x\0".as_ptr() as *const c_char,
                            c as c_int,
                        );
                    }
                    for i in 0..5 {
                        output_buffer.buffer[wp + i] = tmp[i];
                    }
                    wp += 5;
                }
            }
        }
    }
    output_buffer.buffer[start + output_length + 1] = b'"';
    output_buffer.buffer[start + output_length + 2] = 0;
    true
}

fn print_string(item: &Node, p: &mut PrintBuffer) -> bool {
    print_string_ptr(item.valuestring.as_deref(), p)
}

// ---------------------------------------------------------------------------
// Value/array/object printing
// ---------------------------------------------------------------------------

unsafe fn print_value(item: *const Node, output_buffer: &mut PrintBuffer) -> bool {
    if item.is_null() {
        return false;
    }
    let typ = (*item).typ & 0xFF;
    match typ {
        x if x == CJSON_NULL => {
            let start = match ensure(output_buffer, 5) {
                Some(s) => s,
                None => return false,
            };
            let s = b"null";
            for i in 0..s.len() {
                output_buffer.buffer[start + i] = s[i];
            }
            output_buffer.buffer[start + 4] = 0;
            true
        }
        x if x == CJSON_FALSE => {
            let start = match ensure(output_buffer, 6) {
                Some(s) => s,
                None => return false,
            };
            let s = b"false";
            for i in 0..s.len() {
                output_buffer.buffer[start + i] = s[i];
            }
            output_buffer.buffer[start + 5] = 0;
            true
        }
        x if x == CJSON_TRUE => {
            let start = match ensure(output_buffer, 5) {
                Some(s) => s,
                None => return false,
            };
            let s = b"true";
            for i in 0..s.len() {
                output_buffer.buffer[start + i] = s[i];
            }
            output_buffer.buffer[start + 4] = 0;
            true
        }
        x if x == CJSON_NUMBER => print_number(&*item, output_buffer),
        x if x == CJSON_RAW => {
            let bytes = match &(*item).valuestring {
                Some(b) => b.clone(),
                None => return false,
            };
            let raw_length = cstr_len(&bytes) + 1;
            let start = match ensure(output_buffer, raw_length) {
                Some(s) => s,
                None => return false,
            };
            for i in 0..raw_length - 1 {
                output_buffer.buffer[start + i] = bytes[i];
            }
            output_buffer.buffer[start + raw_length - 1] = 0;
            true
        }
        x if x == CJSON_STRING => print_string(&*item, output_buffer),
        x if x == CJSON_ARRAY => print_array(item, output_buffer),
        x if x == CJSON_OBJECT => print_object(item, output_buffer),
        _ => false,
    }
}

unsafe fn print_array(item: *const Node, output_buffer: &mut PrintBuffer) -> bool {
    let mut current = (*item).child;
    let start = match ensure(output_buffer, 1) {
        Some(s) => s,
        None => return false,
    };
    output_buffer.buffer[start] = b'[';
    output_buffer.offset += 1;
    output_buffer.depth += 1;

    while !current.is_null() {
        if !print_value(current, output_buffer) {
            return false;
        }
        update_offset(output_buffer);
        if !(*current).next.is_null() {
            let length = if output_buffer.format { 2 } else { 1 };
            let start = match ensure(output_buffer, length + 1) {
                Some(s) => s,
                None => return false,
            };
            output_buffer.buffer[start] = b',';
            if output_buffer.format {
                output_buffer.buffer[start + 1] = b' ';
            }
            output_buffer.buffer[start + length] = 0;
            output_buffer.offset += length;
        }
        current = (*current).next;
    }
    let start = match ensure(output_buffer, 2) {
        Some(s) => s,
        None => return false,
    };
    output_buffer.buffer[start] = b']';
    output_buffer.buffer[start + 1] = 0;
    output_buffer.depth -= 1;
    true
}

unsafe fn print_object(item: *const Node, output_buffer: &mut PrintBuffer) -> bool {
    let mut current = (*item).child;
    let length = if output_buffer.format { 2 } else { 1 };
    let start = match ensure(output_buffer, length + 1) {
        Some(s) => s,
        None => return false,
    };
    output_buffer.buffer[start] = b'{';
    output_buffer.depth += 1;
    if output_buffer.format {
        output_buffer.buffer[start + 1] = b'\n';
    }
    output_buffer.offset += length;

    while !current.is_null() {
        if output_buffer.format {
            let depth = output_buffer.depth;
            let s = match ensure(output_buffer, depth) {
                Some(s) => s,
                None => return false,
            };
            for i in 0..depth {
                output_buffer.buffer[s + i] = b'\t';
            }
            output_buffer.offset += depth;
        }

        // print key
        if !print_string_ptr((*current).string.as_deref(), output_buffer) {
            return false;
        }
        update_offset(output_buffer);

        let length = if output_buffer.format { 2 } else { 1 };
        let s = match ensure(output_buffer, length) {
            Some(s) => s,
            None => return false,
        };
        output_buffer.buffer[s] = b':';
        if output_buffer.format {
            output_buffer.buffer[s + 1] = b'\t';
        }
        output_buffer.offset += length;

        // print value
        if !print_value(current, output_buffer) {
            return false;
        }
        update_offset(output_buffer);

        let mut length = if output_buffer.format { 1 } else { 0 };
        if !(*current).next.is_null() {
            length += 1;
        }
        let s = match ensure(output_buffer, length + 1) {
            Some(s) => s,
            None => return false,
        };
        let mut wp = s;
        if !(*current).next.is_null() {
            output_buffer.buffer[wp] = b',';
            wp += 1;
        }
        if output_buffer.format {
            output_buffer.buffer[wp] = b'\n';
        }
        output_buffer.buffer[s + length] = 0;
        output_buffer.offset += length;

        current = (*current).next;
    }

    let depth = output_buffer.depth;
    let needed = if output_buffer.format { depth + 1 } else { 2 };
    let s = match ensure(output_buffer, needed) {
        Some(s) => s,
        None => return false,
    };
    let mut wp = s;
    if output_buffer.format {
        for _ in 0..depth - 1 {
            output_buffer.buffer[wp] = b'\t';
            wp += 1;
        }
    }
    output_buffer.buffer[wp] = b'}';
    wp += 1;
    output_buffer.buffer[wp] = 0;
    output_buffer.depth -= 1;
    true
}

// Top-level print — returns NUL-terminated bytes of the formatted JSON.
unsafe fn cjson_print(item: *const Node, format: bool) -> Option<Vec<u8>> {
    let default_buffer_size = 256usize;
    let mut buffer = PrintBuffer::new(default_buffer_size, format);
    if !print_value(item, &mut buffer) {
        return None;
    }
    update_offset(&mut buffer);
    let mut result = buffer.buffer;
    result.truncate(buffer.offset + 1);
    if let Some(last) = result.last_mut() {
        *last = 0;
    } else {
        result.push(0);
    }
    Some(result)
}

unsafe fn cjson_print_preallocated(
    item: *const Node,
    buffer: &mut [u8],
    length: i32,
    format: bool,
) -> bool {
    if length < 0 || buffer.is_empty() {
        return false;
    }
    let mut p = PrintBuffer::new_preallocated(length as usize, format);
    let result = print_value(item, &mut p);
    if result {
        // Copy back
        let n = p.buffer.len().min(buffer.len());
        buffer[..n].copy_from_slice(&p.buffer[..n]);
    }
    result
}

// ---------------------------------------------------------------------------
// Construction helpers — translated from cJSON.c
// ---------------------------------------------------------------------------

unsafe fn cjson_strdup(s: &[u8]) -> Vec<u8> {
    // includes terminating NUL
    let mut v = Vec::with_capacity(s.len() + 1);
    v.extend_from_slice(s);
    v.push(0);
    v
}

unsafe fn cjson_create_object() -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_OBJECT;
    n
}

unsafe fn cjson_create_array() -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_ARRAY;
    n
}

unsafe fn cjson_create_string(s: &str) -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_STRING;
    (*n).valuestring = Some(cjson_strdup(s.as_bytes()));
    n
}

unsafe fn cjson_create_string_c(s: *const c_char) -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_STRING;
    if s.is_null() {
        cjson_delete(n);
        return ptr::null_mut();
    }
    let len = libc::strlen(s);
    let mut v = Vec::with_capacity(len + 1);
    for i in 0..len {
        v.push(*s.add(i) as u8);
    }
    v.push(0);
    (*n).valuestring = Some(v);
    n
}

unsafe fn cjson_create_number(num: f64) -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_NUMBER;
    (*n).valuedouble = num;
    if num >= i32::MAX as f64 {
        (*n).valueint = i32::MAX;
    } else if num <= i32::MIN as f64 {
        (*n).valueint = i32::MIN;
    } else {
        (*n).valueint = num as i32;
    }
    n
}

unsafe fn cjson_create_false() -> *mut Node {
    let n = Node::new();
    (*n).typ = CJSON_FALSE;
    n
}

unsafe fn cjson_create_int_array(numbers: &[i32]) -> *mut Node {
    let a = cjson_create_array();
    let mut p: *mut Node = ptr::null_mut();
    for (i, &num) in numbers.iter().enumerate() {
        let n = cjson_create_number(num as f64);
        if n.is_null() {
            cjson_delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            // suffix_object
            (*p).next = n;
            (*n).prev = p;
        }
        p = n;
    }
    if !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

unsafe fn cjson_create_string_array_c(strings: &[*const c_char]) -> *mut Node {
    let a = cjson_create_array();
    let mut p: *mut Node = ptr::null_mut();
    for (i, &s) in strings.iter().enumerate() {
        let n = cjson_create_string_c(s);
        if n.is_null() {
            cjson_delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            (*p).next = n;
            (*n).prev = p;
        }
        p = n;
    }
    if !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

unsafe fn add_item_to_array(array: *mut Node, item: *mut Node) -> bool {
    if item.is_null() || array.is_null() || array == item {
        return false;
    }
    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else if !(*child).prev.is_null() {
        let last = (*child).prev;
        (*last).next = item;
        (*item).prev = last;
        (*(*array).child).prev = item;
    }
    true
}

unsafe fn add_item_to_object(
    object: *mut Node,
    key: &str,
    item: *mut Node,
    constant_key: bool,
) -> bool {
    if object.is_null() || item.is_null() || object == item {
        return false;
    }
    let new_type;
    let new_key: Vec<u8>;
    if constant_key {
        new_key = cjson_strdup(key.as_bytes());
        new_type = (*item).typ | CJSON_STRING_IS_CONST;
    } else {
        new_key = cjson_strdup(key.as_bytes());
        new_type = (*item).typ & !CJSON_STRING_IS_CONST;
    }
    (*item).string = Some(new_key);
    (*item).typ = new_type;
    add_item_to_array(object, item)
}

unsafe fn cjson_add_item_to_object(object: *mut Node, key: &str, item: *mut Node) -> bool {
    add_item_to_object(object, key, item, false)
}

unsafe fn cjson_add_string_to_object(object: *mut Node, name: &str, string: &str) -> *mut Node {
    let s = cjson_create_string(string);
    if add_item_to_object(object, name, s, false) {
        s
    } else {
        cjson_delete(s);
        ptr::null_mut()
    }
}

unsafe fn cjson_add_number_to_object(object: *mut Node, name: &str, number: f64) -> *mut Node {
    let n = cjson_create_number(number);
    if add_item_to_object(object, name, n, false) {
        n
    } else {
        cjson_delete(n);
        ptr::null_mut()
    }
}

unsafe fn cjson_add_false_to_object(object: *mut Node, name: &str) -> *mut Node {
    let f = cjson_create_false();
    if add_item_to_object(object, name, f, false) {
        f
    } else {
        cjson_delete(f);
        ptr::null_mut()
    }
}

// ---------------------------------------------------------------------------
// Version (mirrors C: snprintf into static buffer, returns it).
// ---------------------------------------------------------------------------

thread_local! {
    static VERSION_BUFFER: RefCell<[u8; 15]> = RefCell::new([0; 15]);
}

fn cjson_version() -> String {
    format!(
        "{}.{}.{}",
        CJSON_VERSION_MAJOR, CJSON_VERSION_MINOR, CJSON_VERSION_PATCH
    )
}

// ---------------------------------------------------------------------------
// Helpers used by test.c (driver function and create_objects)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Record {
    precision: &'static str,
    lat: f64,
    lon: f64,
    address: &'static str,
    city: &'static str,
    state: &'static str,
    zip: &'static str,
    country: &'static str,
}

fn print_str_with_newline(s: &[u8]) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Drop trailing NUL if present
    let n = cstr_len(s);
    out.write_all(&s[..n]).unwrap();
    out.write_all(b"\n").unwrap();
}

unsafe fn print_preallocated(root: *mut Node) -> i32 {
    // formatted print
    let out = match cjson_print(root, true) {
        Some(v) => v,
        None => return -1,
    };
    let out_len = cstr_len(&out);

    let len = out_len + 5;
    let mut buf: Vec<u8> = vec![0u8; len];

    let len_fail = out_len;
    let mut buf_fail: Vec<u8> = vec![0u8; len_fail];

    // Print to buffer
    if !cjson_print_preallocated(root, &mut buf, len as i32, true) {
        println!("cJSON_PrintPreallocated failed!");
        // Compare out vs buf (as C strings)
        let buf_n = cstr_len(&buf);
        if out[..out_len] != buf[..buf_n] {
            println!("cJSON_PrintPreallocated not the same as cJSON_Print!");
            print!("cJSON_Print result:\n");
            std::io::Write::write_all(&mut std::io::stdout().lock(), &out[..out_len]).unwrap();
            println!();
            print!("cJSON_PrintPreallocated result:\n");
            std::io::Write::write_all(&mut std::io::stdout().lock(), &buf[..buf_n]).unwrap();
            println!();
        }
        return -1;
    }

    // success — print %s\n
    print_str_with_newline(&buf);

    // Force failure with smaller buffer
    if !buf_fail.is_empty()
        && cjson_print_preallocated(root, &mut buf_fail, len_fail as i32, true)
    {
        println!("cJSON_PrintPreallocated failed to show error with insufficient memory!");
        print!("cJSON_Print result:\n");
        std::io::Write::write_all(&mut std::io::stdout().lock(), &out[..out_len]).unwrap();
        println!();
        let bf_n = cstr_len(&buf_fail);
        print!("cJSON_PrintPreallocated result:\n");
        std::io::Write::write_all(&mut std::io::stdout().lock(), &buf_fail[..bf_n]).unwrap();
        println!();
        return -1;
    }

    0
}

unsafe fn create_objects(strings: &[&str; 7], numbers: &[[i32; 3]; 3], ids: &[i32; 4], fields: &[Record; 2]) {
    // volatile double zero = 0.0
    let zero: f64 = 0.0;

    // Video object
    let mut root = cjson_create_object();
    let name = cjson_create_string("Jack (\"Bee\") Nimble");
    cjson_add_item_to_object(root, "name", name);
    let fmt = cjson_create_object();
    cjson_add_item_to_object(root, "format", fmt);
    cjson_add_string_to_object(fmt, "type", "rect");
    cjson_add_number_to_object(fmt, "width", 1920.0);
    cjson_add_number_to_object(fmt, "height", 1080.0);
    cjson_add_false_to_object(fmt, "interlace");
    cjson_add_number_to_object(fmt, "frame rate", 24.0);

    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // Days of the week (string array)
    // Need to pass *const c_char[]
    let cstrs: Vec<CString> = strings.iter().map(|s| CString::new(*s).unwrap()).collect();
    let cptrs: Vec<*const c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
    root = cjson_create_string_array_c(&cptrs);
    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // Matrix
    root = cjson_create_array();
    for i in 0..3 {
        let row = cjson_create_int_array(&numbers[i]);
        add_item_to_array(root, row);
    }
    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // Gallery
    root = cjson_create_object();
    let img = cjson_create_object();
    cjson_add_item_to_object(root, "Image", img);
    cjson_add_number_to_object(img, "Width", 800.0);
    cjson_add_number_to_object(img, "Height", 600.0);
    cjson_add_string_to_object(img, "Title", "View from 15th Floor");
    let thm = cjson_create_object();
    cjson_add_item_to_object(img, "Thumbnail", thm);
    cjson_add_string_to_object(thm, "Url", "http:/*www.example.com/image/481989943");
    cjson_add_number_to_object(thm, "Height", 125.0);
    cjson_add_string_to_object(thm, "Width", "100");
    let ids_arr = cjson_create_int_array(ids);
    cjson_add_item_to_object(img, "IDs", ids_arr);

    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // Records
    root = cjson_create_array();
    for i in 0..2 {
        let fld = cjson_create_object();
        add_item_to_array(root, fld);
        cjson_add_string_to_object(fld, "precision", fields[i].precision);
        cjson_add_number_to_object(fld, "Latitude", fields[i].lat);
        cjson_add_number_to_object(fld, "Longitude", fields[i].lon);
        cjson_add_string_to_object(fld, "Address", fields[i].address);
        cjson_add_string_to_object(fld, "City", fields[i].city);
        cjson_add_string_to_object(fld, "State", fields[i].state);
        cjson_add_string_to_object(fld, "Zip", fields[i].zip);
        cjson_add_string_to_object(fld, "Country", fields[i].country);
    }
    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // Number = 1.0/zero (infinity → null)
    root = cjson_create_object();
    cjson_add_number_to_object(root, "number", 1.0 / zero);
    if print_preallocated(root) != 0 {
        cjson_delete(root);
        std::process::exit(1);
    }
    cjson_delete(root);

    // suppress unused warning
    let _ = Rc::new(RefCell::new(0u8));
}

unsafe fn driver(strings: &[&str; 7], numbers: &[[i32; 3]; 3], ids: &[i32; 4], fields: &[Record; 2]) -> i32 {
    println!("Version: {}", cjson_version());
    create_objects(strings, numbers, ids, fields);
    0
}

fn main() {
    let strings: [&str; 7] = [
        "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
    ];
    let numbers: [[i32; 3]; 3] = [[0, -1, 0], [1, 0, 0], [0, 0, 1]];
    let ids: [i32; 4] = [116, 943, 234, 38793];
    let fields: [Record; 2] = [
        Record {
            precision: "zip",
            lat: 37.7668,
            lon: -122.3959,
            address: "",
            city: "SAN FRANCISCO",
            state: "CA",
            zip: "94107",
            country: "US",
        },
        Record {
            precision: "zip",
            lat: 37.371991,
            lon: -122.026020,
            address: "",
            city: "SUNNYVALE",
            state: "CA",
            zip: "94085",
            country: "US",
        },
    ];
    unsafe {
        let _ = driver(&strings, &numbers, &ids, &fields);
    }
}
