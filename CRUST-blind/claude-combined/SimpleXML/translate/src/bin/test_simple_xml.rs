use SimpleXML::simple_xml::{parse_xml_from_text, XMLElement, XMLParser};

#[test]
fn test_xmlelement_new() {
    let e = XMLElement::new("name".to_string(), "Kien".to_string());
    assert_eq!(e.tag_name, "name");
    assert_eq!(e.value, "Kien");
    assert_eq!(e.children.size(), 0);
}

#[test]
fn test_xmlparser_new() {
    let _p = XMLParser::new();
}

#[test]
fn test_parse_via_method() {
    let mut p = XMLParser::new();
    let elem = p.parse("<name>Kien</name>").unwrap();
    assert_eq!(elem.tag_name, "name");
    assert_eq!(elem.value, "Kien");
    assert_eq!(elem.children.size(), 0);
}

#[test]
fn test_simple_parse_single_tag() {
    let s = "<name>Kien</name>";
    let elem = parse_xml_from_text(s).unwrap();
    assert_eq!(elem.tag_name, "name");
    assert_eq!(elem.value, "Kien");
    assert_eq!(elem.children.size(), 0);
}

#[test]
fn test_parse_programmer_xml() {
    // Mirror C test_xml exactly
    let s = "<programmer>    <name>Kien Nguyen Trung</name>    <languages>    <language>C</language>    <language>C++</language>    <language>Python</language>    <language>Ruby</language>    <language>Objective C</language>    <language>Java</language>    <language>Javascript</language>    <language>Lua</language>    <language>C#</language>    <language>PHP</language>    </languages>     </programmer>";
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

    let langs = ["C", "C++", "Python", "Ruby", "Objective C", "Java", "Javascript", "Lua", "C#", "PHP"];
    for i in 0..10 {
        let child = child2.children.get_element_at(i).unwrap();
        assert_eq!(child.tag_name, "language");
        assert_eq!(child.value, langs[i]);
    }
}

#[test]
fn test_parse_nested() {
    let s = "<a><b>x</b></a>";
    let elem = parse_xml_from_text(s).unwrap();
    assert_eq!(elem.tag_name, "a");
    assert_eq!(elem.children.size(), 1);
    let b = elem.children.get_element_at(0).unwrap();
    assert_eq!(b.tag_name, "b");
    assert_eq!(b.value, "x");
    assert_eq!(b.children.size(), 0);
}

#[test]
fn test_parse_two_children() {
    let s = "<root><a>A</a><b>B</b></root>";
    let elem = parse_xml_from_text(s).unwrap();
    assert_eq!(elem.tag_name, "root");
    assert_eq!(elem.children.size(), 2);
    let c0 = elem.children.get_element_at(0).unwrap();
    let c1 = elem.children.get_element_at(1).unwrap();
    assert_eq!(c0.tag_name, "a");
    assert_eq!(c0.value, "A");
    assert_eq!(c1.tag_name, "b");
    assert_eq!(c1.value, "B");
}

fn main() {}
