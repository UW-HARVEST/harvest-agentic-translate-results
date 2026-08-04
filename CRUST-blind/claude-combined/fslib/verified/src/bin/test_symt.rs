use fslib::symt::SymTable;

#[test]
fn test_add_get() {
    let mut st = SymTable::new();
    st.add(1, "one");
    st.add(2, "two");
    st.add(3, "three");
    assert_eq!(st.get(1), Some("one"));
    assert_eq!(st.get(2), Some("two"));
    assert_eq!(st.get(3), Some("three"));
    assert_eq!(st.n_items, 3);
}

#[test]
fn test_getr() {
    let mut st = SymTable::new();
    st.add(1, "one");
    st.add(2, "two");
    st.build_reverse();
    assert_eq!(st.getr("one"), Some(1));
    assert_eq!(st.getr("two"), Some(2));
    assert_eq!(st.getr("nonexistent"), None);
}

#[test]
fn test_fnv32() {
    use fslib::symt::fnv32;
    // FNV1a hash test (just check stability and inequality)
    let h1 = fnv32("hello");
    let h2 = fnv32("hello");
    let h3 = fnv32("world");
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

fn main() {}
