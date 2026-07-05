
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    rust_main_main()
}



use std::cell::RefCell;
use std::rc::Rc;

pub type tree_id_t = u64;

type NodeRef = Rc<RefCell<tree_node_t>>;

#[derive(Clone, Default)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: Option<NodeRef>,
    pub occupied: bool,
    pub deleted: bool,
}

pub struct hashmap_t {
    pub entries: Vec<hashmap_entry_t>,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

pub struct tree_t {
    pub node_map: hashmap_t,
    pub root_id: tree_id_t,
    pub has_root: bool,
    pub node_count: usize,
}

pub struct tree_node_t {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; MAX_CHILDREN as usize],
    pub child_count: i32,
    pub data: [u8; MAX_DATA_LENGTH as usize],
}

fn rust_hash_function(key: tree_id_t) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 14695981039346656037u64;
    for b in key.to_ne_bytes().iter() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

fn rust_find_slot(map: &hashmap_t, key: tree_id_t) -> Option<usize> {
    if map.capacity == 0 {
        return None;
    }
    let index = (rust_hash_function(key) as usize) % map.capacity;
    for probe in 0..map.capacity {
        let current = (index + probe) % map.capacity;
        let entry = &map.entries[current];
        if !entry.occupied {
            return None;
        }
        if !entry.deleted && entry.key == key {
            return Some(current);
        }
    }
    None
}

fn rust_hashmap_get(map: &hashmap_t, key: tree_id_t) -> Option<NodeRef> {
    rust_find_slot(map, key).and_then(|i| map.entries[i].value.clone())
}

fn rust_hashmap_contains(map: &hashmap_t, key: tree_id_t) -> bool {
    rust_find_slot(map, key).is_some()
}

fn rust_hashmap_create() -> hashmap_t {
    let capacity = HASHMAP_INITIAL_CAPACITY as usize;
    hashmap_t {
        entries: vec![hashmap_entry_t::default(); capacity],
        capacity,
        size: 0,
        deleted_count: 0,
    }
}

fn rust_should_resize(map: &hashmap_t) -> bool {
    let load = (map.size + map.deleted_count) as f64 / map.capacity as f64;
    load > HASHMAP_LOAD_FACTOR as f64
}

fn rust_hashmap_resize(map: &mut hashmap_t) -> Result<(), ()> {
    let old_capacity = map.capacity;
    let new_capacity = map.capacity * 2;
    let new_entries = vec![hashmap_entry_t::default(); new_capacity];
    let old_entries = std::mem::replace(&mut map.entries, new_entries);
    map.capacity = new_capacity;
    map.size = 0;
    map.deleted_count = 0;
    for i in 0..old_capacity {
        let entry = &old_entries[i];
        if entry.occupied && !entry.deleted {
            if let Some(val) = entry.value.clone() {
                rust_hashmap_put(map, entry.key, val)?;
            }
        }
    }
    Ok(())
}

fn rust_hashmap_put(map: &mut hashmap_t, key: tree_id_t, value: NodeRef) -> Result<(), ()> {
    if rust_should_resize(map) {
        rust_hashmap_resize(map)?;
    }
    let index = (rust_hash_function(key) as usize) % map.capacity;
    for probe in 0..map.capacity {
        let current = (index + probe) % map.capacity;
        let entry = &mut map.entries[current];
        if !entry.occupied {
            entry.key = key;
            entry.value = Some(value);
            entry.occupied = true;
            entry.deleted = false;
            map.size += 1;
            return Ok(());
        } else if entry.deleted {
            entry.key = key;
            entry.value = Some(value);
            entry.deleted = false;
            map.size += 1;
            map.deleted_count -= 1;
            return Ok(());
        } else if entry.key == key {
            entry.value = Some(value);
            return Ok(());
        }
    }
    Err(())
}

fn rust_hashmap_remove(map: &mut hashmap_t, key: tree_id_t) -> Option<NodeRef> {
    let slot = rust_find_slot(map, key)?;
    let value = map.entries[slot].value.take();
    map.entries[slot].deleted = true;
    map.size -= 1;
    map.deleted_count += 1;
    value
}

fn rust_hashmap_size(map: &hashmap_t) -> usize {
    map.size
}

fn rust_test_hashmap_basic() {
    println!("\n=== Testing Hashmap Basic Operations ===");
    let mut map = rust_hashmap_create();
    assert_eq!(rust_hashmap_size(&map), 0);

    let make_node = |v: i32| -> NodeRef {
        Rc::new(RefCell::new(tree_node_t {
            id: v as u64,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN as usize],
            child_count: v,
            data: [0u8; MAX_DATA_LENGTH as usize],
        }))
    };

    let val1 = make_node(42);
    let val2 = make_node(100);
    let val3 = make_node(200);
    assert!(rust_hashmap_put(&mut map, 1, val1.clone()).is_ok());
    assert!(rust_hashmap_put(&mut map, 2, val2.clone()).is_ok());
    assert!(rust_hashmap_put(&mut map, 3, val3.clone()).is_ok());
    assert_eq!(rust_hashmap_size(&map), 3);

    assert_eq!(rust_hashmap_get(&map, 1).unwrap().borrow().child_count, 42);
    assert_eq!(rust_hashmap_get(&map, 2).unwrap().borrow().child_count, 100);
    assert_eq!(rust_hashmap_get(&map, 3).unwrap().borrow().child_count, 200);

    let val4 = make_node(500);
    assert!(rust_hashmap_put(&mut map, 1, val4).is_ok());
    assert_eq!(rust_hashmap_size(&map), 3);
    assert_eq!(rust_hashmap_get(&map, 1).unwrap().borrow().child_count, 500);

    let removed = rust_hashmap_remove(&mut map, 2).expect("remove should return value");
    assert!(Rc::ptr_eq(&removed, &val2));
    assert_eq!(rust_hashmap_size(&map), 2);
    assert!(rust_hashmap_get(&map, 2).is_none());

    assert!(rust_hashmap_contains(&map, 1));
    assert!(!rust_hashmap_contains(&map, 2));
    assert!(rust_hashmap_contains(&map, 3));

    println!("\u{2713} PASS: test_hashmap_basic");
}

fn rust_test_hashmap_collisions() {
    println!("\n=== Testing Hashmap Collisions ===");
    let mut map = rust_hashmap_create();

    let mut values: Vec<NodeRef> = Vec::with_capacity(100);
    for i in 0..100i32 {
        let node = Rc::new(RefCell::new(tree_node_t {
            id: i as u64,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN as usize],
            child_count: i * 10,
            data: [0u8; MAX_DATA_LENGTH as usize],
        }));
        values.push(node.clone());
        assert!(rust_hashmap_put(&mut map, i as u64, node).is_ok());
    }

    assert_eq!(rust_hashmap_size(&map), 100);

    for i in 0..100i32 {
        let val = rust_hashmap_get(&map, i as u64).expect("value must exist");
        assert_eq!(val.borrow().child_count, i * 10);
    }

    println!("\u{2713} PASS: test_hashmap_collisions");
}

fn rust_tree_get_node(tree: &tree_t, id: tree_id_t) -> Option<NodeRef> {
    rust_hashmap_get(&tree.node_map, id)
}

fn rust_tree_contains(tree: &tree_t, id: tree_id_t) -> bool {
    rust_hashmap_contains(&tree.node_map, id)
}

fn rust_copy_data_to_array(data: Option<&str>) -> [u8; MAX_DATA_LENGTH as usize] {
    let mut arr = [0u8; MAX_DATA_LENGTH as usize];
    if let Some(d) = data {
        let bytes = d.as_bytes();
        let max = (MAX_DATA_LENGTH as usize).saturating_sub(1);
        let copy_len = bytes.len().min(max);
        arr[..copy_len].copy_from_slice(&bytes[..copy_len]);
    }
    arr
}

fn rust_tree_add_node(
    tree: &mut tree_t,
    id: tree_id_t,
    parent_id: tree_id_t,
    data: Option<&str>,
) -> Result<(), String> {
    if rust_tree_contains(tree, id) {
        let msg = format!("Error: Node with ID {} already exists", id);
        eprintln!("{}", msg);
        return Err(msg);
    }

    let data_arr = rust_copy_data_to_array(data);

    let effective_parent_id = if !tree.has_root {
        tree.root_id = id;
        tree.has_root = true;
        0
    } else {
        let parent = rust_tree_get_node(tree, parent_id).ok_or_else(|| {
            let msg = format!("Error: Parent node {} not found", parent_id);
            eprintln!("{}", msg);
            msg
        })?;

        let mut parent_mut = parent.borrow_mut();
        if parent_mut.child_count as u32 >= MAX_CHILDREN {
            let msg = "Error: Parent has maximum children".to_string();
            eprintln!("{}", msg);
            return Err(msg);
        }
        let idx = parent_mut.child_count as usize;
        parent_mut.child_ids[idx] = id;
        parent_mut.child_count += 1;
        parent_id
    };

    let node = Rc::new(RefCell::new(tree_node_t {
        id,
        parent_id: effective_parent_id,
        child_ids: [0; MAX_CHILDREN as usize],
        child_count: 0,
        data: data_arr,
    }));

    rust_hashmap_put(&mut tree.node_map, id, node).map_err(|_| {
        let msg = "Error: Failed to add node to hashmap".to_string();
        eprintln!("{}", msg);
        msg
    })?;

    tree.node_count += 1;
    Ok(())
}

fn rust_tree_create() -> tree_t {
    tree_t {
        node_map: rust_hashmap_create(),
        root_id: 0,
        has_root: false,
        node_count: 0,
    }
}

fn rust_tree_size(tree: &tree_t) -> usize {
    tree.node_count
}

fn rust_test_tree_add_children() {
    println!("\n=== Testing Tree Add Children ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("child1")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 3, 1, Some("child2")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 4, 1, Some("child3")).is_ok());

    assert_eq!(rust_tree_size(&tree), 4);

    let root = rust_tree_get_node(&tree, 1).expect("root exists");
    let root_b = root.borrow();
    assert_eq!(root_b.child_count, 3);
    assert_eq!(root_b.child_ids[0], 2);
    assert_eq!(root_b.child_ids[1], 3);
    assert_eq!(root_b.child_ids[2], 4);

    println!("\u{2713} PASS: test_tree_add_children");
}

fn rust_test_tree_add_root() {
    println!("\n=== Testing Tree Add Root ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert_eq!(rust_tree_size(&tree), 1);
    assert!(tree.has_root);
    assert_eq!(tree.root_id, 1);

    let root = rust_tree_get_node(&tree, 1).expect("root exists");
    let root_b = root.borrow();
    assert_eq!(root_b.id, 1);
    let end = root_b
        .data
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(root_b.data.len());
    let s = std::str::from_utf8(&root_b.data[..end]).unwrap_or("");
    assert_eq!(s, "root");
    assert_eq!(root_b.child_count, 0);

    println!("\u{2713} PASS: test_tree_add_root");
}

fn rust_tree_count_descendants(tree: &tree_t, id: tree_id_t) -> i32 {
    let Some(node) = rust_tree_get_node(tree, id) else {
        return -1;
    };
    let node_b = node.borrow();
    let child_count = node_b.child_count as usize;
    let child_ids = node_b.child_ids;
    drop(node_b);

    let mut count: i32 = 0;
    for &child_id in child_ids.iter().take(child_count) {
        count += 1;
        let sub = rust_tree_count_descendants(tree, child_id);
        if sub > 0 {
            count += sub;
        }
    }
    count
}

fn rust_tree_get_height(tree: &tree_t, id: tree_id_t) -> i32 {
    let Some(node) = rust_tree_get_node(tree, id) else {
        return -1;
    };
    let node_b = node.borrow();
    let child_count = node_b.child_count as usize;
    if child_count == 0 {
        return 0;
    }
    let child_ids = node_b.child_ids;
    drop(node_b);

    let max_height = child_ids
        .iter()
        .take(child_count)
        .map(|&cid| rust_tree_get_height(tree, cid))
        .max()
        .unwrap_or(0);
    max_height + 1
}

fn rust_tree_print_helper(tree: &tree_t, id: tree_id_t, depth: i32) {
    let Some(node) = rust_tree_get_node(tree, id) else {
        return;
    };
    let node_b = node.borrow();
    let indent = "  ".repeat(depth.max(0) as usize);
    let end = node_b
        .data
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(node_b.data.len());
    let s = std::str::from_utf8(&node_b.data[..end]).unwrap_or("");
    println!("{}[{}] {}", indent, node_b.id, s);
    let child_count = node_b.child_count as usize;
    let child_ids = node_b.child_ids;
    drop(node_b);

    for &child_id in child_ids.iter().take(child_count) {
        rust_tree_print_helper(tree, child_id, depth + 1);
    }
}

fn rust_tree_print(tree: Option<&tree_t>) {
    match tree {
        Some(t) if t.has_root => rust_tree_print_helper(t, t.root_id, 0),
        _ => println!("(empty tree)"),
    }
}

fn rust_test_tree_complex_structure() {
    println!("\n=== Testing Tree Complex Structure ===");
    let mut tree = rust_tree_create();

    let nodes: &[(tree_id_t, tree_id_t, &str)] = &[
        (1, 0, "root"),
        (2, 1, "child1"),
        (3, 1, "child2"),
        (4, 1, "child3"),
        (5, 2, "gc1"),
        (6, 2, "gc2"),
        (7, 3, "gc3"),
        (8, 4, "gc4"),
        (9, 4, "gc5"),
        (10, 7, "ggc1"),
    ];
    for &(id, parent, data) in nodes {
        assert!(rust_tree_add_node(&mut tree, id, parent, Some(data)).is_ok());
    }

    assert_eq!(rust_tree_size(&tree), 10);
    assert_eq!(rust_tree_get_height(&tree, 1), 3);
    assert_eq!(rust_tree_count_descendants(&tree, 1), 9);
    assert_eq!(rust_tree_count_descendants(&tree, 2), 2);
    assert_eq!(rust_tree_count_descendants(&tree, 7), 1);

    rust_tree_print(Some(&tree));

    println!("\u{2713} PASS: test_tree_complex_structure");
}

fn rust_test_tree_count_descendants() {
    println!("\n=== Testing Tree Count Descendants ===");
    let mut tree = rust_tree_create();

    let nodes: &[(tree_id_t, tree_id_t, &str)] = &[
        (1, 0, "root"),
        (2, 1, "child1"),
        (3, 2, "grandchild1"),
        (4, 2, "grandchild2"),
        (5, 1, "child2"),
    ];
    for &(id, parent, data) in nodes {
        assert!(rust_tree_add_node(&mut tree, id, parent, Some(data)).is_ok());
    }

    assert_eq!(rust_tree_count_descendants(&tree, 1), 4);
    assert_eq!(rust_tree_count_descendants(&tree, 2), 2);
    assert_eq!(rust_tree_count_descendants(&tree, 3), 0);
    assert_eq!(rust_tree_count_descendants(&tree, 5), 0);

    println!("\u{2713} PASS: test_tree_count_descendants");
}

fn rust_test_tree_creation() {
    println!("\n=== Testing Tree Creation ===");
    let tree = rust_tree_create();
    assert_eq!(rust_tree_size(&tree), 0);
    assert!(!tree.has_root);
    println!("\u{2713} PASS: test_tree_creation");
}

fn rust_tree_get_depth(tree: &tree_t, id: tree_id_t) -> i32 {
    if !rust_tree_contains(tree, id) {
        return -1;
    }
    let mut depth: i32 = 0;
    let mut current_id = id;
    while current_id != tree.root_id {
        let Some(node) = rust_tree_get_node(tree, current_id) else {
            return -1;
        };
        current_id = node.borrow().parent_id;
        depth += 1;
    }
    depth
}

fn rust_test_tree_deep_hierarchy() {
    println!("\n=== Testing Tree Deep Hierarchy ===");
    let mut tree = rust_tree_create();

    let levels: &[(tree_id_t, tree_id_t, &str)] = &[
        (1, 0, "level0"),
        (2, 1, "level1"),
        (3, 2, "level2"),
        (4, 3, "level3"),
        (5, 4, "level4"),
    ];
    for &(id, parent, data) in levels {
        assert!(rust_tree_add_node(&mut tree, id, parent, Some(data)).is_ok());
    }

    assert_eq!(rust_tree_size(&tree), 5);

    for (id, expected_depth) in (1..=5u64).zip(0..=4i32) {
        assert_eq!(rust_tree_get_depth(&tree, id), expected_depth);
    }

    assert_eq!(rust_tree_get_height(&tree, 1), 4);
    assert_eq!(rust_tree_get_height(&tree, 2), 3);
    assert_eq!(rust_tree_get_height(&tree, 5), 0);

    println!("\u{2713} PASS: test_tree_deep_hierarchy");
}

fn rust_test_tree_duplicate_id() {
    println!("\n=== Testing Tree Duplicate ID ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("child")).is_ok());

    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("duplicate")).is_err());
    assert_eq!(rust_tree_size(&tree), 2);

    println!("\u{2713} PASS: test_tree_duplicate_id");
}

fn rust_tree_find_path(
    tree: &tree_t,
    id: tree_id_t,
    path: &mut [tree_id_t],
    max_length: i32,
) -> i32 {
    if !rust_tree_contains(tree, id) {
        return -1;
    }

    let mut temp_path: Vec<tree_id_t> = Vec::new();
    let mut current_id = id;
    while temp_path.len() < 1000 {
        temp_path.push(current_id);
        if current_id == tree.root_id {
            break;
        }
        let Some(node) = rust_tree_get_node(tree, current_id) else {
            return -1;
        };
        current_id = node.borrow().parent_id;
    }

    let max_length = max_length.max(0) as usize;
    let length = temp_path.len().min(max_length).min(path.len());
    for (dst, src) in path.iter_mut().zip(temp_path.iter().rev()).take(length) {
        *dst = *src;
    }
    length as i32
}

fn rust_test_tree_find_path() {
    println!("\n=== Testing Tree Find Path ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("child")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 3, 2, Some("grandchild")).is_ok());

    let mut path: [tree_id_t; 10] = [0; 10];

    let length = rust_tree_find_path(&tree, 3, &mut path, 10);
    assert_eq!(length, 3);
    assert_eq!(&path[..3], &[1, 2, 3]);

    let length = rust_tree_find_path(&tree, 1, &mut path, 10);
    assert_eq!(length, 1);
    assert_eq!(path[0], 1);

    println!("\u{2713} PASS: test_tree_find_path");
}

fn rust_test_tree_max_children() {
    println!("\n=== Testing Tree Max Children ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());

    for i in 0..MAX_CHILDREN {
        assert!(rust_tree_add_node(&mut tree, i as u64 + 2, 1, Some("child")).is_ok());
    }

    assert!(rust_tree_add_node(&mut tree, MAX_CHILDREN as u64 + 2, 1, Some("overflow")).is_err());
    assert_eq!(rust_tree_size(&tree), MAX_CHILDREN as usize + 1);

    println!("\u{2713} PASS: test_tree_max_children");
}

fn rust_tree_remove_subtree(tree: &mut tree_t, id: tree_id_t) -> i32 {
    let Some(node) = rust_tree_get_node(tree, id) else {
        return -1;
    };
    let (child_count, child_ids) = {
        let node_b = node.borrow();
        (node_b.child_count as usize, node_b.child_ids)
    };
    drop(node);

    for &child_id in child_ids.iter().take(child_count) {
        rust_tree_remove_subtree(tree, child_id);
    }

    if rust_hashmap_remove(&mut tree.node_map, id).is_some() {
        tree.node_count -= 1;
    }
    0
}

fn rust_tree_remove_node(tree: &mut tree_t, id: tree_id_t) -> i32 {
    let Some(node) = rust_tree_get_node(tree, id) else {
        eprintln!("Error: Node {} not found", id);
        return -1;
    };

    if id == tree.root_id {
        drop(node);
        rust_tree_remove_subtree(tree, id);
        tree.has_root = false;
        tree.root_id = 0;
        return 0;
    }

    let parent_id = node.borrow().parent_id;
    drop(node);

    if let Some(parent) = rust_tree_get_node(tree, parent_id) {
        let mut parent_mut = parent.borrow_mut();
        let cc = parent_mut.child_count as usize;
        if let Some(pos) = parent_mut.child_ids[..cc].iter().position(|&c| c == id) {
            for j in pos..cc.saturating_sub(1) {
                parent_mut.child_ids[j] = parent_mut.child_ids[j + 1];
            }
            parent_mut.child_count -= 1;
        }
    }

    rust_tree_remove_subtree(tree, id);
    0
}

fn rust_test_tree_remove_leaf() {
    println!("\n=== Testing Tree Remove Leaf ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("child1")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 3, 1, Some("child2")).is_ok());

    assert_eq!(rust_tree_size(&tree), 3);

    assert_eq!(rust_tree_remove_node(&mut tree, 3), 0);
    assert_eq!(rust_tree_size(&tree), 2);
    assert!(!rust_tree_contains(&tree, 3));

    let root = rust_tree_get_node(&tree, 1).expect("root exists");
    let root_b = root.borrow();
    assert_eq!(root_b.child_count, 1);
    assert_eq!(root_b.child_ids[0], 2);

    println!("\u{2713} PASS: test_tree_remove_leaf");
}

fn rust_test_tree_remove_root() {
    println!("\n=== Testing Tree Remove Root ===");
    let mut tree = rust_tree_create();

    assert!(rust_tree_add_node(&mut tree, 1, 0, Some("root")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 2, 1, Some("child1")).is_ok());
    assert!(rust_tree_add_node(&mut tree, 3, 1, Some("child2")).is_ok());

    assert_eq!(rust_tree_size(&tree), 3);

    assert_eq!(rust_tree_remove_node(&mut tree, 1), 0);
    assert_eq!(rust_tree_size(&tree), 0);
    assert!(!tree.has_root);

    println!("\u{2713} PASS: test_tree_remove_root");
}

fn rust_test_tree_remove_subtree() {
    println!("\n=== Testing Tree Remove Subtree ===");
    let mut tree = rust_tree_create();

    let nodes: &[(tree_id_t, tree_id_t, &str)] = &[
        (1, 0, "root"),
        (2, 1, "child1"),
        (3, 2, "grandchild1"),
        (4, 2, "grandchild2"),
        (5, 1, "child2"),
    ];
    for &(id, parent, data) in nodes {
        assert!(rust_tree_add_node(&mut tree, id, parent, Some(data)).is_ok());
    }

    assert_eq!(rust_tree_size(&tree), 5);

    assert_eq!(rust_tree_remove_node(&mut tree, 2), 0);
    assert_eq!(rust_tree_size(&tree), 2);
    assert!(!rust_tree_contains(&tree, 2));
    assert!(!rust_tree_contains(&tree, 3));
    assert!(!rust_tree_contains(&tree, 4));
    assert!(rust_tree_contains(&tree, 1));
    assert!(rust_tree_contains(&tree, 5));

    println!("\u{2713} PASS: test_tree_remove_subtree");
}



#[unsafe(no_mangle)]
pub extern "C" fn rust_main_main() -> core::ffi::c_int {
    println!("╔════════════════════════════════════════╗");
    println!("║  TREE WITH HASHMAP ID MAPPING TESTS   ║");
    println!("╚════════════════════════════════════════╝");

    // Hashmap tests
    rust_test_hashmap_basic();
    rust_test_hashmap_collisions();

    // Tree creation tests
    rust_test_tree_creation();
    rust_test_tree_add_root();
    rust_test_tree_add_children();

    // Tree structure tests
    rust_test_tree_deep_hierarchy();
    rust_test_tree_complex_structure();

    // Tree removal tests
    rust_test_tree_remove_leaf();
    rust_test_tree_remove_subtree();
    rust_test_tree_remove_root();

    // Tree query tests
    rust_test_tree_count_descendants();
    rust_test_tree_find_path();

    // Error handling tests
    rust_test_tree_duplicate_id();
    rust_test_tree_max_children();

    println!();
    println!("========================================");
    println!("  All tests passed successfully!");
    println!("========================================");

    0
}
