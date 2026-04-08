use fslib::symt::SymTable;

#[test]
fn test_add_get() {
    let mut st = SymTable::new();
    st.add(0, "eps");
    st.add(1, "hello");
    st.add(2, "world");
    assert_eq!(st.n_items, 3);
    assert_eq!(st.get(0), Some("eps"));
    assert_eq!(st.get(1), Some("hello"));
    assert_eq!(st.get(2), Some("world"));
}

#[test]
fn test_reverse_lookup() {
    let mut st = SymTable::new();
    st.add(0, "eps");
    st.add(1, "hello");
    st.add(2, "world");
    assert_eq!(st.getr("hello"), Some(1));
    assert_eq!(st.getr("world"), Some(2));
    assert_eq!(st.getr("eps"), Some(0));
}

#[test]
fn test_get_out_of_range() {
    let st = SymTable::new();
    assert_eq!(st.get(9999), None);
}

#[test]
fn test_fnv32() {
    use fslib::symt::fnv32;
    // Just verify it returns consistent values
    let h1 = fnv32("hello");
    let h2 = fnv32("hello");
    assert_eq!(h1, h2);
    // Different strings should (very likely) produce different hashes
    let h3 = fnv32("world");
    assert_ne!(h1, h3);
}

fn main() {}
