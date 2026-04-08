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
        start: 0,
        end: 0,
        rangetype: RangeType::Empty,
        next: None,
    }
}
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let new_elem = Box::new(RangeElement {
        start: num,
        end: 0,
        rangetype: RangeType::Single,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            let mut ptr = &mut head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(new_elem);
            Some(head)
        }
    }
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let rt = if start == end { RangeType::Single } else { RangeType::StartEnd };
    let new_elem = Box::new(RangeElement {
        start,
        end,
        rangetype: rt,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            let mut ptr = &mut head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(new_elem);
            Some(head)
        }
    }
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let new_elem = Box::new(RangeElement {
        start: num,
        end: 0,
        rangetype: RangeType::GreaterEqual,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            let mut ptr = &mut head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(new_elem);
            Some(head)
        }
    }
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut ptr = start_of_list;
    while let Some(elem) = ptr {
        match elem.rangetype {
            RangeType::Single => { if elem.start == num { return true; } }
            RangeType::GreaterEqual => { if num >= elem.start { return true; } }
            RangeType::StartEnd => { if num >= elem.start && num <= elem.end { return true; } }
            _ => {}
        }
        ptr = &elem.next;
    }
    false
}
pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
    buf.clear();
    let s = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => "[]".to_string(),
    };
    buf.push_str(&s[..s.len().min(bufsize)]);
    // Return lifetime is tied to &self by elision; buf is populated as side effect
    ""
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut ptr = start_of_list;
    while let Some(elem) = ptr {
        let s = match elem.rangetype {
            RangeType::Single => format!("[{}]", elem.start),
            RangeType::GreaterEqual => format!("[{}-]", elem.start),
            RangeType::StartEnd => format!("[{}-{}]", elem.start, elem.end),
            RangeType::Empty => "[]".to_string(),
        };
        let copylen = s.len().min(bufsize - buf.len());
        buf.push_str(&s[..copylen]);
        if buf.len() >= bufsize { break; }
        ptr = &elem.next;
    }
    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;

    for ch in text.chars() {
        if ch >= '0' && ch <= '9' {
            numbuf.push(ch);
        } else if ch == '-' {
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
        } else if ch == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = RangeElement::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                let end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
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
        let end = numbuf.parse::<u32>().unwrap_or(0);
        if end == 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
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
