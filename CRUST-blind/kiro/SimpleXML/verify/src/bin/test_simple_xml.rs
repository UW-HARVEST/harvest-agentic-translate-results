use SimpleXML::simple_xml::{parse_xml_from_text, XMLElement};

#[test]
fn test_simple_element() {
    // C: tag=root, value=hello, children=0
    let elem = parse_xml_from_text("<root>hello</root>").unwrap();
    assert_eq!(elem.tag_name, "root");
    assert_eq!(elem.value, "hello");
    assert_eq!(elem.children.size(), 0);
}

#[test]
fn test_nested_single_child() {
    // C: a has 1 child, child b has tag=b, value=text, children=0
    let elem = parse_xml_from_text("<a><b>text</b></a>").unwrap();
    assert_eq!(elem.tag_name, "a");
    assert_eq!(elem.children.size(), 1);
    let child = elem.children.get_element_at(0).unwrap();
    assert_eq!(child.tag_name, "b");
    assert_eq!(child.value, "text");
    assert_eq!(child.children.size(), 0);
}

#[test]
fn test_multiple_children() {
    // C: root has 3 children: a/1, b/2, c/3
    let elem = parse_xml_from_text("<root><a>1</a><b>2</b><c>3</c></root>").unwrap();
    assert_eq!(elem.tag_name, "root");
    assert_eq!(elem.children.size(), 3);

    let c0 = elem.children.get_element_at(0).unwrap();
    assert_eq!(c0.tag_name, "a");
    assert_eq!(c0.value, "1");

    let c1 = elem.children.get_element_at(1).unwrap();
    assert_eq!(c1.tag_name, "b");
    assert_eq!(c1.value, "2");

    let c2 = elem.children.get_element_at(2).unwrap();
    assert_eq!(c2.tag_name, "c");
    assert_eq!(c2.value, "3");
}

#[test]
fn test_deep_nesting() {
    // C: a→b→c, c has value=deep
    let elem = parse_xml_from_text("<a><b><c>deep</c></b></a>").unwrap();
    assert_eq!(elem.tag_name, "a");
    assert_eq!(elem.children.size(), 1);

    let b = elem.children.get_element_at(0).unwrap();
    assert_eq!(b.tag_name, "b");
    assert_eq!(b.children.size(), 1);

    let c = b.children.get_element_at(0).unwrap();
    assert_eq!(c.tag_name, "c");
    assert_eq!(c.value, "deep");
    assert_eq!(c.children.size(), 0);
}

#[test]
fn test_full_xml_from_c_test() {
    // Exact test from C test.c test_xml()
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
    for (i, lang) in languages.iter().enumerate() {
        let child = child2.children.get_element_at(i).unwrap();
        assert_eq!(child.tag_name, "language");
        assert_eq!(child.value, *lang);
    }
}

#[test]
fn test_xml_error_on_invalid() {
    let result = parse_xml_from_text("not xml at all");
    assert!(result.is_err());
}

#[test]
fn test_xml_element_new() {
    let elem = XMLElement::new("test".to_string(), "val".to_string());
    assert_eq!(elem.tag_name, "test");
    assert_eq!(elem.value, "val");
    assert_eq!(elem.children.size(), 0);
    assert_eq!(elem.children.capacity, 8);
}

fn main() {}
