//! Rust counterpart of `translation/tests/cprobe/probe.c`.
//!
//! `main.rs` only reaches the happy paths of `tree.rs`/`hashmap.rs`. This second
//! binary drives the remaining branches — NULL data, missing parents, missing
//! nodes, `MAX_DATA_LENGTH` truncation, `max_length` clamping in `find_path`,
//! tombstone reuse, resizing, and long pseudo-random operation mixes — so they
//! can be diffed against the C library compiled from the unmodified `c_src`.
//!
//! It exists purely for verification; `driver` is unaffected by it.

#[path = "../cout.rs"]
mod cout;
#[path = "../hashmap.rs"]
mod hashmap;
#[path = "../tree.rs"]
mod tree;

use cout::{init_c_runtime, out_flush, out_write};
use hashmap::{Hashmap, TreeId};
use tree::{Tree, MAX_CHILDREN, MAX_DATA_LENGTH};

// ------------------------------------------------------------------- helpers

static mut LCG_STATE: u64 = 0;

fn lcg_seed(s: u64) {
    unsafe { LCG_STATE = s };
}

fn lcg_next() -> u64 {
    unsafe {
        LCG_STATE = LCG_STATE
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        LCG_STATE >> 16
    }
}

/// Mirrors `dump_int_map`. Values are stored as indices, which is what the C
/// probe prints (`value - base`).
fn dump_int_map(map: &Hashmap<usize>) {
    c_printf!(
        "map size={} capacity={} deleted={}\n",
        map.size,
        map.capacity,
        map.deleted_count
    );
    for i in 0..map.capacity {
        let e = &map.entries[i];
        if e.occupied == 0 {
            c_printf!("  [{}] empty\n", i);
        } else if e.deleted != 0 {
            c_printf!("  [{}] key={} tombstone\n", i, e.key);
        } else {
            c_printf!("  [{}] key={} val={}\n", i, e.key, e.value.unwrap() as i64);
        }
    }
}

fn dump_tree(t: &Tree) {
    c_printf!(
        "tree count={} root={} has_root={} map_size={} map_cap={} map_deleted={}\n",
        t.node_count,
        t.root_id,
        t.has_root,
        t.node_map.size,
        t.node_map.capacity,
        t.node_map.deleted_count
    );
    for i in 0..t.node_map.capacity {
        let e = &t.node_map.entries[i];
        if e.occupied == 0 || e.deleted != 0 {
            continue;
        }
        let n = &t.nodes[e.value.unwrap()];
        c_printf!(
            "  slot={} id={} parent={} nchild={} data='",
            i,
            n.id,
            n.parent_id,
            n.child_count
        );
        out_write(n.data_bytes());
        out_write(b"' children=[");
        for j in 0..n.child_count {
            c_printf!(
                "{}{}",
                if j != 0 { "," } else { "" },
                n.child_ids[j as usize]
            );
        }
        c_printf!("]\n");
    }
    c_printf!("print:\n");
    t.print();
}

fn show_path(t: &Tree, id: TreeId, buf: &mut [TreeId], max_length: i32) {
    let cap = buf.len();
    let n = t.find_path(id, buf, max_length);
    c_printf!("find_path(id={}, max={}) = {} [", id, max_length, n);
    let mut i = 0;
    while i < n && (i as usize) < cap {
        c_printf!("{}{}", if i != 0 { "," } else { "" }, buf[i as usize]);
        i += 1;
    }
    c_printf!("]\n");
}

fn show_queries(t: &Tree, id: TreeId) {
    c_printf!(
        "id={} contains={} depth={} height={} descendants={}\n",
        id,
        t.contains(id),
        t.get_depth(id),
        t.get_height(id),
        t.count_descendants(id)
    );
}

fn repeat(c: u8, n: usize) -> String {
    String::from_utf8(vec![c; n]).unwrap()
}

// ----------------------------------------------------------------- scenarios

fn sc_empty_print() {
    let t = Tree::create();
    c_printf!(
        "size={} has_root={} root_id={}\n",
        t.size(),
        t.has_root,
        t.root_id
    );
    t.print();
    dump_tree(&t);
    t.delete();
}

fn sc_null_data() {
    let mut t = Tree::create();
    c_printf!("add={}\n", t.add_node(7, 0, None));
    c_printf!("add_child={}\n", t.add_node(8, 7, None));
    dump_tree(&t);
    t.delete();
}

fn sc_data_lengths() {
    let lens: [usize; 7] = [0, 1, 254, 255, 256, 300, 1024];
    for (k, &len) in lens.iter().enumerate() {
        let mut t = Tree::create();
        let s = repeat(b'A' + (k % 26) as u8, len);
        c_printf!("len={} add={}\n", len, t.add_node(1, 0, Some(&s)));
        let n = t.get_node(1).unwrap();
        // `strlen(n->data)` is the length up to the first NUL byte.
        c_printf!("  strlen={} data='", n.data_bytes().len());
        out_write(n.data_bytes());
        out_write(b"'\n");
        c_printf!(
            "  last={} byte254={}\n",
            n.data[MAX_DATA_LENGTH - 1] as i32,
            n.data[MAX_DATA_LENGTH - 2] as i32
        );
        t.print();
        t.delete();
    }
}

fn sc_parent_missing() {
    let mut t = Tree::create();
    c_printf!("root={}\n", t.add_node(1, 0, Some("root")));
    c_printf!("orphan={}\n", t.add_node(2, 99, Some("orphan")));
    c_printf!("orphan_zero={}\n", t.add_node(3, 0, Some("parent-zero")));
    c_printf!("size={}\n", t.size());
    dump_tree(&t);
    t.delete();
}

fn sc_duplicate_ids() {
    let mut t = Tree::create();
    c_printf!("root={}\n", t.add_node(1, 0, Some("root")));
    c_printf!("dup_root={}\n", t.add_node(1, 0, Some("again")));
    c_printf!("child={}\n", t.add_node(2, 1, Some("child")));
    c_printf!("dup_child={}\n", t.add_node(2, 1, Some("again")));
    c_printf!(
        "dup_child_other_parent={}\n",
        t.add_node(2, 2, Some("again"))
    );
    c_printf!("size={}\n", t.size());
    dump_tree(&t);
    t.delete();
}

fn sc_remove_missing() {
    let mut t = Tree::create();
    c_printf!("empty_remove={}\n", t.remove_node(1));
    c_printf!("root={}\n", t.add_node(1, 0, Some("root")));
    c_printf!("missing_remove={}\n", t.remove_node(42));
    c_printf!("zero_remove={}\n", t.remove_node(0));
    c_printf!("root_remove={}\n", t.remove_node(1));
    c_printf!("again={}\n", t.remove_node(1));
    dump_tree(&t);
    t.delete();
}

fn sc_queries_missing() {
    let mut t = Tree::create();
    let mut buf = [0u64; 16];
    show_queries(&t, 1);
    show_path(&t, 1, &mut buf, 16);
    c_printf!("root={}\n", t.add_node(1, 0, Some("root")));
    show_queries(&t, 1);
    show_queries(&t, 2);
    show_queries(&t, 0);
    show_path(&t, 2, &mut buf, 16);
    show_path(&t, 0, &mut buf, 16);
    t.delete();
}

fn sc_path_bounds() {
    let mut t = Tree::create();
    t.add_node(1, 0, Some("a"));
    t.add_node(2, 1, Some("b"));
    t.add_node(3, 2, Some("c"));
    t.add_node(4, 3, Some("d"));
    let mut buf = [0u64; 16];
    for max in -2..=6 {
        buf = [0u64; 16];
        show_path(&t, 4, &mut buf, max);
        c_printf!(
            "  buf=[{},{},{},{},{}]\n",
            buf[0],
            buf[1],
            buf[2],
            buf[3],
            buf[4]
        );
    }
    show_path(&t, 1, &mut buf, 1);
    t.delete();
}

fn sc_remove_root_then_add() {
    let mut t = Tree::create();
    t.add_node(10, 0, Some("root"));
    t.add_node(11, 10, Some("child"));
    t.add_node(12, 11, Some("grandchild"));
    dump_tree(&t);
    c_printf!("remove_root={}\n", t.remove_node(10));
    dump_tree(&t);
    c_printf!("readd={}\n", t.add_node(20, 999, Some("new-root")));
    dump_tree(&t);
    c_printf!("readd_child={}\n", t.add_node(21, 20, Some("new-child")));
    show_queries(&t, 20);
    show_queries(&t, 21);
    dump_tree(&t);
    t.delete();
}

fn sc_max_children() {
    let mut t = Tree::create();
    t.add_node(1, 0, Some("root"));
    for i in 0..MAX_CHILDREN {
        let rc = t.add_node((i + 2) as TreeId, 1, Some("child"));
        if rc != 0 {
            c_printf!("unexpected failure at {}\n", i);
        }
    }
    let before = t.size();
    let overflow = t.add_node(MAX_CHILDREN as TreeId + 2, 1, Some("overflow"));
    c_printf!("count={} overflow={}\n", before, overflow);
    c_printf!(
        "overflow2={}\n",
        t.add_node(MAX_CHILDREN as TreeId + 3, 1, Some("overflow"))
    );
    c_printf!("remove_first={}\n", t.remove_node(2));
    c_printf!(
        "refill={}\n",
        t.add_node(MAX_CHILDREN as TreeId + 2, 1, Some("refill"))
    );
    c_printf!(
        "overflow3={}\n",
        t.add_node(MAX_CHILDREN as TreeId + 4, 1, Some("overflow"))
    );
    let root = t.get_node(1).unwrap();
    c_printf!("root_children={}\n", root.child_count);
    for i in 0..root.child_count {
        c_printf!(
            "{}{}",
            if i != 0 { "," } else { "  " },
            root.child_ids[i as usize]
        );
    }
    c_printf!("\n");
    c_printf!(
        "height={} descendants={}\n",
        t.get_height(1),
        t.count_descendants(1)
    );
    t.delete();
}

fn sc_remove_child_positions() {
    let labels = ["first", "middle", "last"];
    let victims: [TreeId; 3] = [2, 4, 7];
    for k in 0..3 {
        let mut t = Tree::create();
        t.add_node(1, 0, Some("root"));
        for i in 2..=7u64 {
            t.add_node(i, 1, Some("child"));
        }
        c_printf!(
            "remove {} ({}) = {}\n",
            labels[k],
            victims[k],
            t.remove_node(victims[k])
        );
        let root = t.get_node(1).unwrap();
        c_printf!("  nchild={} [", root.child_count);
        for i in 0..root.child_count {
            c_printf!(
                "{}{}",
                if i != 0 { "," } else { "" },
                root.child_ids[i as usize]
            );
        }
        c_printf!(
            "] stale_slot={}\n",
            root.child_ids[root.child_count as usize]
        );
        dump_tree(&t);
        t.delete();
    }
}

fn sc_subtree_cascade() {
    let mut t = Tree::create();
    t.add_node(1, 0, Some("root"));
    t.add_node(2, 1, Some("a"));
    t.add_node(3, 2, Some("aa"));
    t.add_node(4, 3, Some("aaa"));
    t.add_node(5, 2, Some("ab"));
    t.add_node(6, 1, Some("b"));
    dump_tree(&t);
    c_printf!("remove_2={}\n", t.remove_node(2));
    dump_tree(&t);
    c_printf!("remove_3={}\n", t.remove_node(3));
    c_printf!("remove_6={}\n", t.remove_node(6));
    dump_tree(&t);
    c_printf!("remove_1={}\n", t.remove_node(1));
    dump_tree(&t);
    t.delete();
}

fn sc_id_zero() {
    let mut t = Tree::create();
    c_printf!("root_zero={}\n", t.add_node(0, 0, Some("zero-root")));
    c_printf!("child={}\n", t.add_node(1, 0, Some("child")));
    c_printf!("grand={}\n", t.add_node(2, 1, Some("grand")));
    show_queries(&t, 0);
    show_queries(&t, 1);
    show_queries(&t, 2);
    let mut buf = [0u64; 8];
    show_path(&t, 2, &mut buf, 8);
    show_path(&t, 0, &mut buf, 8);
    dump_tree(&t);
    c_printf!("remove_root={}\n", t.remove_node(0));
    dump_tree(&t);
    t.delete();
}

fn sc_big_ids() {
    let ids: [TreeId; 12] = [
        0,
        1,
        255,
        256,
        65535,
        4294967295,
        4294967296,
        9223372036854775807,
        9223372036854775808,
        18446744073709551615,
        0x0102030405060708,
        0x00000000000000FF,
    ];
    let mut t = Tree::create();
    c_printf!("root={}\n", t.add_node(ids[0], 0, Some("root")));
    for i in 1..ids.len() {
        c_printf!("add {} = {}\n", ids[i], t.add_node(ids[i], ids[0], Some("n")));
    }
    dump_tree(&t);
    for &id in ids.iter() {
        show_queries(&t, id);
    }
    c_printf!(
        "dup_max={}\n",
        t.add_node(18446744073709551615, ids[0], Some("d"))
    );
    t.delete();
}

fn sc_deep_chain() {
    let mut t = Tree::create();
    let depth: TreeId = 1100;
    c_printf!("root={}\n", t.add_node(1, 0, Some("n1")));
    for i in 2..=depth {
        let data = format!("n{}", i);
        if t.add_node(i, i - 1, Some(&data)) != 0 {
            c_printf!("add failed at {}\n", i);
            break;
        }
    }
    c_printf!("size={}\n", t.size());
    c_printf!(
        "depth_last={} height_root={} descendants_root={}\n",
        t.get_depth(depth),
        t.get_height(1),
        t.count_descendants(1)
    );
    let mut buf = vec![0u64; 2048];
    let n = t.find_path(depth, &mut buf, 2048);
    c_printf!(
        "path_len={} first={} last={}\n",
        n,
        buf[0],
        buf[(n - 1) as usize]
    );
    let n = t.find_path(500, &mut buf, 2048);
    c_printf!(
        "path500_len={} first={} last={}\n",
        n,
        buf[0],
        buf[(n - 1) as usize]
    );
    c_printf!("remove_mid={}\n", t.remove_node(550));
    c_printf!("size={} height_root={}\n", t.size(), t.get_height(1));
    c_printf!("depth_last={}\n", t.get_depth(depth));
    t.delete();
}

fn sc_clear_map() {
    let mut map: Hashmap<usize> = Hashmap::create();
    for i in 0..12usize {
        map.put(i as TreeId, i);
    }
    map.remove(3);
    dump_int_map(&map);
    map.clear();
    c_printf!(
        "after clear: size={} contains5={} has5={}\n",
        map.size(),
        map.contains(5),
        if map.get(5).is_some() { 1 } else { 0 }
    );
    dump_int_map(&map);
    map.put(5, 7);
    dump_int_map(&map);
    map.destroy();
}

fn sc_tombstones() {
    let mut map: Hashmap<usize> = Hashmap::create();
    for k in 0..10u64 {
        map.put(k, k as usize);
    }
    dump_int_map(&map);
    let mut k = 0u64;
    while k < 10 {
        let v = map.remove(k);
        c_printf!(
            "remove {} -> {}\n",
            k,
            v.map(|x| x as i64).unwrap_or(-1)
        );
        k += 2;
    }
    dump_int_map(&map);
    for k in 0..10u64 {
        // Sequenced to match the C probe, which cannot rely on printf argument
        // evaluation order.
        let rc = map.put(k, k as usize + 20);
        let sz = map.size();
        let del = map.deleted_count;
        c_printf!("put {} -> {} (size={} deleted={})\n", k, rc, sz, del);
    }
    dump_int_map(&map);
    for k in 0..10u64 {
        let v = map.get(k);
        c_printf!(
            "get {} -> {} contains={}\n",
            k,
            v.map(|x| x as i64).unwrap_or(-1),
            map.contains(k)
        );
    }
    for k in 0..10u64 {
        let a = map.remove(k);
        let b = map.remove(k);
        c_printf!(
            "double remove {} -> {} / {}\n",
            k,
            a.map(|x| x as i64).unwrap_or(-1),
            b.map(|x| x as i64).unwrap_or(-1)
        );
    }
    dump_int_map(&map);
    map.destroy();
}

/// Which slot a key lands in when it is the only key in a fresh map, i.e. its
/// hash modulo the initial capacity. Derived empirically, mirroring the C probe,
/// because `hash_function` is private to hashmap.c.
fn home_slot(key: TreeId) -> usize {
    let mut m: Hashmap<usize> = Hashmap::create();
    m.put(key, 0);
    let mut slot = 0usize;
    for i in 0..m.capacity {
        if m.entries[i].occupied != 0 {
            slot = i;
            break;
        }
    }
    m.destroy();
    slot
}

fn sc_collision_probing() {
    let mut homes = [0usize; 400];
    for k in 0..400u64 {
        homes[k as usize] = home_slot(k);
    }
    let mut a: TreeId = 0;
    let mut b: TreeId = 0;
    let mut found = 0i32;
    for x in 0..400u64 {
        if found != 0 {
            break;
        }
        for y in (x + 1)..400u64 {
            if homes[x as usize] == homes[y as usize] {
                a = x;
                b = y;
                found = 1;
                break;
            }
        }
    }
    c_printf!(
        "found={} a={} b={} home={}\n",
        found,
        a,
        b,
        homes[a as usize]
    );

    let mut map: Hashmap<usize> = Hashmap::create();
    let pa = map.put(a, 1);
    let pb = map.put(b, 2);
    c_printf!("put a={} put b={}\n", pa, pb);
    dump_int_map(&map);

    let ra = map.remove(a);
    c_printf!("remove a -> {}\n", ra.map(|x| x as i64).unwrap_or(-1));
    let gb = map.get(b);
    c_printf!(
        "get b past tombstone -> {} contains={} size={}\n",
        gb.map(|x| x as i64).unwrap_or(-1),
        map.contains(b),
        map.size()
    );
    dump_int_map(&map);

    let rc = map.put(b, 3);
    let sz = map.size();
    c_printf!("put b again -> {} size={}\n", rc, sz);
    dump_int_map(&map);
    let g2 = map.get(b);
    c_printf!("get b -> {}\n", g2.map(|x| x as i64).unwrap_or(-1));

    let r1 = map.remove(b);
    c_printf!(
        "remove b -> {} then get -> ",
        r1.map(|x| x as i64).unwrap_or(-1)
    );
    let g3 = map.get(b);
    c_printf!(
        "{} size={}\n",
        g3.map(|x| x as i64).unwrap_or(-1),
        map.size()
    );
    dump_int_map(&map);
    let r2 = map.remove(b);
    c_printf!(
        "remove b -> {} then get -> ",
        r2.map(|x| x as i64).unwrap_or(-1)
    );
    let g4 = map.get(b);
    c_printf!(
        "{} size={}\n",
        g4.map(|x| x as i64).unwrap_or(-1),
        map.size()
    );
    dump_int_map(&map);

    for k in 0..40u64 {
        map.put(1000 + k, (k % 64) as usize);
    }
    c_printf!(
        "after fill size={} capacity={} deleted={}\n",
        map.size(),
        map.capacity,
        map.deleted_count
    );
    c_printf!("get a -> {} get b -> {}\n", map.contains(a), map.contains(b));
    dump_int_map(&map);
    map.destroy();
}

fn sc_resize_map() {
    let mut map: Hashmap<usize> = Hashmap::create();
    for i in 0..200usize {
        let rc = map.put(i as TreeId, i);
        if rc != 0 || map.capacity != 16 {
            c_printf!(
                "i={} rc={} size={} capacity={}\n",
                i,
                rc,
                map.size(),
                map.capacity
            );
        }
        if i % 25 == 0 {
            c_printf!(
                "i={} size={} capacity={} deleted={}\n",
                i,
                map.size(),
                map.capacity,
                map.deleted_count
            );
        }
    }
    dump_int_map(&map);
    let mut i = 0usize;
    while i < 200 {
        map.remove(i as TreeId);
        i += 2;
    }
    c_printf!(
        "after deletes size={} capacity={} deleted={}\n",
        map.size(),
        map.capacity,
        map.deleted_count
    );
    for i in 200..400usize {
        map.put(i as TreeId, i);
    }
    c_printf!(
        "after refill size={} capacity={} deleted={}\n",
        map.size(),
        map.capacity,
        map.deleted_count
    );
    dump_int_map(&map);
    for i in 200..400usize {
        map.put(i as TreeId, i - 200);
    }
    c_printf!(
        "after updates size={} capacity={} deleted={}\n",
        map.size(),
        map.capacity,
        map.deleted_count
    );
    map.destroy();
}

fn sc_stress_map() {
    lcg_seed(0x0123456789ABCDEF);
    let mut map: Hashmap<usize> = Hashmap::create();
    for step in 0..4000 {
        let r = lcg_next();
        let op = (r % 4) as i32;
        let key: TreeId = (r >> 8) % 96;
        let vidx = ((r >> 24) % 512) as usize;
        if op <= 1 {
            c_printf!("{} put {} {} -> {}\n", step, key, vidx, map.put(key, vidx));
        } else if op == 2 {
            let v = map.remove(key);
            c_printf!("{} rm {} -> {}\n", step, key, v.map(|x| x as i64).unwrap_or(-1));
        } else {
            let v = map.get(key);
            c_printf!(
                "{} get {} -> {} c={} s={}\n",
                step,
                key,
                v.map(|x| x as i64).unwrap_or(-1),
                map.contains(key),
                map.size()
            );
        }
        if step % 1000 == 0 {
            dump_int_map(&map);
        }
    }
    dump_int_map(&map);
    map.destroy();
}

fn sc_stress_tree() {
    lcg_seed(0xFEEDFACECAFEBEEF);
    let mut t = Tree::create();
    c_printf!("root={}\n", t.add_node(1, 0, Some("root")));
    let mut next_id: TreeId = 2;
    let mut buf = [0u64; 64];
    for step in 0..1500 {
        let r = lcg_next();
        let op = (r % 8) as i32;
        if op <= 3 {
            let id: TreeId = if (r >> 3) % 5 == 0 {
                (r >> 16) % next_id
            } else {
                next_id
            };
            let parent: TreeId = (r >> 8) % (next_id + 3);
            let data = format!("n{}", id);
            let rc = t.add_node(id, parent, Some(&data));
            let sz = t.size();
            c_printf!("{} add id={} p={} -> {} size={}\n", step, id, parent, rc, sz);
            next_id += 1;
        } else if op == 4 || op == 5 {
            let id: TreeId = (r >> 8) % (next_id + 3);
            let rc = t.remove_node(id);
            let sz = t.size();
            let hr = t.has_root;
            let root = t.root_id;
            c_printf!(
                "{} rm id={} -> {} size={} has_root={} root={}\n",
                step,
                id,
                rc,
                sz,
                hr,
                root
            );
        } else {
            let id: TreeId = (r >> 8) % (next_id + 3);
            show_queries(&t, id);
            show_path(&t, id, &mut buf, ((r >> 20) % 8) as i32);
        }
        if step % 250 == 0 {
            dump_tree(&t);
        }
    }
    dump_tree(&t);
    t.delete();
}

// ------------------------------------------------------------------ dispatch

fn main() {
    init_c_runtime();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        crate::c_eprintf!("usage: probe <scenario>\n");
        out_flush();
        std::process::exit(2);
    }

    let scenarios: &[(&str, fn())] = &[
        ("empty_print", sc_empty_print),
        ("null_data", sc_null_data),
        ("data_lengths", sc_data_lengths),
        ("parent_missing", sc_parent_missing),
        ("duplicate_ids", sc_duplicate_ids),
        ("remove_missing", sc_remove_missing),
        ("queries_missing", sc_queries_missing),
        ("path_bounds", sc_path_bounds),
        ("remove_root_then_add", sc_remove_root_then_add),
        ("max_children", sc_max_children),
        ("remove_child_positions", sc_remove_child_positions),
        ("subtree_cascade", sc_subtree_cascade),
        ("id_zero", sc_id_zero),
        ("big_ids", sc_big_ids),
        ("deep_chain", sc_deep_chain),
        ("clear_map", sc_clear_map),
        ("tombstones", sc_tombstones),
        ("collision_probing", sc_collision_probing),
        ("resize_map", sc_resize_map),
        ("stress_map", sc_stress_map),
        ("stress_tree", sc_stress_tree),
    ];

    for (name, run) in scenarios {
        if *name == args[1] {
            run();
            out_flush();
            return;
        }
    }

    crate::c_eprintf!("unknown scenario: {}\n", args[1]);
    out_flush();
    std::process::exit(3);
}
