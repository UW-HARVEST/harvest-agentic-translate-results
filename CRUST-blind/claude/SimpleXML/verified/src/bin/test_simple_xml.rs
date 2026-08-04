use SimpleXML::simple_xml::{parse_xml_from_text, XMLElement, XMLParser, XMLTokenType, ParseState};

#[test]
fn test_xml_element_new() {
    let e = XMLElement::new("foo".to_string(), "bar".to_string());
    assert_eq!(e.tag_name, "foo");
    assert_eq!(e.value, "bar");
    assert_eq!(e.children.size(), 0);
}

#[test]
fn test_simple_single_element() {
    // <name>Kien Nguyen Trung</name>
    let xml = "<name>Kien Nguyen Trung</name>";
    let root = parse_xml_from_text(xml).expect("parse failed");
    assert_eq!(root.tag_name, "name");
    assert_eq!(root.value, "Kien Nguyen Trung");
    assert_eq!(root.children.size(), 0);
}

#[test]
fn test_xml_programmer_full() {
    // Mirror C test_xml(); whitespace treatment: spaces get trimmed,
    // and the original C parser accepts the long line-broken concat below.
    let s = "<programmer>    <name>Kien Nguyen Trung</name>    \
             <languages>    <language>C</language>    \
             <language>C++</language>    \
             <language>Python</language>    \
             <language>Ruby</language>    \
             <language>Objective C</language>    \
             <language>Java</language>    \
             <language>Javascript</language>    \
             <language>Lua</language>    \
             <language>C#</language>    \
             <language>PHP</language>    </languages>     </programmer>";

    let elem = parse_xml_from_text(s).expect("parse failed");
    assert_eq!(elem.tag_name, "programmer");
    assert_eq!(elem.children.size(), 2);

    let child1 = elem.children.get_element_at(0).unwrap();
    assert_eq!(child1.tag_name, "name");
    assert_eq!(child1.value, "Kien Nguyen Trung");
    assert_eq!(child1.children.size(), 0);

    let child2 = elem.children.get_element_at(1).unwrap();
    assert_eq!(child2.tag_name, "languages");
    assert_eq!(child2.children.size(), 10);

    let langs = [
        "C", "C++", "Python", "Ruby", "Objective C", "Java",
        "Javascript", "Lua", "C#", "PHP",
    ];
    for (i, expected) in langs.iter().enumerate() {
        let child = child2.children.get_element_at(i).unwrap();
        assert_eq!(child.tag_name, "language");
        assert_eq!(child.value, *expected);
    }
}

#[test]
fn test_token_types_distinct() {
    // basic enum equality sanity
    assert_eq!(XMLTokenType::Text, XMLTokenType::Text);
    assert_ne!(XMLTokenType::BeginOpenTag, XMLTokenType::BeginCloseTag);
    assert_ne!(XMLTokenType::EndTag, XMLTokenType::Text);
}

#[test]
fn test_parse_state_distinct() {
    assert_eq!(ParseState::State1, ParseState::State1);
    assert_ne!(ParseState::State1, ParseState::State2);
    assert_ne!(ParseState::State8, ParseState::StateError);
}

#[test]
fn test_parser_new() {
    let _p = XMLParser::new();
    // Just ensure it can be created. Internal fields aren't exposed, but
    // the parser should be usable.
}

#[test]
fn test_parser_reuse_via_helper() {
    // simple element
    let r1 = parse_xml_from_text("<a>1</a>").unwrap();
    assert_eq!(r1.tag_name, "a");
    assert_eq!(r1.value, "1");

    // a different element
    let r2 = parse_xml_from_text("<b>two</b>").unwrap();
    assert_eq!(r2.tag_name, "b");
    assert_eq!(r2.value, "two");
}

#[test]
fn test_nested_two_level() {
    // <root><child>val</child></root>
    let xml = "<root><child>val</child></root>";
    let r = parse_xml_from_text(xml).expect("parse failed");
    assert_eq!(r.tag_name, "root");
    assert_eq!(r.children.size(), 1);
    let c0 = r.children.get_element_at(0).unwrap();
    assert_eq!(c0.tag_name, "child");
    assert_eq!(c0.value, "val");
    assert_eq!(c0.children.size(), 0);
}

#[test]
fn test_multiple_siblings() {
    let xml = "<root><a>1</a><b>2</b><c>3</c></root>";
    let r = parse_xml_from_text(xml).expect("parse failed");
    assert_eq!(r.tag_name, "root");
    assert_eq!(r.children.size(), 3);

    let c0 = r.children.get_element_at(0).unwrap();
    assert_eq!(c0.tag_name, "a");
    assert_eq!(c0.value, "1");

    let c1 = r.children.get_element_at(1).unwrap();
    assert_eq!(c1.tag_name, "b");
    assert_eq!(c1.value, "2");

    let c2 = r.children.get_element_at(2).unwrap();
    assert_eq!(c2.tag_name, "c");
    assert_eq!(c2.value, "3");
}

#[test]
fn test_deep_nesting() {
    let xml = "<a><b><c>deep</c></b></a>";
    let r = parse_xml_from_text(xml).expect("parse failed");
    assert_eq!(r.tag_name, "a");
    assert_eq!(r.children.size(), 1);

    let b = r.children.get_element_at(0).unwrap();
    assert_eq!(b.tag_name, "b");
    assert_eq!(b.children.size(), 1);

    let c = b.children.get_element_at(0).unwrap();
    assert_eq!(c.tag_name, "c");
    assert_eq!(c.value, "deep");
    assert_eq!(c.children.size(), 0);
}

fn main() {}
