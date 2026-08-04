use fslib::symt::{fnv32, SymTable};

#[test]
fn test_fnv32_hash_values() {
    // Computed via C with FNV_OFFSET_32=2166136261, FNV_PRIME_32=16777619
    assert_eq!(fnv32("hello"), 1335831723);
    assert_eq!(fnv32("world"), 933488787);
    assert_eq!(fnv32(""), 2166136261);
    assert_eq!(fnv32("a"), 3826002220);
    assert_eq!(fnv32("ab"), 1294271946);
    assert_eq!(fnv32("abc"), 440920331);
    assert_eq!(fnv32("foo bar"), 1170285226);
    assert_eq!(fnv32("<start>"), 2930691589);
    assert_eq!(fnv32("test_token"), 3776810683);
}

#[test]
fn test_symt_new() {
    let s = SymTable::new();
    assert_eq!(s.n_items, 0);
    assert_eq!(s.n_max, 1024);
    assert_eq!(s.sym.len(), 1024);
}

#[test]
fn test_symt_add_get() {
    let mut s = SymTable::new();
    s.add(0, "<eps>");
    s.add(1, "hello");
    s.add(2, "world");
    assert_eq!(s.get(0), Some("<eps>"));
    assert_eq!(s.get(1), Some("hello"));
    assert_eq!(s.get(2), Some("world"));
}

#[test]
fn test_symt_get_unknown_returns_none() {
    let mut s = SymTable::new();
    s.add(0, "<eps>");
    s.add(1, "hello");
    s.add(2, "world");
    // C returns NULL for id > n_items; in C's case n_items=3 (after 3 adds),
    // and for id=100, sym[100] is NULL anyway, so returns NULL.
    assert_eq!(s.get(100), None);
}

#[test]
fn test_symt_getr() {
    let mut s = SymTable::new();
    s.add(0, "<eps>");
    s.add(1, "hello");
    s.add(2, "world");
    assert_eq!(s.getr("<eps>"), Some(0));
    assert_eq!(s.getr("hello"), Some(1));
    assert_eq!(s.getr("world"), Some(2));
    assert_eq!(s.getr("notfound"), None);
}

#[test]
fn test_symt_n_items_after_adds() {
    let mut s = SymTable::new();
    s.add(0, "a");
    s.add(1, "b");
    s.add(2, "c");
    assert_eq!(s.n_items, 3);
}

#[test]
fn test_symt_resize_for_large_id() {
    // C resizes when id >= n_max (initial 1024)
    let mut s = SymTable::new();
    s.add(2000, "big");
    assert!(s.n_max >= 2001);
    assert_eq!(s.get(2000), Some("big"));
}

#[test]
fn test_symt_compile() {
    let mut s = SymTable::new();
    s.compile("a\t1\nb\t2\nc\t3\n");
    assert_eq!(s.get(1), Some("a"));
    assert_eq!(s.get(2), Some("b"));
    assert_eq!(s.get(3), Some("c"));
}

#[test]
fn test_symt_read_from_string() {
    let input = "alpha\t1\nbeta\t2\ngamma\t3\n";
    let mut s = SymTable::new();
    let mut br = std::io::Cursor::new(input.as_bytes());
    s.read(&mut br).unwrap();
    assert_eq!(s.get(1), Some("alpha"));
    assert_eq!(s.get(2), Some("beta"));
    assert_eq!(s.get(3), Some("gamma"));
}

#[test]
fn test_symt_negative_id_returns_none() {
    let s = SymTable::new();
    assert_eq!(s.get(-1), None);
    assert_eq!(s.get(-100), None);
}

fn main() {}
