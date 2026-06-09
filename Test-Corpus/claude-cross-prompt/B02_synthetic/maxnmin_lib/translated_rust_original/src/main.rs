use std::io::{self, Read};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone, Copy)]
struct Node {
    id: i32,
    parent_id: i32,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: i32,
}

impl Node {
    fn new() -> Self {
        Node {
            id: 0,
            parent_id: 0,
            name: [0u8; MAX_NAME_LEN],
            value: 0.0,
            active: 0,
        }
    }
}

struct Storage {
    nodes: [Node; MAX_NODES],
    count: usize,
}

impl Storage {
    fn new() -> Self {
        Storage {
            nodes: [Node::new(); MAX_NODES],
            count: 0,
        }
    }
}

fn add_node(storage: &mut Storage, id: i32, parent_id: i32, name: &str, value: f64) -> i32 {
    if storage.count >= MAX_NODES {
        return -1;
    }

    let mut new_node = Node {
        id,
        parent_id,
        name: [0u8; MAX_NAME_LEN],
        value,
        active: 1,
    };

    // Mimic strncpy(new_node.name, name, MAX_NAME_LEN - 1) and then NUL-terminate
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(MAX_NAME_LEN - 1);
    new_node.name[..copy_len].copy_from_slice(&bytes[..copy_len]);
    new_node.name[MAX_NAME_LEN - 1] = 0;

    storage.nodes[storage.count] = new_node;
    storage.count += 1;
    (storage.count as i32) - 1
}

fn find_node_by_id(storage: &Storage, id: i32) -> Option<usize> {
    for i in 0..storage.count {
        if storage.nodes[i].id == id && storage.nodes[i].active != 0 {
            return Some(i);
        }
    }
    None
}

fn get_children_count(storage: &Storage, parent_id: i32) -> i32 {
    let mut count = 0i32;
    for i in 0..storage.count {
        if storage.nodes[i].parent_id == parent_id && storage.nodes[i].active != 0 {
            count += 1;
        }
    }
    count
}

fn calculate_subtree_sum(storage: &Storage, node_id: i32) -> f64 {
    let idx = match find_node_by_id(storage, node_id) {
        Some(i) => i,
        None => return 0.0,
    };

    let mut sum = storage.nodes[idx].value;

    for i in 0..storage.count {
        if storage.nodes[i].parent_id == node_id && storage.nodes[i].active != 0 {
            sum += calculate_subtree_sum(storage, storage.nodes[i].id);
        }
    }

    sum
}

fn process_string(name: &[u8; MAX_NAME_LEN]) -> i32 {
    // Treat the buffer like a C string: sum bytes until NUL
    let mut result: i32 = 0;
    // The C code first checks `if (*str)` — if first byte is 0, returns 0 (which is the same anyway)
    if name[0] != 0 {
        let mut i = 0;
        while i < MAX_NAME_LEN && name[i] != 0 {
            // result += (int)(*str)  -- char in C is typically signed but on some platforms unsigned
            // For ASCII content (<128), this is the same.
            result = result.wrapping_add(name[i] as i8 as i32);
            i += 1;
        }
    }
    result
}

fn safe_double_to_int(d: f64) -> i32 {
    if d > i32::MAX as f64 {
        return i32::MAX;
    }
    if d < i32::MIN as f64 {
        return i32::MIN;
    }
    // NaN check: in C, `d != d` is true only for NaN
    if d.is_nan() {
        return 0;
    }
    // (int)d in C truncates toward zero for in-range non-NaN values.
    // Rust `as i32` for f64 does saturating cast, but values are already in range.
    d as i32
}

fn maxnmin(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let mut storage = Storage::new();

    add_node(&mut storage, 1, -1, "root", 10.5);
    add_node(&mut storage, 2, 1, "child1", 20.7);
    add_node(&mut storage, 3, 1, "child2", 15.3);
    add_node(&mut storage, 4, 2, "grandchild1", 5.9);
    add_node(&mut storage, 5, 2, "grandchild2", 8.2);
    add_node(&mut storage, 6, 3, "grandchild3", 12.4);

    let node_id = (param1 % 6).wrapping_add(1);
    if let Some(idx) = find_node_by_id(&storage, node_id) {
        let name_buf = storage.nodes[idx].name;
        if name_buf[0] != 0 {
            result = result.wrapping_add(process_string(&name_buf));
        }

        let subtree_sum = calculate_subtree_sum(&storage, node_id);
        let sum_as_int = safe_double_to_int(subtree_sum);
        result = result.wrapping_add(sum_as_int);
    }

    let second_node_id = (param2 % 6).wrapping_add(1);
    if let Some(idx) = find_node_by_id(&storage, second_node_id) {
        let value_multiplied = storage.nodes[idx].value * (param3 as f64);
        let converted_value = safe_double_to_int(value_multiplied);
        result = result.wrapping_add(converted_value);
    }

    let parent_id = (param4 % 3).wrapping_add(1);
    let children = get_children_count(&storage, parent_id);
    result = result.wrapping_add(children.wrapping_mul(10));

    // (double)(param1 + param2) -- C int addition wraps in practice
    let sum12 = param1.wrapping_add(param2) as f64;
    // (double)(param3 + 1)
    let sum3 = param3.wrapping_add(1) as f64;
    let mut calculation = sum12 / sum3;
    calculation *= param4 as f64;

    let final_calc = safe_double_to_int(calculation);
    result = result.wrapping_add(final_calc);

    result
}

fn read_all_stdin() -> String {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    buf
}

/// Mimic scanf("%d", ...): skip leading whitespace (including newlines),
/// then parse an optional sign and digits.
fn scan_int(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    if input[*pos] == b'+' || input[*pos] == b'-' {
        *pos += 1;
    }
    let digits_start = *pos;
    while *pos < input.len() && (input[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }
    if *pos == digits_start {
        return None;
    }

    let s = std::str::from_utf8(&input[start..*pos]).ok()?;
    // scanf wraps on overflow per C; use wrapping parse via i64 then cast.
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => None,
    }
}

fn main() {
    let buf = read_all_stdin();
    let bytes = buf.as_bytes();
    let mut pos = 0usize;

    let p1 = scan_int(bytes, &mut pos).unwrap_or(0);
    let p2 = scan_int(bytes, &mut pos).unwrap_or(0);
    let p3 = scan_int(bytes, &mut pos).unwrap_or(0);
    let p4 = scan_int(bytes, &mut pos).unwrap_or(0);

    let r = maxnmin(p1, p2, p3, p4);
    println!("{}", r);
}
