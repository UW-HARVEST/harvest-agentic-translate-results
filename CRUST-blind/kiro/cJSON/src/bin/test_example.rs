use cJSON::cjson::{parse, CJson};

#[test]
fn test_example_build_nested_objects() {
    let mut root = CJson::create_object();
    let mut node1 = CJson::create_object();
    let mut node2 = CJson::create_object();
    let mut node3 = CJson::create_object();

    node1.add_item_to_object("node1_key1", CJson::create_bool(false)).unwrap();
    node1.add_item_to_object("node1_key2", CJson::create_bool(true)).unwrap();

    node2.add_item_to_object("node2_key1", CJson::create_string("node2_value1")).unwrap();
    node2.add_item_to_object("node2_key2", CJson::create_string("node2_value2")).unwrap();

    node3.add_item_to_object("node3_key1", CJson::create_number(1000.0)).unwrap();
    node3.add_item_to_object("node3_key2", CJson::create_number(2000.0)).unwrap();

    node1.add_item_to_object("node1_node3", node3).unwrap();

    root.add_item_to_object("root_node1", node1).unwrap();
    root.add_item_to_object("root_node2", node2).unwrap();

    // Verify structure
    if let Some(CJson::Object(n1)) = root.get_object_item("root_node1") {
        assert_eq!(n1.get("node1_key1"), Some(&CJson::Bool(false)));
        assert_eq!(n1.get("node1_key2"), Some(&CJson::Bool(true)));
        if let Some(CJson::Object(n3)) = n1.get("node1_node3") {
            assert_eq!(n3.get("node3_key1"), Some(&CJson::Number(1000.0)));
            assert_eq!(n3.get("node3_key2"), Some(&CJson::Number(2000.0)));
        } else {
            panic!("expected node3 object");
        }
    } else {
        panic!("expected node1 object");
    }

    if let Some(CJson::Object(n2)) = root.get_object_item("root_node2") {
        assert_eq!(n2.get("node2_key1"), Some(&CJson::String("node2_value1".into())));
        assert_eq!(n2.get("node2_key2"), Some(&CJson::String("node2_value2".into())));
    } else {
        panic!("expected node2 object");
    }
}

#[test]
fn test_example_roundtrip() {
    // Build an object, print it, parse it back, verify
    let mut obj = CJson::create_object();
    obj.add_item_to_object("key", CJson::create_string("value")).unwrap();
    obj.add_item_to_object("num", CJson::create_number(42.0)).unwrap();

    let printed = obj.print_unformatted();
    let reparsed = parse(&printed, false).unwrap();

    // Both should have same keys/values
    assert_eq!(reparsed.get_object_item("key"), Some(&CJson::String("value".into())));
    assert_eq!(reparsed.get_object_item("num"), Some(&CJson::Number(42.0)));
}

#[test]
fn test_example_parse_and_access() {
    let input = r#"{"root_node1":{"node1_key1":false,"node1_key2":true,"node1_node3":{"node3_key1":1000,"node3_key2":2000}},"root_node2":{"node2_key1":"node2_value1","node2_key2":"node2_value2"}}"#;
    let j = parse(input, false).unwrap();

    let n1 = j.get_object_item("root_node1").unwrap();
    if let CJson::Object(map) = n1 {
        assert_eq!(map.get("node1_key1"), Some(&CJson::Bool(false)));
        assert_eq!(map.get("node1_key2"), Some(&CJson::Bool(true)));
    } else {
        panic!("expected object");
    }
}

fn main() {}
