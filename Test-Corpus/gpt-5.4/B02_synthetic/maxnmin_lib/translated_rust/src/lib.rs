use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

#[derive(Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [u8; MAX_NAME_LEN],
    value: f64,
    active: bool,
}

impl Node {
    fn new(id: c_int, parent_id: c_int, name: &str, value: f64) -> Self {
        let mut name_buf = [0u8; MAX_NAME_LEN];
        let bytes = name.as_bytes();
        let copy_len = bytes.len().min(MAX_NAME_LEN.saturating_sub(1));
        name_buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
        Self {
            id,
            parent_id,
            name: name_buf,
            value,
            active: true,
        }
    }

    fn name_len(&self) -> usize {
        self.name.iter().position(|&b| b == 0).unwrap_or(MAX_NAME_LEN)
    }
}

struct Storage {
    nodes: Vec<Node>,
}

impl Storage {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn reset(&mut self) {
        self.nodes.clear();
    }

    fn add_node(&mut self, id: c_int, parent_id: c_int, name: &str, value: f64) -> c_int {
        if self.nodes.len() >= MAX_NODES {
            return -1;
        }
        self.nodes.push(Node::new(id, parent_id, name, value));
        (self.nodes.len() - 1) as c_int
    }

    fn find_node_by_id(&self, id: c_int) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id && n.active)
    }

    fn get_children_count(&self, parent_id: c_int) -> c_int {
        self.nodes
            .iter()
            .filter(|n| n.parent_id == parent_id && n.active)
            .count() as c_int
    }

    fn calculate_subtree_sum(&self, node_id: c_int) -> f64 {
        let Some(node) = self.find_node_by_id(node_id) else {
            return 0.0;
        };

        let mut sum = node.value;
        for child in self
            .nodes
            .iter()
            .filter(|n| n.parent_id == node_id && n.active)
        {
            sum += self.calculate_subtree_sum(child.id);
        }
        sum
    }
}

fn storage() -> &'static Mutex<Storage> {
    static STORAGE: OnceLock<Mutex<Storage>> = OnceLock::new();
    STORAGE.get_or_init(|| Mutex::new(Storage::new()))
}

fn process_string_bytes(bytes: &[u8]) -> c_int {
    bytes.iter().map(|&b| b as c_int).sum()
}

fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d < c_int::MIN as f64 {
        return c_int::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn maxnmin(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut storage = storage().lock().unwrap();
    storage.reset();

    storage.add_node(1, -1, "root", 10.5);
    storage.add_node(2, 1, "child1", 20.7);
    storage.add_node(3, 1, "child2", 15.3);
    storage.add_node(4, 2, "grandchild1", 5.9);
    storage.add_node(5, 2, "grandchild2", 8.2);
    storage.add_node(6, 3, "grandchild3", 12.4);

    let node_id = (param1 % 6) + 1;
    if let Some(selected_node) = storage.find_node_by_id(node_id) {
        let name_len = selected_node.name_len();
        if name_len > 0 {
            result += process_string_bytes(&selected_node.name[..name_len]);
        }

        let subtree_sum = storage.calculate_subtree_sum(node_id);
        let sum_as_int = safe_double_to_int(subtree_sum);
        result += sum_as_int;
    }

    let second_node_id = (param2 % 6) + 1;
    if let Some(second_node) = storage.find_node_by_id(second_node_id) {
        let value_multiplied = second_node.value * param3 as f64;
        let converted_value = safe_double_to_int(value_multiplied);
        result += converted_value;
    }

    let parent_id = (param4 % 3) + 1;
    let children = storage.get_children_count(parent_id);
    result += children * 10;

    let mut calculation = (param1 + param2) as f64 / (param3 + 1) as f64;
    calculation *= param4 as f64;

    let final_calc = safe_double_to_int(calculation);
    result += final_calc;

    result
}
