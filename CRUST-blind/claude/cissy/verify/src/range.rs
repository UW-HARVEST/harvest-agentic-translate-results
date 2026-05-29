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
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::Single;
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            // Walk to end and append
            {
                let mut ptr: &mut RangeElement = &mut *head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_deref_mut().unwrap();
                }
                ptr.next = Some(new_elem);
            }
            Some(head)
        }
    }
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
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = &mut *head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_deref_mut().unwrap();
                }
                ptr.next = Some(new_elem);
            }
            Some(head)
        }
    }
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut new_elem = Box::new(RangeElement::new());
    new_elem.start = num;
    new_elem.rangetype = RangeType::GreaterEqual;
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = &mut *head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_deref_mut().unwrap();
                }
                ptr.next = Some(new_elem);
            }
            Some(head)
        }
    }
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut cur = start_of_list.as_deref();
    while let Some(p) = cur {
        match p.rangetype {
            RangeType::Single => {
                if p.start == num {
                    return true;
                }
            }
            RangeType::GreaterEqual => {
                if num >= p.start {
                    return true;
                }
            }
            RangeType::StartEnd => {
                if num >= p.start && num <= p.end {
                    return true;
                }
            }
            RangeType::Empty => {}
        }
        cur = p.next.as_deref();
    }
    false
}
pub fn to_string(&self, buf: &mut String, _bufsize: usize) -> &str {
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
    // Return a 'static str; callers use buf directly. The elided
    // lifetime in the signature is tied to &self, and 'static
    // coerces to any lifetime.
    ""
}
pub fn list_to_string<'a>(buf: &'a mut String, _bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut cur = start_of_list.as_deref();
    while let Some(p) = cur {
        let mut tmp = String::new();
        p.to_string(&mut tmp, 256);
        buf.push_str(&tmp);
        cur = p.next.as_deref();
    }
    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

    let bytes = text.as_bytes();
    for &b in bytes {
        let ch = b as char;
        if ch.is_ascii_digit() {
            numbuf.push(ch);
        } else if ch == '-' {
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
        } else if ch == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = RangeElement::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1i32 as i32);
                }
                rv = RangeElement::add_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = RangeElement::add_single(start, rv);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    // finish up
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = RangeElement::add_greater_equal(start, rv);
        }
    } else if start > 0 {
        end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1i32 as i32);
        }
        rv = RangeElement::add_start_end(start, end, rv);
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        rv = RangeElement::add_single(start, rv);
    }
    rv
}
}
}
