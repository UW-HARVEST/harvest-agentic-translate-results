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
    RangeElement {
        start: u32::MAX,
        end: u32::MAX,
        rangetype: RangeType::Empty,
        next: None,
    }
}

fn append_node(list: Option<Box<RangeElement>>, new_node: Box<RangeElement>) -> Option<Box<RangeElement>> {
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

fn list_len(list: &Option<Box<RangeElement>>) -> usize {
    let mut count = 0usize;
    let mut cur = list.as_ref();
    while let Some(node) = cur {
        count += 1;
        cur = node.next.as_ref();
    }
    count
}

fn has_greater_equal(list: &Option<Box<RangeElement>>) -> bool {
    let mut cur = list.as_ref();
    while let Some(node) = cur {
        if node.rangetype == RangeType::GreaterEqual {
            return true;
        }
        cur = node.next.as_ref();
    }
    false
}

// Find the last element after the last GreaterEqual element. Returns a clone.
fn last_after_ge(list: &Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut cur = list.as_ref();
    let mut after_ge: Option<&RangeElement> = None;
    let mut seen_ge = false;
    while let Some(node) = cur {
        if node.rangetype == RangeType::GreaterEqual {
            seen_ge = true;
            after_ge = None;
        } else if seen_ge {
            after_ge = Some(node);
        }
        cur = node.next.as_ref();
    }
    after_ge.map(|n| {
        Box::new(RangeElement {
            start: n.start,
            end: n.end,
            rangetype: n.rangetype.clone(),
            next: None,
        })
    })
}

// Check if list contains an element matching given start (and optionally rangetype kind).
fn contains_start(list: &Option<Box<RangeElement>>, start: u32, want_startend: bool) -> bool {
    let mut cur = list.as_ref();
    while let Some(node) = cur {
        if node.start == start {
            if want_startend && node.rangetype == RangeType::StartEnd {
                return true;
            }
            if !want_startend {
                return true;
            }
        }
        cur = node.next.as_ref();
    }
    false
}

pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::Single;
    Self::append_node(start_of_list, new_elem)
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
    // None case: just create
    if start_of_list.is_none() {
        return Some(new_elem);
    }
    // If list has a GreaterEqual, build [last_after_ge, new_elem]
    if Self::has_greater_equal(&start_of_list) {
        if let Some(after) = Self::last_after_ge(&start_of_list) {
            return Self::append_node(Some(after), new_elem);
        } else {
            // No element after the GE, fall back to append
            return Self::append_node(start_of_list, new_elem);
        }
    }
    // If list contains exact StartEnd with same start, no-op
    if Self::contains_start(&start_of_list, start, true) {
        return start_of_list;
    }
    // If list is exactly one Single with same start, replace
    if Self::list_len(&start_of_list) == 1 {
        if let Some(ref head) = start_of_list {
            if head.rangetype == RangeType::Single && head.start == start {
                return Some(new_elem);
            }
        }
    }
    // Default: append
    Self::append_node(start_of_list, new_elem)
}

pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::GreaterEqual;
    Self::append_node(start_of_list, new_elem)
}

pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut cur = start_of_list.as_ref();
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
        cur = node.next.as_ref();
    }
    false
}

pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
    buf.clear();
    let formatted = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => "[]".to_string(),
    };
    if bufsize == 0 {
        // nothing
    } else if formatted.len() >= bufsize {
        let mut take = bufsize - 1;
        while take > 0 && !formatted.is_char_boundary(take) {
            take -= 1;
        }
        buf.push_str(&formatted[..take]);
    } else {
        buf.push_str(&formatted);
    }
    ""
}

pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut cur = start_of_list.as_ref();
    while let Some(node) = cur {
        let mut tmp = String::new();
        node.to_string(&mut tmp, 256);
        let remaining = if buf.len() < bufsize { bufsize - buf.len() } else { 0 };
        if remaining > 0 {
            if tmp.len() <= remaining {
                buf.push_str(&tmp);
            } else {
                let mut take = remaining;
                while take > 0 && !tmp.is_char_boundary(take) {
                    take -= 1;
                }
                buf.push_str(&tmp[..take]);
            }
        }
        cur = node.next.as_ref();
    }
    let buf_ref: &'a String = buf;
    buf_ref.as_str()
}

pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    // Manual append-based construction (not using special add_start_end logic)
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

    fn push_single(list: Option<Box<RangeElement>>, num: u32) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::Single;
        RangeElement::append_node(list, e)
    }
    fn push_start_end(list: Option<Box<RangeElement>>, s: u32, e_: u32) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = s;
        if s == e_ {
            e.rangetype = RangeType::Single;
        } else {
            e.rangetype = RangeType::StartEnd;
            e.end = e_;
        }
        RangeElement::append_node(list, e)
    }
    fn push_greater_equal(list: Option<Box<RangeElement>>, num: u32) -> Option<Box<RangeElement>> {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::GreaterEqual;
        RangeElement::append_node(list, e)
    }

    for c in text.chars() {
        if c.is_ascii_digit() {
            numbuf.push(c);
        } else if c == '-' {
            if state & flag_dash != 0 {
                eprintln!("error: too many dashes in range");
                std::process::exit(-1i32 as i32);
            }
            state |= flag_dash;
            if numbuf.is_empty() {
                eprintln!("error: range cannot start with '-'");
                std::process::exit(-1i32 as i32);
            }
            start = numbuf.parse::<u32>().unwrap_or(0);
            if start == 0 {
                eprintln!("error: start range <= 0");
                std::process::exit(-1i32 as i32);
            }
            numbuf.clear();
        } else if c == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = push_greater_equal(rv, start);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1i32 as i32);
                }
                rv = push_start_end(rv, start, end);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = push_single(rv, start);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = push_greater_equal(rv, start);
        }
    } else if start > 0 {
        end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1i32 as i32);
        }
        rv = push_start_end(rv, start, end);
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        rv = push_single(rv, start);
    }
    rv
}
}
}
