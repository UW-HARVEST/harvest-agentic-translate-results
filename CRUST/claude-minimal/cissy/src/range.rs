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

fn append(mut start_of_list: Option<Box<RangeElement>>, new_elem: Box<RangeElement>) -> Option<Box<RangeElement>> {
    match start_of_list.as_mut() {
        None => Some(new_elem),
        Some(head) => {
            let mut ptr: &mut RangeElement = head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(new_elem);
            start_of_list
        }
    }
}

pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    let mut elem = Box::new(RangeElement::new());
    elem.start = num;
    elem.rangetype = RangeType::Single;
    Self::append(start_of_list, elem)
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    let mut elem = Box::new(RangeElement::new());
    elem.start = start;
    if start == end {
        elem.rangetype = RangeType::Single;
    } else {
        elem.rangetype = RangeType::StartEnd;
        elem.end = end;
    }
    Self::append(start_of_list, elem)
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut elem = Box::new(RangeElement::new());
    elem.start = num;
    elem.rangetype = RangeType::GreaterEqual;
    Self::append(start_of_list, elem)
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut cursor = start_of_list.as_deref();
    while let Some(ptr) = cursor {
        match ptr.rangetype {
            RangeType::Single => {
                if ptr.start == num {
                    return true;
                }
            }
            RangeType::GreaterEqual => {
                if num >= ptr.start {
                    return true;
                }
            }
            RangeType::StartEnd => {
                if num >= ptr.start && num <= ptr.end {
                    return true;
                }
            }
            RangeType::Empty => {}
        }
        cursor = ptr.next.as_deref();
    }
    false
}
pub fn to_string<'a>(&self, buf: &'a mut String, bufsize: usize) -> &'a str {
    let s = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => "[]".to_string(),
    };
    let truncated = if s.len() >= bufsize && bufsize > 0 {
        &s[..bufsize - 1]
    } else {
        &s[..]
    };
    buf.push_str(truncated);
    &buf[..]
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut cursor = start_of_list.as_deref();
    while let Some(ptr) = cursor {
        let s = match ptr.rangetype {
            RangeType::Single => format!("[{}]", ptr.start),
            RangeType::GreaterEqual => format!("[{}-]", ptr.start),
            RangeType::StartEnd => format!("[{}-{}]", ptr.start, ptr.end),
            RangeType::Empty => "[]".to_string(),
        };
        let remaining = if buf.len() < bufsize {
            bufsize - buf.len()
        } else {
            0
        };
        let copylen = if s.len() > remaining { remaining } else { s.len() };
        if copylen > 0 {
            // ensure char boundary (all ascii here, safe)
            buf.push_str(&s[..copylen]);
        }
        cursor = ptr.next.as_deref();
    }
    &buf[..]
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

    for c in text.chars() {
        if c.is_ascii_digit() {
            numbuf.push(c);
        } else if c == '-' {
            if state & flag_dash != 0 {
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
        } else if c == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = Self::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
                }
                rv = Self::add_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = Self::add_single(start, rv);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    // finish up
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = Self::add_greater_equal(start, rv);
        }
    } else if start > 0 {
        end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
        }
        rv = Self::add_start_end(start, end, rv);
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        rv = Self::add_single(start, rv);
    }
    rv
}
}
}
