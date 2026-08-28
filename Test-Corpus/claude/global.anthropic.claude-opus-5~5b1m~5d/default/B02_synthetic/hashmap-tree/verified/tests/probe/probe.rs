//! Branch-coverage probe, Rust side.
//!
//! Mirrors tests/probe/probe.c statement for statement. It is compiled by
//! `branch_coverage.rs` into its own executable that includes the translation's
//! modules as source; the two probe *executables* are then run as subprocesses
//! and their stdout/stderr/exit status compared. The graded `driver` binary is
//! unaffected by this file.

#[macro_use]
#[path = "../../src/cio.rs"]
mod cio;
#[path = "../../src/hashmap.rs"]
mod hashmap;
#[path = "../../src/tree.rs"]
mod tree;

use hashmap::{Hashmap, TreeId};
use tree::{Tree, MAX_CHILDREN};

const NVALS: usize = 64;

/// The C probe fills `VALS[i] = i * 7 + 1` and stores `&VALS[i]` in the map.
fn vals(i: usize) -> i32 {
    assert!(i < NVALS);
    (i as i32) * 7 + 1
}

/// The C probe stores `&VALS[i]`; here the map stores `i` itself, so a stored
/// value is `Some(i)` and a NULL pointer is `None`.
type Map = Hashmap<usize>;

// ---------------- hashmap helpers ----------------

fn hm_state(m: &Map, tag: &str) {
    c_printf!(
        "{}: size={} cap={} del={}\n",
        tag,
        m.size,
        m.capacity,
        m.deleted_count
    );
}

fn hm_get(m: &Map, k: TreeId) {
    match m.get(k) {
        Some(i) => c_printf!("  get({})={} contains={}\n", k, vals(i), m.contains(k)),
        None => c_printf!("  get({})=(null) contains={}\n", k, m.contains(k)),
    }
}

fn hm_put(m: &mut Map, k: TreeId, vi: i32) {
    let rc = if vi < 0 {
        m.put_value(k, None)
    } else {
        m.put(k, vi as usize)
    };
    c_printf!(
        "  put({}, {})={} size={} cap={} del={}\n",
        k,
        if vi < 0 { "NULL" } else { "val" },
        rc,
        m.size,
        m.capacity,
        m.deleted_count
    );
}

fn hm_remove(m: &mut Map, k: TreeId) {
    match m.remove(k) {
        Some(i) => c_printf!(
            "  remove({})={} size={} del={}\n",
            k,
            vals(i),
            m.size,
            m.deleted_count
        ),
        None => c_printf!(
            "  remove({})=(null) size={} del={}\n",
            k,
            m.size,
            m.deleted_count
        ),
    }
}

// ---------------- tree helpers ----------------

fn t_state(t: &Tree, tag: &str) {
    c_printf!(
        "{}: size={} has_root={} root_id={}\n",
        tag,
        t.size(),
        t.has_root,
        t.root_id
    );
}

fn t_node(t: &Tree, id: TreeId) {
    let idx = match t.get_node(id) {
        Some(i) => i,
        None => {
            c_printf!("  node({})=(null)\n", id);
            return;
        }
    };
    let n = t.node(idx);
    c_printf!(
        "  node({}): parent={} nchild={} [",
        n.id,
        n.parent_id,
        n.child_count
    );
    for i in 0..n.child_count {
        c_printf!(
            "{}{}",
            if i != 0 { "," } else { "" },
            n.child_ids[i as usize]
        );
    }
    let data = n.data_cstr();
    c_printf!("] datalen={} data=\"", data.len());
    cio::out_bytes(data);
    c_printf!("\"\n");
}

fn t_query(t: &Tree, id: TreeId) {
    c_printf!(
        "  q({}): contains={} depth={} height={} desc={}\n",
        id,
        t.contains(id),
        t.get_depth(id),
        t.get_height(id),
        t.count_descendants(id)
    );
}

fn t_path(t: &Tree, id: TreeId, max_len: i32) {
    let mut path: [TreeId; 64] = [0; 64];
    let n = t.find_path(id, &mut path, max_len);
    c_printf!("  path({}, max={})={} [", id, max_len, n);
    let mut i = 0;
    while i < n && i < 64 {
        c_printf!("{}{}", if i != 0 { "," } else { "" }, path[i as usize]);
        i += 1;
    }
    c_printf!("]\n");
}

fn t_add(t: &mut Tree, id: TreeId, parent: TreeId, data: Option<&str>) {
    let rc = t.add_node(id, parent, data);
    c_printf!(
        "  add(id={},parent={},data={})={} size={}\n",
        id,
        parent,
        if data.is_some() { "str" } else { "NULL" },
        rc,
        t.size()
    );
}

fn t_remove(t: &mut Tree, id: TreeId) {
    let rc = t.remove_node(id);
    c_printf!(
        "  remove({})={} size={} has_root={} root_id={}\n",
        id,
        rc,
        t.size(),
        t.has_root,
        t.root_id
    );
}

// ================= sections =================

fn sec_hashmap_growth() {
    c_printf!("\n### hashmap growth ###\n");
    let mut m: Map = Hashmap::create();
    hm_state(&m, "fresh");

    for i in 0..31 {
        hm_put(&mut m, i as TreeId, i);
    }
    hm_state(&m, "after 31 puts");

    for i in 0..34 {
        hm_get(&m, i as TreeId);
    }

    hm_put(&mut m, 3, 20);
    hm_get(&m, 3);
    hm_state(&m, "after update");
}

fn sec_hashmap_deletion() {
    c_printf!("\n### hashmap deletion / reuse / clear ###\n");
    let mut m: Map = Hashmap::create();

    for i in 0..10 {
        hm_put(&mut m, i as TreeId, i);
    }
    hm_state(&m, "10 keys");

    hm_remove(&mut m, 0);
    hm_remove(&mut m, 4);
    hm_remove(&mut m, 9);
    hm_remove(&mut m, 9);
    hm_remove(&mut m, 777);
    hm_state(&m, "after removes");

    for i in 0..11 {
        hm_get(&m, i as TreeId);
    }

    hm_put(&mut m, 4, 11);
    hm_put(&mut m, 0, 12);
    hm_put(&mut m, 9, 13);
    hm_state(&m, "after reinsert");
    for i in 0..10 {
        hm_get(&m, i as TreeId);
    }

    hm_remove(&mut m, 1);
    hm_remove(&mut m, 2);
    hm_remove(&mut m, 3);
    for i in 10..25 {
        hm_put(&mut m, i as TreeId, i % NVALS as i32);
    }
    hm_state(&m, "after growth with tombstones");
    for i in 0..25 {
        hm_get(&m, i as TreeId);
    }

    hm_put(&mut m, 100, -1);
    hm_get(&m, 100);
    hm_state(&m, "after NULL value");
    hm_remove(&mut m, 100);
    hm_state(&m, "after removing NULL value");

    hm_put(&mut m, 0, 1);
    hm_put(&mut m, 18446744073709551615, 2);
    hm_put(&mut m, 9223372036854775808, 3);
    hm_get(&m, 18446744073709551615);
    hm_get(&m, 9223372036854775808);
    hm_state(&m, "after extreme keys");

    m.clear();
    hm_state(&m, "after clear");
    for i in 0..5 {
        hm_get(&m, i as TreeId);
    }
    hm_get(&m, 18446744073709551615);
    hm_put(&mut m, 5, 5);
    hm_get(&m, 5);
    hm_state(&m, "after put post-clear");
}

fn sec_tree_empty() {
    c_printf!("\n### tree: empty-state branches ###\n");
    let mut t = Tree::create();
    t_state(&t, "fresh");
    t.print();
    t_node(&t, 1);
    t_query(&t, 1);
    t_query(&t, 0);
    t_path(&t, 1, 10);
    t_path(&t, 0, 10);
    t_remove(&mut t, 1);
    t_remove(&mut t, 0);
}

fn sec_tree_add_paths() {
    c_printf!("\n### tree: add-node validation order ###\n");
    let mut t = Tree::create();

    t_add(&mut t, 10, 12345, None);
    t_state(&t, "after NULL-data root");
    t_node(&t, 10);
    t.print();

    t_add(&mut t, 11, 99, Some("orphan"));
    t_state(&t, "after bad parent");
    t_node(&t, 11);

    t_add(&mut t, 10, 99, Some("dup-bad-parent"));

    t_add(&mut t, 12, 12, Some("self"));

    let long_data: String = (0..399)
        .map(|i| (b'a' + (i % 26) as u8) as char)
        .collect();
    t_add(&mut t, 13, 10, Some(&long_data));
    t_node(&t, 13);

    let d255: String = std::iter::repeat('x').take(255).collect();
    t_add(&mut t, 14, 10, Some(&d255));
    t_node(&t, 14);

    let d256: String = std::iter::repeat('y').take(256).collect();
    t_add(&mut t, 15, 10, Some(&d256));
    t_node(&t, 15);

    t_add(&mut t, 16, 10, Some(""));
    t_node(&t, 16);

    t.print();
    t_state(&t, "final");
}

fn sec_tree_max_children() {
    c_printf!("\n### tree: MAX_CHILDREN boundary ###\n");
    let mut t = Tree::create();
    t_add(&mut t, 1, 0, Some("root"));
    for i in 0..MAX_CHILDREN {
        let rc = t.add_node((i + 2) as TreeId, 1, Some("c"));
        if rc != 0 {
            c_printf!("  unexpected failure at child {}\n", i);
        }
    }
    t_state(&t, "root full");
    t_add(&mut t, 100, 1, Some("overflow"));

    t_add(&mut t, 2, 1, Some("dup-on-full"));

    t_node(&t, 1);
    t_query(&t, 1);

    t_remove(&mut t, 17);
    t_node(&t, 1);
    t_add(&mut t, 100, 1, Some("now-fits"));
    t_node(&t, 1);
}

fn sec_tree_child_removal() {
    c_printf!("\n### tree: child-list shifting ###\n");
    let mut t = Tree::create();
    t_add(&mut t, 1, 0, Some("root"));
    for i in 2..=7 {
        t_add(&mut t, i as TreeId, 1, Some("c"));
    }
    t_node(&t, 1);

    t_remove(&mut t, 4);
    t_node(&t, 1);
    t_remove(&mut t, 2);
    t_node(&t, 1);
    t_remove(&mut t, 7);
    t_node(&t, 1);
    t_remove(&mut t, 4);
    t_node(&t, 1);

    t.print();
}

fn sec_tree_subtree_and_root() {
    c_printf!("\n### tree: subtree removal, root removal, re-add ###\n");
    let mut t = Tree::create();
    t_add(&mut t, 1, 0, Some("root"));
    t_add(&mut t, 2, 1, Some("a"));
    t_add(&mut t, 3, 2, Some("aa"));
    t_add(&mut t, 4, 3, Some("aaa"));
    t_add(&mut t, 5, 1, Some("b"));
    t_add(&mut t, 6, 5, Some("bb"));
    t.print();
    t_query(&t, 1);
    t_query(&t, 2);

    t_remove(&mut t, 2);
    t_state(&t, "after subtree removal");
    for id in 1..=6u64 {
        c_printf!("  contains({})={}\n", id, t.contains(id));
    }
    t.print();

    t_remove(&mut t, 1);
    t_state(&t, "after root removal");
    t.print();
    t_query(&t, 1);

    t_add(&mut t, 20, 0, Some("new-root"));
    t_add(&mut t, 21, 20, Some("new-child"));
    t_add(&mut t, 3, 21, Some("recycled-id"));
    t_state(&t, "after re-add");
    t.print();
    t_query(&t, 20);
    t_path(&t, 3, 10);
}

fn sec_tree_zero_and_max_ids() {
    c_printf!("\n### tree: id 0 and id UINT64_MAX ###\n");
    let mut t = Tree::create();
    t_add(&mut t, 0, 0, Some("zero-root"));
    t_state(&t, "zero root");
    t_add(&mut t, 18446744073709551615, 0, Some("max-child"));
    t_add(&mut t, 1, 18446744073709551615, Some("deep"));
    t.print();
    t_query(&t, 0);
    t_query(&t, 18446744073709551615);
    t_query(&t, 1);
    t_path(&t, 1, 10);
    t_node(&t, 18446744073709551615);
    t_remove(&mut t, 18446744073709551615);
    t_state(&t, "after removing max id");
    t.print();
}

fn sec_tree_deep_chain() {
    c_printf!("\n### tree: deep chain, path truncation, 1000-entry cap ###\n");
    let mut t = Tree::create();

    t_add(&mut t, 2000, 0, Some("chain-root"));
    for i in 1..1010 {
        let buf = format!("n{}", i);
        if t.add_node((2000 + i) as TreeId, (2000 + i - 1) as TreeId, Some(&buf)) != 0 {
            c_printf!("  chain add failed at {}\n", i);
            break;
        }
    }
    t_state(&t, "chain built");

    c_printf!("  depth(2000)={}\n", t.get_depth(2000));
    c_printf!("  depth(2500)={}\n", t.get_depth(2500));
    c_printf!("  depth(3009)={}\n", t.get_depth(3009));
    c_printf!("  height(2000)={}\n", t.get_height(2000));
    c_printf!("  height(3009)={}\n", t.get_height(3009));
    c_printf!("  desc(2000)={}\n", t.count_descendants(2000));
    c_printf!("  desc(3000)={}\n", t.count_descendants(3000));

    t_path(&t, 2005, 10);
    t_path(&t, 2005, 3);
    t_path(&t, 2005, 1);
    t_path(&t, 2005, 0);
    t_path(&t, 3009, 64);
    t_path(&t, 3009, 5);
    t_path(&t, 2000, 64);
    t_path(&t, 12345, 10);
}

fn sec_tree_wide_and_print() {
    c_printf!("\n### tree: wide fan-out printing ###\n");
    let mut t = Tree::create();
    t_add(&mut t, 1, 0, Some("root"));
    for i in 0..5 {
        t_add(&mut t, (10 + i) as TreeId, 1, Some("mid"));
        for j in 0..3 {
            t_add(
                &mut t,
                (100 + i * 10 + j) as TreeId,
                (10 + i) as TreeId,
                Some("leaf"),
            );
        }
    }
    t.print();
    t_query(&t, 1);
    for i in 0..5 {
        t_query(&t, (10 + i) as TreeId);
    }
    t_path(&t, 123, 10);
}

fn main() {
    cio::restore_default_sigpipe();
    c_printf!("=== BRANCH PROBE ===\n");
    sec_hashmap_growth();
    sec_hashmap_deletion();
    sec_tree_empty();
    sec_tree_add_paths();
    sec_tree_max_children();
    sec_tree_child_removal();
    sec_tree_subtree_and_root();
    sec_tree_zero_and_max_ids();
    sec_tree_deep_chain();
    sec_tree_wide_and_print();
    c_printf!("=== PROBE DONE ===\n");
    cio::flush();
    std::process::exit(0);
}
