pub mod range {
use std::fmt::Write;

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

fn list_len(list: &Option<Box<RangeElement>>) -> usize {
    let mut n = 0;
    let mut cur = list;
    while let Some(node) = cur {
        n += 1;
        cur = &node.next;
    }
    n
}

fn last_node(list: &Option<Box<RangeElement>>) -> Option<&RangeElement> {
    let mut cur = list;
    let mut last: Option<&RangeElement> = None;
    while let Some(node) = cur {
        last = Some(node);
        cur = &node.next;
    }
    last
}

fn append_at_end(list: Option<Box<RangeElement>>, new_elem: Box<RangeElement>) -> Option<Box<RangeElement>> {
    match list {
        None => Some(new_elem),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = &mut *head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_mut().unwrap();
                }
                ptr.next = Some(new_elem);
            }
            Some(head)
        }
    }
}

fn element_str(e: &RangeElement) -> String {
    match e.rangetype {
        RangeType::Single => format!("[{}]", e.start),
        RangeType::GreaterEqual => format!("[{}-]", e.start),
        RangeType::StartEnd => format!("[{}-{}]", e.start, e.end),
        RangeType::Empty => "[]".to_string(),
    }
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
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::Single;
    // Smart-add: skip if duplicate of last element
    if let Some(last) = last_node(&start_of_list) {
        if last.rangetype == RangeType::Single && last.start == num {
            return start_of_list;
        }
    }
    append_at_end(start_of_list, new_elem)
}

pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = start;
    if start == end {
        new_elem.rangetype = RangeType::Single;
    } else {
        new_elem.rangetype = RangeType::StartEnd;
        new_elem.end = end;
    }

    // Smart-add behavior:
    // 1. If list has exactly one element which is (start, Single), replace it.
    // 2. If last element is (start, end, StartEnd) duplicate, skip.
    // 3. Otherwise, append.
    let len = list_len(&start_of_list);
    if let Some(last) = last_node(&start_of_list) {
        if last.rangetype == RangeType::StartEnd
            && last.start == start
            && last.end == end
        {
            return start_of_list;
        }
        if last.rangetype == RangeType::Single
            && last.start == start
            && start == end
        {
            return start_of_list;
        }
        if len == 1
            && last.rangetype == RangeType::Single
            && last.start == start
            && start != end
        {
            // Replace single-element list with the new range
            return Some(new_elem);
        }
    }
    append_at_end(start_of_list, new_elem)
}

pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::GreaterEqual;

    // Skip duplicate
    if let Some(last) = last_node(&start_of_list) {
        if last.rangetype == RangeType::GreaterEqual && last.start == num {
            return start_of_list;
        }
    }
    append_at_end(start_of_list, new_elem)
}

pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut cur = start_of_list;
    loop {
        match cur {
            None => return false,
            Some(node) => {
                if node.rangetype == RangeType::Single && node.start == num {
                    return true;
                }
                if node.rangetype == RangeType::GreaterEqual && num >= node.start {
                    return true;
                }
                if node.rangetype == RangeType::StartEnd && num >= node.start && num <= node.end {
                    return true;
                }
                cur = &node.next;
            }
        }
    }
}

pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
    buf.clear();
    let s = element_str(self);
    let truncated = if s.len() >= bufsize && bufsize > 0 {
        s[..bufsize.saturating_sub(1)].to_string()
    } else {
        s
    };
    buf.push_str(&truncated);
    ""
}

pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();

    // Collect elements
    let mut elements: Vec<&RangeElement> = Vec::new();
    let mut cur = start_of_list;
    while let Some(node) = cur {
        elements.push(node);
        cur = &node.next;
    }
    if elements.is_empty() {
        return "";
    }

    // Find the first index of the minimum start; output tail from there.
    let mut min_start = elements[0].start;
    let mut min_idx = 0;
    for (i, e) in elements.iter().enumerate() {
        if e.start < min_start {
            min_start = e.start;
            min_idx = i;
        }
    }

    for e in &elements[min_idx..] {
        let s = element_str(e);
        if buf.len() + s.len() < bufsize {
            let _ = write!(buf, "{}", s);
        } else if buf.len() < bufsize {
            let remaining = bufsize - buf.len();
            let truncated = &s[..remaining.min(s.len())];
            buf.push_str(truncated);
        }
    }

    ""
}

pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    // Build the list directly without using smart add_* — match C semantics
    fn raw_append(list: Option<Box<RangeElement>>, new_elem: Box<RangeElement>) -> Option<Box<RangeElement>> {
        append_at_end(list, new_elem)
    }
    fn raw_single(num: u32, list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::Single;
        raw_append(list, e)
    }
    fn raw_start_end(start: u32, end: u32, list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = start;
        if start == end {
            e.rangetype = RangeType::Single;
        } else {
            e.rangetype = RangeType::StartEnd;
            e.end = end;
        }
        raw_append(list, e)
    }
    fn raw_greater_equal(num: u32, list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::GreaterEqual;
        raw_append(list, e)
    }

    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

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
                    rv = raw_greater_equal(start, rv);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
                }
                rv = raw_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = raw_single(start, rv);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    // finish up
    if numbuf.is_empty() {
        if (state & flag_dash) != 0 {
            rv = raw_greater_equal(start, rv);
        }
    } else if start > 0 {
        end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
        }
        rv = raw_start_end(start, end, rv);
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        rv = raw_single(start, rv);
    }
    rv
}
}
}
