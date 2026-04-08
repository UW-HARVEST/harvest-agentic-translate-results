use SimpleXML::simple_xml::{parse_xml_from_text, XMLElement};

#[test]
fn test_simple_element() {
    let e = parse_xml_from_text("<root>hello</root>").unwrap();
    assert_eq!(e.tag_name, "root");
    assert_eq!(e.value, "hello");
    assert_eq!(e.children.size(), 0);
}

#[test]
fn test_trimming() {
    let e = parse_xml_from_text("<root>  hello  </root>").unwrap();
    assert_eq!(e.value, "hello");
}

#[test]
fn test_nested_single_child() {
    let e = parse_xml_from_text("<a><b>inner</b></a>").unwrap();
    assert_eq!(e.tag_name, "a");
    assert_eq!(e.children.size(), 1);
    // Parent value is empty string in Rust (C had UB here)
    assert_eq!(e.value, "");
    let child = e.children.get_element_at(0).unwrap();
    assert_eq!(child.tag_name, "b");
    assert_eq!(child.value, "inner");
    assert_eq!(child.children.size(), 0);
}

#[test]
fn test_multiple_children() {
    let e = parse_xml_from_text("<root><a>1</a><b>2</b><c>3</c></root>").unwrap();
    assert_eq!(e.tag_name, "root");
    assert_eq!(e.children.size(), 3);

    let a = e.children.get_element_at(0).unwrap();
    assert_eq!(a.tag_name, "a");
    assert_eq!(a.value, "1");

    let b = e.children.get_element_at(1).unwrap();
    assert_eq!(b.tag_name, "b");
    assert_eq!(b.value, "2");

    let c = e.children.get_element_at(2).unwrap();
    assert_eq!(c.tag_name, "c");
    assert_eq!(c.value, "3");
}

#[test]
fn test_deep_nesting() {
    let e = parse_xml_from_text("<a><b><c>deep</c></b></a>").unwrap();
    assert_eq!(e.tag_name, "a");
    assert_eq!(e.children.size(), 1);

    let b = e.children.get_element_at(0).unwrap();
    assert_eq!(b.tag_name, "b");
    assert_eq!(b.children.size(), 1);

    let c = b.children.get_element_at(0).unwrap();
    assert_eq!(c.tag_name, "c");
    assert_eq!(c.value, "deep");
    assert_eq!(c.children.size(), 0);
}

#[test]
fn test_full_c_test_xml() {
    // Replicate the C test_xml() test case
    let s = "<programmer>\
    <name>Kien Nguyen Trung</name>\
    <languages>\
    <language>C</language>\
    <language>C++</language>\
    <language>Python</language>\
    <language>Ruby</language>\
    <language>Objective C</language>\
    <language>Java</language>\
    <language>Javascript</language>\
    <language>Lua</language>\
    <language>C#</language>\
    <language>PHP</language>\
    </languages> \
    </programmer>";

    let elem = parse_xml_from_text(s).unwrap();
    assert_eq!(elem.tag_name, "programmer");
    assert_eq!(elem.children.size(), 2);

    let child1 = elem.children.get_element_at(0).unwrap();
    assert_eq!(child1.tag_name, "name");
    assert_eq!(child1.children.size(), 0);
    assert_eq!(child1.value, "Kien Nguyen Trung");

    let child2 = elem.children.get_element_at(1).unwrap();
    assert_eq!(child2.tag_name, "languages");
    assert_eq!(child2.children.size(), 10);

    let languages = [
        "C", "C++", "Python", "Ruby", "Objective C",
        "Java", "Javascript", "Lua", "C#", "PHP",
    ];
    for (i, expected) in languages.iter().enumerate() {
        let child = child2.children.get_element_at(i).unwrap();
        assert_eq!(child.tag_name, "language");
        assert_eq!(child.value, *expected);
    }
}

#[test]
fn test_xml_element_new() {
    let e = XMLElement::new("tag".to_string(), "val".to_string());
    assert_eq!(e.tag_name, "tag");
    assert_eq!(e.value, "val");
    assert_eq!(e.children.size(), 0);
    assert_eq!(e.parent, ());
}

#[test]
fn test_parse_error() {
    // Invalid XML: text without tags
    let result = parse_xml_from_text("just text");
    assert!(result.is_err());
}

fn main() {}
