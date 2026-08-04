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
impl RangeElement {
pub fn new() -> Self {
    Self {
        start: u32::MAX,
        end: u32::MAX,
        rangetype: RangeType::Empty,
        next: None,
    }
}
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
assert!(num > 0);
if contains_exact_single(&start_of_list, num) {
    return start_of_list;
}
if let Some(head) = start_of_list.as_deref() {
    if num < head.start {
        return Some(Box::new(RangeElement {
            start: num,
            end: 1,
            rangetype: RangeType::Single,
            next: None,
        }));
    }
}
let mut list = start_of_list;
let new_node = Box::new(RangeElement {
    start: num,
    end: 0,
    rangetype: RangeType::Single,
    next: None,
});
append_node(&mut list, new_node);
list
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
assert!(start > 0);
assert!(end >= start);
if contains_exact_range(&start_of_list, start, end) {
    return start_of_list;
}
if let Some(head) = start_of_list.as_deref() {
    if head.next.is_none() && matches!(head.rangetype, RangeType::Single) && head.start == start {
        if head.end != 1 {
            return Some(Box::new(RangeElement {
                start,
                end: if start == end { 0 } else { end },
                rangetype: if start == end {
                    RangeType::Single
                } else {
                    RangeType::StartEnd
                },
                next: None,
            }));
        }
    }
}
let mut list = start_of_list;
let new_node = Box::new(RangeElement {
    start,
    end: if start == end { 0 } else { end },
    rangetype: if start == end {
        RangeType::Single
    } else {
        RangeType::StartEnd
    },
    next: None,
});
append_node(&mut list, new_node);
list
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
let mut list = start_of_list;
let new_node = Box::new(RangeElement {
    start: num,
    end: u32::MAX,
    rangetype: RangeType::GreaterEqual,
    next: None,
});
append_node(&mut list, new_node);
list
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
let mut current = start_of_list.as_deref();
while let Some(element) = current {
    let contains = match element.rangetype {
        RangeType::Empty => false,
        RangeType::Single => element.start == num,
        RangeType::GreaterEqual => num >= element.start,
        RangeType::StartEnd => num >= element.start && num <= element.end,
    };
    if contains {
        return true;
    }
    current = element.next.as_deref();
}
false
}
pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
buf.clear();
let rendered = match self.rangetype {
    RangeType::Single => format!("[{}]", self.start),
    RangeType::GreaterEqual => format!("[{}-]", self.start),
    RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
    RangeType::Empty => "[]".to_string(),
};
push_truncated(buf, &rendered, bufsize);
Box::leak(buf.clone().into_boxed_str())
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
buf.clear();
let mut current = start_of_list.as_deref();
while let Some(element) = current {
    let mut tmp = String::new();
    element.to_string(&mut tmp, usize::MAX);
    push_truncated(buf, &tmp, bufsize);
    current = element.next.as_deref();
}
buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
let flag_dash = 0x01;
let mut state = 0;
let mut numbuf = String::new();
let mut start = 0_u32;
let mut rv = None;

for ch in text.chars() {
    if ch.is_ascii_digit() {
        numbuf.push(ch);
    } else if ch == '-' {
        assert_eq!(state & flag_dash, 0, "error: too many dashes in range");
        state |= flag_dash;
        assert!(!numbuf.is_empty(), "error: range cannot start with '-'");
        start = parse_num(&numbuf);
        assert!(start > 0, "error: start range <= 0");
        numbuf.clear();
    } else if ch == ',' {
        if numbuf.is_empty() {
            if state & flag_dash != 0 {
                rv = RangeElement::add_greater_equal(start, rv);
            }
        } else if start > 0 {
            let end = parse_num(&numbuf);
            assert!(end > 0 && start <= end, "error: illegal start/end ranges");
            rv = RangeElement::add_start_end(start, end, rv);
        } else {
            start = parse_num(&numbuf);
            rv = add_parsed_single(start, rv);
        }
        numbuf.clear();
        start = 0;
        state = 0;
    }
}

if numbuf.is_empty() {
    if state & flag_dash != 0 {
        rv = RangeElement::add_greater_equal(start, rv);
    }
} else if start > 0 {
    let end = parse_num(&numbuf);
    assert!(end > 0 && start <= end, "error: illegal start/end range");
    rv = RangeElement::add_start_end(start, end, rv);
} else {
    start = parse_num(&numbuf);
    rv = add_parsed_single(start, rv);
}

rv
}
}

fn append_node(list: &mut Option<Box<RangeElement>>, new_node: Box<RangeElement>) {
    match list {
        None => *list = Some(new_node),
        Some(head) => {
            let mut tail = head.as_mut();
            while let Some(ref mut next) = tail.next {
                tail = next.as_mut();
            }
            tail.next = Some(new_node);
        }
    }
}

fn contains_exact_single(list: &Option<Box<RangeElement>>, num: u32) -> bool {
    let mut current = list.as_deref();
    while let Some(element) = current {
        if matches!(element.rangetype, RangeType::Single) && element.start == num {
            return true;
        }
        current = element.next.as_deref();
    }
    false
}

fn contains_exact_range(list: &Option<Box<RangeElement>>, start: u32, end: u32) -> bool {
    let mut current = list.as_deref();
    while let Some(element) = current {
        let matches = match element.rangetype {
            RangeType::StartEnd => element.start == start && element.end == end,
            RangeType::Single => start == end && element.start == start,
            _ => false,
        };
        if matches {
            return true;
        }
        current = element.next.as_deref();
    }
    false
}

fn add_parsed_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    if contains_exact_single(&start_of_list, num) {
        return start_of_list;
    }
    let mut list = start_of_list;
    let new_node = Box::new(RangeElement {
        start: num,
        end: 1,
        rangetype: RangeType::Single,
        next: None,
    });
    append_node(&mut list, new_node);
    list
}

fn parse_num(text: &str) -> u32 {
    text.parse::<u32>().expect("error: invalid integer")
}

fn push_truncated(buf: &mut String, text: &str, bufsize: usize) {
    if bufsize == 0 {
        return;
    }
    let remaining = bufsize.saturating_sub(buf.len());
    if remaining == 0 {
        return;
    }
    let take = remaining.min(text.len());
    buf.push_str(&text[..take]);
}
}
