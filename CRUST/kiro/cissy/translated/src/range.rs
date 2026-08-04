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
    RangeElement { start: u32::MAX, end: u32::MAX, rangetype: RangeType::Empty, next: None }
}

fn append_element(mut head: Box<RangeElement>, new_el: RangeElement) -> Box<RangeElement> {
    let mut ptr = &mut *head;
    while ptr.next.is_some() {
        ptr = ptr.next.as_mut().unwrap();
    }
    ptr.next = Some(Box::new(new_el));
    head
}

pub fn add_single(num: u32, _start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    Some(Box::new(RangeElement { start: num, end: u32::MAX, rangetype: RangeType::Single, next: None }))
}
pub fn add_start_end(start: u32, end: u32, _start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    let (rt, e) = if start == end { (RangeType::Single, u32::MAX) } else { (RangeType::StartEnd, end) };
    Some(Box::new(RangeElement { start, end: e, rangetype: rt, next: None }))
}
pub fn add_greater_equal(num: u32, _start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    Some(Box::new(RangeElement { start: num, end: u32::MAX, rangetype: RangeType::GreaterEqual, next: None }))
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut ptr = start_of_list;
    loop {
        match ptr {
            None => return false,
            Some(el) => {
                match el.rangetype {
                    RangeType::Single if el.start == num => return true,
                    RangeType::GreaterEqual if num >= el.start => return true,
                    RangeType::StartEnd if num >= el.start && num <= el.end => return true,
                    _ => {}
                }
                ptr = &el.next;
            }
        }
    }
}
pub fn to_string<'a>(&self, buf: &'a mut String, _bufsize: usize) -> &'a str {
    self.to_string_into(buf);
    buf.as_str()
}
pub fn list_to_string<'a>(buf: &'a mut String, _bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut ptr = start_of_list;
    while let Some(el) = ptr {
        el.to_string_into(buf);
        ptr = &el.next;
    }
    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: i32 = 0;

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
            start = numbuf.parse::<i32>().unwrap_or(0);
            if start <= 0 {
                eprintln!("error: start range <= 0");
                std::process::exit(-1);
            }
            numbuf.clear();
        } else if c == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: u32::MAX, rangetype: RangeType::GreaterEqual, next: None });
                }
            } else if start > 0 {
                let end = numbuf.parse::<i32>().unwrap_or(0);
                if end <= 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
                }
                let (rt, e) = if start == end { (RangeType::Single, u32::MAX) } else { (RangeType::StartEnd, end as u32) };
                rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: e, rangetype: rt, next: None });
            } else {
                start = numbuf.parse::<i32>().unwrap_or(0);
                rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: u32::MAX, rangetype: RangeType::Single, next: None });
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    // finish up
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: u32::MAX, rangetype: RangeType::GreaterEqual, next: None });
        }
    } else if start > 0 {
        let end = numbuf.parse::<i32>().unwrap_or(0);
        if end <= 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
        }
        let (rt, e) = if start == end { (RangeType::Single, u32::MAX) } else { (RangeType::StartEnd, end as u32) };
        rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: e, rangetype: rt, next: None });
    } else {
        start = numbuf.parse::<i32>().unwrap_or(0);
        rv = Self::append_to_list(rv, RangeElement { start: start as u32, end: u32::MAX, rangetype: RangeType::Single, next: None });
    }
    rv
}

fn append_to_list(list: Option<Box<RangeElement>>, el: RangeElement) -> Option<Box<RangeElement>> {
    match list {
        None => Some(Box::new(el)),
        Some(mut head) => {
            let mut ptr = &mut *head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(Box::new(el));
            Some(head)
        }
    }
}

fn to_string_into(&self, buf: &mut String) {
    match self.rangetype {
        RangeType::Single => buf.push_str(&format!("[{}]", self.start)),
        RangeType::GreaterEqual => buf.push_str(&format!("[{}-]", self.start)),
        RangeType::StartEnd => buf.push_str(&format!("[{}-{}]", self.start, self.end)),
        RangeType::Empty => buf.push_str("[]"),
    }
}
}
}
