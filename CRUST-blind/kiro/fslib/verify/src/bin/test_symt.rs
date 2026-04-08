use fslib::symt::SymTable;

#[test]
fn test_add_and_get() {
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
fn test_getr() {
    let mut st = SymTable::new();
    st.add(0, "eps");
    st.add(1, "hello");
    st.add(2, "world");
    assert_eq!(st.getr("hello"), Some(1));
    assert_eq!(st.getr("world"), Some(2));
    assert_eq!(st.getr("eps"), Some(0));
}

#[test]
fn test_getr_missing() {
    let st = SymTable::new();
    assert_eq!(st.getr("nonexistent"), None);
}

#[test]
fn test_get_out_of_range() {
    let st = SymTable::new();
    assert_eq!(st.get(9999), None);
}

#[test]
fn test_add_large_id() {
    let mut st = SymTable::new();
    st.add(2000, "big");
    assert_eq!(st.get(2000), Some("big"));
    assert_eq!(st.getr("big"), Some(2000));
}

#[test]
fn test_compile() {
    let mut st = SymTable::new();
    st.compile("one\t1\ntwo\t2");
    assert_eq!(st.get(1), Some("one"));
    assert_eq!(st.get(2), Some("two"));
    assert_eq!(st.getr("one"), Some(1));
    assert_eq!(st.getr("two"), Some(2));
}

fn main() {}
