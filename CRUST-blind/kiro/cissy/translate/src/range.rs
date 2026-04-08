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
    let new_elem = RangeElement {
        start: num,
        end: u32::MAX,
        rangetype: RangeType::Single,
        next: None,
    };
    Self::append_to_list(start_of_list, new_elem)
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let new_elem = if start == end {
        RangeElement { start, end: u32::MAX, rangetype: RangeType::Single, next: None }
    } else {
        RangeElement { start, end, rangetype: RangeType::StartEnd, next: None }
    };
    Self::append_to_list(start_of_list, new_elem)
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let new_elem = RangeElement {
        start: num,
        end: u32::MAX,
        rangetype: RangeType::GreaterEqual,
        next: None,
    };
    Self::append_to_list(start_of_list, new_elem)
}

fn append_to_list(start_of_list: Option<Box<RangeElement>>, new_elem: RangeElement) -> Option<Box<RangeElement>> {
    match start_of_list {
        None => Some(Box::new(new_elem)),
        Some(mut head) => {
            let mut ptr = &mut head;
            while ptr.next.is_some() {
                ptr = ptr.next.as_mut().unwrap();
            }
            ptr.next = Some(Box::new(new_elem));
            Some(head)
        }
    }
}

pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut ptr = start_of_list;
    while let Some(elem) = ptr {
        match elem.rangetype {
            RangeType::Single if elem.start == num => return true,
            RangeType::GreaterEqual if num >= elem.start => return true,
            RangeType::StartEnd if num >= elem.start && num <= elem.end => return true,
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
    // Return lifetime is tied to &self; buf is the actual output
    ""
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut ptr = start_of_list;
    while let Some(elem) = ptr {
        let tmp = match elem.rangetype {
            RangeType::Single => format!("[{}]", elem.start),
            RangeType::GreaterEqual => format!("[{}-]", elem.start),
            RangeType::StartEnd => format!("[{}-{}]", elem.start, elem.end),
            RangeType::Empty => "[]".to_string(),
        };
        if buf.len() + tmp.len() <= bufsize {
            buf.push_str(&tmp);
        } else {
            let remaining = bufsize.saturating_sub(buf.len());
            buf.push_str(&tmp[..remaining]);
            break;
        }
        ptr = &elem.next;
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
                    rv = Self::add_greater_equal(start as u32, rv);
                }
            } else if start > 0 {
                let end = numbuf.parse::<i32>().unwrap_or(0);
                if end <= 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(-1);
                }
                rv = Self::add_start_end(start as u32, end as u32, rv);
            } else {
                start = numbuf.parse::<i32>().unwrap_or(0);
                rv = Self::add_single(start as u32, rv);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
    }
    // finish up
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = Self::add_greater_equal(start as u32, rv);
        }
    } else if start > 0 {
        let end = numbuf.parse::<i32>().unwrap_or(0);
        if end <= 0 || start > end {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit(-1);
        }
        rv = Self::add_start_end(start as u32, end as u32, rv);
    } else {
        start = numbuf.parse::<i32>().unwrap_or(0);
        rv = Self::add_single(start as u32, rv);
    }
    rv
}
}
}
