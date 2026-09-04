// probe.rs -- the Rust counterpart of translation/tests/cprobe/probe.c.
//
// Same scenarios, same output format, driven the same way (argv[1]). This exists
// because c_src/src/main.c never reaches most branches in tree.c / hashmap.c and
// the driver has no input channel, so the only way to diff those paths is a
// second pair of executables over the same library code.
//
// The library modules are pulled in by path so that the graded `driver` binary
// (src/main.rs) is left exactly as it is.

#[macro_use]
#[path = "../cstdio.rs"]
mod cstdio;

#[path = "../hashmap.rs"]
mod hashmap;

#[path = "../tree.rs"]
mod tree;

use hashmap::{Hashmap, TreeId};
use tree::{Tree, MAX_CHILDREN};

/// See the note in src/main.rs: the Rust runtime ignores SIGPIPE, C does not.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

// ---------- shared dump helpers (mirror probe.c exactly) ----------

fn dump_map(map: &Hashmap, label: &str) {
    c_println!(
        "{}: size={} capacity={} deleted={}",
        label,
        map.size,
        map.capacity,
        map.deleted_count
    );
    for i in 0..map.capacity {
        if map.entries[i].occupied {
            c_println!(
                "  slot {} key={} occupied={} deleted={} value={}",
                i,
                map.entries[i].key,
                i32::from(map.entries[i].occupied),
                i32::from(map.entries[i].deleted),
                if map.entries[i].value.is_some() {
                    "set"
                } else {
                    "null"
                }
            );
        }
    }
}

fn dump_node(tree: &Tree, id: TreeId) {
    let n = match tree.node(id) {
        Some(n) => n,
        None => {
            c_println!("node {}: (null)", id);
            return;
        }
    };
    // `printf("... data='%s' ...")` writes the raw bytes up to the NUL, which
    // need not be valid UTF-8, so the data is emitted as bytes.
    cstdio::out(
        format!(
            "node {}: parent={} child_count={} data='",
            n.id, n.parent_id, n.child_count
        )
        .as_bytes(),
    );
    cstdio::out(n.data_cstr());
    cstdio::out(b"' children=[");
    for i in 0..n.child_count {
        if i != 0 {
            cstdio::out(b",");
        }
        cstdio::out(format!("{}", n.child_ids[i as usize]).as_bytes());
    }
    cstdio::out(b"]\n");
}

fn dump_tree(tree: &Tree, label: &str) {
    c_println!(
        "{}: size={} has_root={} root_id={}",
        label,
        tree.size(),
        tree.has_root,
        tree.root_id
    );
    tree.print();
}

fn dump_path(tree: &Tree, id: TreeId, max_length: i32, path: &mut [TreeId]) {
    for slot in path.iter_mut() {
        *slot = 0;
    }
    let len = tree.find_path(id, path, max_length);
    cstdio::out(format!("find_path(id={}, max={}) = {} path=[", id, max_length, len).as_bytes());
    let mut i = 0i32;
    while i < len {
        if i != 0 {
            cstdio::out(b",");
        }
        cstdio::out(format!("{}", path[i as usize]).as_bytes());
        i += 1;
    }
    cstdio::out(b"]\n");
}

fn dump_queries(tree: &Tree, id: TreeId) {
    c_println!(
        "id={} contains={} depth={} height={} descendants={}",
        id,
        tree.contains(id),
        tree.get_depth(id),
        tree.get_height(id),
        tree.count_descendants(id)
    );
}

/// Bytes that are not valid UTF-8, plus printf metacharacters.
const WEIRD_DATA: &[u8] = b"\xff\xfe\x80 caf\xc3\xa9 %s %d %%";

// ---------- scenarios ----------

fn sc_empty_print() {
    let tree = Tree::create();
    dump_tree(&tree, "fresh");
    dump_queries(&tree, 0);
    dump_queries(&tree, 1);
    tree.delete();
}

fn sc_null_data() {
    let mut tree = Tree::create();
    c_println!("add(1,0,NULL) = {}", tree.add_node(1, 0, None));
    c_println!("add(2,1,NULL) = {}", tree.add_node(2, 1, None));
    dump_tree(&tree, "null-data");
    dump_node(&tree, 1);
    dump_node(&tree, 2);
    tree.delete();
}

fn sc_parent_missing() {
    let mut tree = Tree::create();
    c_println!("add(1,0,root) = {}", tree.add_node(1, 0, Some(b"root")));
    c_println!("add(2,99,orphan) = {}", tree.add_node(2, 99, Some(b"orphan")));
    c_println!(
        "add(3,0,parent-zero) = {}",
        tree.add_node(3, 0, Some(b"parent-zero"))
    );
    dump_tree(&tree, "after-failures");
    tree.delete();
}

fn sc_remove_missing() {
    let mut tree = Tree::create();
    c_println!("remove(1) on empty = {}", tree.remove_node(1));
    c_println!("add(1,0,root) = {}", tree.add_node(1, 0, Some(b"root")));
    c_println!("remove(42) = {}", tree.remove_node(42));
    dump_tree(&tree, "after-failed-removes");
    tree.delete();
}

fn sc_queries_missing() {
    let mut tree = Tree::create();
    tree.add_node(1, 0, Some(b"root"));
    tree.add_node(2, 1, Some(b"child"));
    dump_queries(&tree, 999);
    let mut path = [0u64; 64];
    dump_path(&tree, 999, 64, &mut path);
    dump_queries(&tree, 1);
    dump_queries(&tree, 2);
    tree.delete();
}

fn sc_find_path_clamp() {
    let mut tree = Tree::create();
    for i in 1..=5u64 {
        tree.add_node(i, i - 1, Some(b"chain"));
    }
    let mut path = [0u64; 64];
    dump_path(&tree, 5, 64, &mut path);
    dump_path(&tree, 5, 5, &mut path);
    dump_path(&tree, 5, 3, &mut path);
    dump_path(&tree, 5, 1, &mut path);
    dump_path(&tree, 5, 0, &mut path);
    dump_path(&tree, 5, -1, &mut path);
    dump_path(&tree, 1, 0, &mut path);
    tree.delete();
}

fn sc_find_path_deep() {
    let mut tree = Tree::create();
    for i in 1..=1200u64 {
        tree.add_node(i, i - 1, Some(b"deep"));
    }
    c_println!("size={} depth(1200)={}", tree.size(), tree.get_depth(1200));
    let mut path = vec![0u64; 2000];
    let len = tree.find_path(1200, &mut path, 2000);
    c_println!(
        "len={} first={} second={} last={}",
        len,
        path[0],
        path[1],
        path[(len - 1) as usize]
    );
    tree.delete();
}

fn sc_data_trunc() {
    let mut tree = Tree::create();

    let a255 = vec![b'a'; 255];
    tree.add_node(1, 0, Some(&a255));

    let b254 = vec![b'b'; 254];
    tree.add_node(2, 1, Some(&b254));

    let c300 = vec![b'c'; 300];
    tree.add_node(3, 1, Some(&c300));

    tree.add_node(4, 1, Some(b""));

    tree.add_node(5, 1, Some(WEIRD_DATA));

    for i in 1..=5u64 {
        let n = tree.node(i).unwrap();
        c_println!("node {} strlen={}", i, n.data_cstr().len());
    }
    tree.print();
    tree.delete();
}

fn sc_hashmap_reuse() {
    let mut map = Hashmap::create();

    for k in 0..8u64 {
        map.put(k, Some(k as usize));
    }
    dump_map(&map, "after-8-puts");

    let r3 = if map.remove(3).is_some() { "set" } else { "null" };
    let r5 = if map.remove(5).is_some() { "set" } else { "null" };
    let r99 = if map.remove(99).is_some() { "set" } else { "null" };
    c_println!("remove(3)={} remove(5)={} remove(99)={}", r3, r5, r99);
    dump_map(&map, "after-removes");

    map.put(3, Some(3));
    dump_map(&map, "after-reinsert");

    map.put(3, Some(7));
    c_println!(
        "contains(3)={} contains(5)={} size={}",
        map.contains(3),
        map.contains(5),
        map.size()
    );
    dump_map(&map, "after-update");
}

fn sc_hashmap_null_value() {
    let mut map = Hashmap::create();
    c_println!("put(7,NULL)={}", map.put(7, None));
    c_println!(
        "size={} contains(7)={} get(7)={}",
        map.size(),
        map.contains(7),
        if map.get(7).is_some() { "set" } else { "null" }
    );
    dump_map(&map, "null-value");
    c_println!(
        "remove(7)={}",
        if map.remove(7).is_some() { "set" } else { "null" }
    );
    dump_map(&map, "after-remove");
}

fn sc_hashmap_clear() {
    let mut map = Hashmap::create();
    for k in 0..20u64 {
        map.put(k, Some(k as usize));
    }
    map.remove(4);
    dump_map(&map, "before-clear");
    map.clear();
    dump_map(&map, "after-clear");
    c_println!("contains(0)={} size={}", map.contains(0), map.size());
    map.put(0, Some(0));
    dump_map(&map, "after-clear-put");
}

fn sc_hashmap_resize() {
    let mut map = Hashmap::create();
    for k in 0..300u64 {
        map.put(k * 7 + 1, Some(k as usize));
        if k % 3 == 0 {
            map.remove(k * 7 + 1);
        }
    }
    dump_map(&map, "resized");
    let mut found = 0;
    for k in 0..300u64 {
        if map.contains(k * 7 + 1) != 0 {
            found += 1;
        }
    }
    c_println!(
        "found={} size={} capacity={} deleted={}",
        found,
        map.size,
        map.capacity,
        map.deleted_count
    );
}

fn sc_big_ids() {
    let mut tree = Tree::create();
    c_println!(
        "add(max,0)={}",
        tree.add_node(18446744073709551615, 0, Some(b"max"))
    );
    c_println!(
        "add(0,max)={}",
        tree.add_node(0, 18446744073709551615, Some(b"zero"))
    );
    c_println!(
        "add(9223372036854775808,max)={}",
        tree.add_node(9223372036854775808, 18446744073709551615, Some(b"high-bit"))
    );
    dump_tree(&tree, "big-ids");
    dump_node(&tree, 18446744073709551615);
    dump_queries(&tree, 18446744073709551615);
    dump_queries(&tree, 0);
    let mut path = [0u64; 64];
    dump_path(&tree, 9223372036854775808, 64, &mut path);
    c_println!(
        "dup(max)={}",
        tree.add_node(18446744073709551615, 0, Some(b"dup"))
    );
    c_println!(
        "orphan={}",
        tree.add_node(5, 12345678901234567890, Some(b"orphan"))
    );
    c_println!(
        "remove(missing-max)={}",
        tree.remove_node(18446744073709551614)
    );
    tree.delete();
}

fn sc_zero_root() {
    let mut tree = Tree::create();
    c_println!("add(0,0,root)={}", tree.add_node(0, 0, Some(b"root")));
    c_println!("add(1,0,child)={}", tree.add_node(1, 0, Some(b"child")));
    c_println!("add(2,1,grand)={}", tree.add_node(2, 1, Some(b"grand")));
    dump_tree(&tree, "zero-root");
    dump_queries(&tree, 0);
    dump_queries(&tree, 1);
    dump_queries(&tree, 2);
    let mut path = [0u64; 64];
    dump_path(&tree, 2, 64, &mut path);
    tree.delete();
}

fn sc_remove_root_readd() {
    let mut tree = Tree::create();
    tree.add_node(1, 0, Some(b"root"));
    tree.add_node(2, 1, Some(b"child"));
    tree.add_node(3, 2, Some(b"grand"));
    dump_tree(&tree, "before");
    c_println!("remove(1)={}", tree.remove_node(1));
    dump_tree(&tree, "after-remove-root");
    dump_map(&tree.node_map, "map-after-remove-root");
    c_println!("add(7,99,newroot)={}", tree.add_node(7, 99, Some(b"newroot")));
    c_println!(
        "add(8,7,newchild)={}",
        tree.add_node(8, 7, Some(b"newchild"))
    );
    c_println!("add(1,7,readded)={}", tree.add_node(1, 7, Some(b"readded")));
    dump_tree(&tree, "after-readd");
    dump_node(&tree, 7);
    dump_map(&tree.node_map, "map-after-readd");
    tree.delete();
}

fn sc_child_shift() {
    let mut tree = Tree::create();
    tree.add_node(1, 0, Some(b"root"));
    for i in 2..=6u64 {
        tree.add_node(i, 1, Some(b"child"));
    }
    dump_node(&tree, 1);
    c_println!("remove(2)={}", tree.remove_node(2));
    dump_node(&tree, 1);
    c_println!("remove(4)={}", tree.remove_node(4));
    dump_node(&tree, 1);
    c_println!("remove(6)={}", tree.remove_node(6));
    dump_node(&tree, 1);
    dump_tree(&tree, "after-shifts");
    c_println!("add(9,1)={}", tree.add_node(9, 1, Some(b"new")));
    dump_node(&tree, 1);
    dump_tree(&tree, "refilled");
    tree.delete();
}

fn sc_max_children() {
    let mut tree = Tree::create();
    tree.add_node(1, 0, Some(b"root"));
    for i in 0..MAX_CHILDREN as u64 {
        tree.add_node(i + 2, 1, Some(b"child"));
    }
    c_println!("overflow={}", tree.add_node(1000, 1, Some(b"overflow")));
    c_println!(
        "size={} height={} descendants={}",
        tree.size(),
        tree.get_height(1),
        tree.count_descendants(1)
    );
    dump_node(&tree, 1);
    c_println!("remove(2)={}", tree.remove_node(2));
    c_println!("refill={}", tree.add_node(1000, 1, Some(b"refill")));
    c_println!("overflow2={}", tree.add_node(1001, 1, Some(b"overflow2")));
    dump_node(&tree, 1);
    tree.delete();
}

fn sc_subtree_removal() {
    let mut tree = Tree::create();
    tree.add_node(1, 0, Some(b"root"));
    let mut next: u64 = 2;
    for _a in 0..3 {
        let branch = next;
        next += 1;
        tree.add_node(branch, 1, Some(b"branch"));
        for _b in 0..3 {
            let leaf = next;
            next += 1;
            tree.add_node(leaf, branch, Some(b"leaf"));
            tree.add_node(next, leaf, Some(b"twig"));
            next += 1;
        }
    }
    dump_tree(&tree, "built");
    c_println!(
        "descendants(1)={} height(1)={}",
        tree.count_descendants(1),
        tree.get_height(1)
    );
    c_println!("remove(2)={}", tree.remove_node(2));
    dump_tree(&tree, "after-remove-branch");
    dump_map(&tree.node_map, "map");
    for i in 1..=next {
        c_println!("contains({})={}", i, tree.contains(i));
    }
    tree.delete();
}

fn sc_dup_and_reinsert() {
    let mut tree = Tree::create();
    c_println!("add(1,0,a)={}", tree.add_node(1, 0, Some(b"a")));
    c_println!("add(1,0,b)={}", tree.add_node(1, 0, Some(b"b")));
    c_println!("add(1,1,c)={}", tree.add_node(1, 1, Some(b"c")));
    c_println!("add(2,1,d)={}", tree.add_node(2, 1, Some(b"d")));
    c_println!("add(2,2,e)={}", tree.add_node(2, 2, Some(b"e")));
    dump_tree(&tree, "dups");
    dump_node(&tree, 1);
    dump_node(&tree, 2);
    tree.delete();
}

fn sc_interleaved_output() {
    let mut tree = Tree::create();
    c_println!("stdout line 1");
    tree.add_node(1, 0, Some(b"root"));
    tree.add_node(1, 0, Some(b"dup"));
    c_println!("stdout line 2");
    tree.remove_node(77);
    c_println!("stdout line 3");
    cstdio::flush();
    tree.add_node(2, 88, Some(b"orphan"));
    c_println!("stdout line 4");
    tree.delete();
}

fn sc_deep_recursion() {
    let mut tree = Tree::create();
    // get_height, count_descendants and remove_subtree are all recursive;
    // the C driver only ever goes 5 deep.
    for i in 1..=5000u64 {
        tree.add_node(i, i - 1, Some(b"chain"));
    }
    c_println!(
        "size={} height={} descendants={} depth(5000)={}",
        tree.size(),
        tree.get_height(1),
        tree.count_descendants(1),
        tree.get_depth(5000)
    );
    c_println!(
        "height(2500)={} descendants(2500)={}",
        tree.get_height(2500),
        tree.count_descendants(2500)
    );
    c_println!("remove(2500)={}", tree.remove_node(2500));
    c_println!(
        "size={} contains(2499)={} contains(2500)={} contains(5000)={}",
        tree.size(),
        tree.contains(2499),
        tree.contains(2500),
        tree.contains(5000)
    );
    c_println!(
        "height={} descendants={}",
        tree.get_height(1),
        tree.count_descendants(1)
    );
    // Sequenced into locals to mirror probe.c; see the note there about C's
    // unspecified printf argument evaluation order.
    let rc = tree.remove_node(1);
    let sz = tree.size();
    let hr = tree.has_root;
    c_println!("remove(1)={} size={} has_root={}", rc, sz, hr);
    tree.delete();
}

fn main() {
    restore_default_sigpipe();

    let arg = std::env::args().nth(1);
    let s = match arg {
        Some(s) => s,
        None => {
            eprintln!("usage: probe <scenario>");
            cstdio::flush();
            std::process::exit(2);
        }
    };

    match s.as_str() {
        "empty_print" => sc_empty_print(),
        "null_data" => sc_null_data(),
        "parent_missing" => sc_parent_missing(),
        "remove_missing" => sc_remove_missing(),
        "queries_missing" => sc_queries_missing(),
        "find_path_clamp" => sc_find_path_clamp(),
        "find_path_deep" => sc_find_path_deep(),
        "data_trunc" => sc_data_trunc(),
        "hashmap_reuse" => sc_hashmap_reuse(),
        "hashmap_null_value" => sc_hashmap_null_value(),
        "hashmap_clear" => sc_hashmap_clear(),
        "hashmap_resize" => sc_hashmap_resize(),
        "big_ids" => sc_big_ids(),
        "zero_root" => sc_zero_root(),
        "remove_root_readd" => sc_remove_root_readd(),
        "child_shift" => sc_child_shift(),
        "max_children" => sc_max_children(),
        "subtree_removal" => sc_subtree_removal(),
        "dup_and_reinsert" => sc_dup_and_reinsert(),
        "interleaved" => sc_interleaved_output(),
        "deep_recursion" => sc_deep_recursion(),
        other => {
            eprintln!("unknown scenario: {}", other);
            cstdio::flush();
            std::process::exit(3);
        }
    }

    cstdio::flush();
    std::process::exit(0);
}
