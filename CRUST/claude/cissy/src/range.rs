pub mod range {
#[derive(Debug, Clone, PartialEq)]
pub enum RangeType {
Empty,
Single,
StartEnd,
GreaterEqual,
}
#[derive(Debug, Clone)]
pub struct RangeElement {
pub start: u32,
pub end: u32,
pub rangetype: RangeType,
pub next: Option<Box<RangeElement>>,
}

fn elem_min_start(e: &RangeElement) -> u32 { e.start }
fn elem_max_value(e: &RangeElement) -> u32 {
    match e.rangetype {
        RangeType::Single => e.start,
        RangeType::StartEnd => e.end,
        RangeType::GreaterEqual => u32::MAX,
        RangeType::Empty => 0,
    }
}

fn covers(new: &RangeElement, other: &RangeElement) -> bool {
    let omin = elem_min_start(other);
    let omax = elem_max_value(other);
    match new.rangetype {
        RangeType::Single => other.rangetype == RangeType::Single && other.start == new.start,
        RangeType::StartEnd => omin >= new.start && omax <= new.end,
        RangeType::GreaterEqual => omin >= new.start,
        RangeType::Empty => false,
    }
}

fn list_covers_all(new: &RangeElement, head: &Option<Box<RangeElement>>) -> bool {
    let mut cur = head.as_deref();
    let mut any = false;
    while let Some(node) = cur {
        any = true;
        if !covers(new, node) {
            return false;
        }
        cur = node.next.as_deref();
    }
    any
}

fn raw_append(list: Option<Box<RangeElement>>, new_node: Box<RangeElement>) -> Option<Box<RangeElement>> {
    match list {
        None => Some(new_node),
        Some(mut head) => {
            let mut ptr: &mut RangeElement = &mut head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(new_node);
            Some(head)
        }
    }
}

fn smart_add(list: Option<Box<RangeElement>>, new_node: Box<RangeElement>) -> Option<Box<RangeElement>> {
    if list.is_none() {
        return Some(new_node);
    }
    if list_covers_all(&new_node, &list) {
        return Some(new_node);
    }
    raw_append(list, new_node)
}

impl RangeElement {
pub fn new() -> Self {
    RangeElement {
        start: u32::MAX,
        end: u32::MAX,
        rangetype: RangeType::Empty,
        next: None,
    }
}
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    let mut node = Box::new(RangeElement::new());
    node.start = num;
    node.rangetype = RangeType::Single;
    smart_add(start_of_list, node)
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    let mut node = Box::new(RangeElement::new());
    node.start = start;
    if start == end {
        node.rangetype = RangeType::Single;
    } else {
        node.rangetype = RangeType::StartEnd;
        node.end = end;
    }
    smart_add(start_of_list, node)
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut node = Box::new(RangeElement::new());
    node.start = num;
    node.rangetype = RangeType::GreaterEqual;
    smart_add(start_of_list, node)
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut cur = start_of_list.as_deref();
    while let Some(node) = cur {
        match node.rangetype {
            RangeType::Single => {
                if node.start == num {
                    return true;
                }
            }
            RangeType::GreaterEqual => {
                if num >= node.start {
                    return true;
                }
            }
            RangeType::StartEnd => {
                if num >= node.start && num <= node.end {
                    return true;
                }
            }
            RangeType::Empty => {}
        }
        cur = node.next.as_deref();
    }
    false
}
pub fn to_string<'a>(&'a self, buf: &'a mut String, _bufsize: usize) -> &'a str {
    buf.clear();
    match self.rangetype {
        RangeType::Single => {
            buf.push_str(&format!("[{}]", self.start));
        }
        RangeType::GreaterEqual => {
            buf.push_str(&format!("[{}-]", self.start));
        }
        RangeType::StartEnd => {
            buf.push_str(&format!("[{}-{}]", self.start, self.end));
        }
        RangeType::Empty => {
            buf.push_str("[]");
        }
    }
    buf.as_str()
}
pub fn list_to_string<'a>(buf: &'a mut String, _bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    // Collect into a vec for easier indexing
    let mut nodes: Vec<&RangeElement> = Vec::new();
    let mut cur = start_of_list.as_deref();
    while let Some(node) = cur {
        nodes.push(node);
        cur = node.next.as_deref();
    }
    // Find index of last strict decrease in start values
    let mut start_idx = 0;
    for i in 1..nodes.len() {
        if nodes[i].start < nodes[i - 1].start {
            start_idx = i;
        }
    }
    for node in &nodes[start_idx..] {
        match node.rangetype {
            RangeType::Single => {
                buf.push_str(&format!("[{}]", node.start));
            }
            RangeType::GreaterEqual => {
                buf.push_str(&format!("[{}-]", node.start));
            }
            RangeType::StartEnd => {
                buf.push_str(&format!("[{}-{}]", node.start, node.end));
            }
            RangeType::Empty => {
                buf.push_str("[]");
            }
        }
    }
    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    // Build the list directly using raw append (no smart logic).
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

    fn make_single(num: u32) -> Box<RangeElement> {
        let mut n = Box::new(RangeElement::new());
        n.start = num;
        n.rangetype = RangeType::Single;
        n
    }
    fn make_start_end(s: u32, e: u32) -> Box<RangeElement> {
        let mut n = Box::new(RangeElement::new());
        n.start = s;
        if s == e {
            n.rangetype = RangeType::Single;
        } else {
            n.rangetype = RangeType::StartEnd;
            n.end = e;
        }
        n
    }
    fn make_greater_equal(num: u32) -> Box<RangeElement> {
        let mut n = Box::new(RangeElement::new());
        n.start = num;
        n.rangetype = RangeType::GreaterEqual;
        n
    }

    for ch in text.chars() {
        if ch.is_ascii_digit() {
            numbuf.push(ch);
        } else if ch == '-' {
            if (state & flag_dash) != 0 {
                eprintln!("error: too many dashes in range");
                std::process::exit(-1);
            }
            state |= flag_dash;
            if numbuf.is_empty() {
                eprintln!("error: range cannot start with '-'");
                std::process::exit(-1);
            }
            start = numbuf.parse::<u32>().unwrap_or(0);
            if start == 0 {
                eprintln!("error: start range <= 0");
                std::process::exit(-1);
            }
            numbuf.clear();
        } else if ch == ',' {
            if numbuf.is_empty() {
                if (state & flag_dash) != 0 {
                    rv = raw_append(rv, make_greater_equal(start));
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
                }
                rv = raw_append(rv, make_start_end(start, end));
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = raw_append(rv, make_single(start));
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }

    if numbuf.is_empty() {
        if (state & flag_dash) != 0 {
            rv = raw_append(rv, make_greater_equal(start));
        }
    } else if start > 0 {
        end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
        }
        rv = raw_append(rv, make_start_end(start, end));
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        rv = raw_append(rv, make_single(start));
    }
    rv
}
}
}
