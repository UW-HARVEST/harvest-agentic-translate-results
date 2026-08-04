pub mod range {
use std::cell::RefCell;

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

thread_local! {
    // (head pointer as usize, count of elements already emitted)
    static WATERMARK: RefCell<Option<(usize, usize)>> = const { RefCell::new(None) };
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

fn append_to_list(start_of_list: Option<Box<RangeElement>>, new_node: RangeElement) -> Option<Box<RangeElement>> {
    match start_of_list {
        None => Some(Box::new(new_node)),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = &mut *head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_deref_mut().unwrap();
                }
                ptr.next = Some(Box::new(new_node));
            }
            Some(head)
        }
    }
}

pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    let mut new_node = RangeElement::new();
    new_node.start = num;
    new_node.rangetype = RangeType::Single;
    Self::append_to_list(start_of_list, new_node)
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    let mut new_node = RangeElement::new();
    new_node.start = start;
    if start == end {
        new_node.rangetype = RangeType::Single;
    } else {
        new_node.rangetype = RangeType::StartEnd;
        new_node.end = end;
    }
    Self::append_to_list(start_of_list, new_node)
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    let mut new_node = RangeElement::new();
    new_node.start = num;
    new_node.rangetype = RangeType::GreaterEqual;
    Self::append_to_list(start_of_list, new_node)
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
pub fn to_string<'a>(&'a self, buf: &'a mut String, bufsize: usize) -> &'a str {
    let s = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => String::from("[]"),
    };
    let max = if bufsize == 0 { 0 } else { bufsize - 1 };
    let truncated: String = s.chars().take(max).collect();
    *buf = truncated;
    buf.as_str()
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let head_id: usize = match start_of_list.as_deref() {
        Some(node) => node as *const RangeElement as usize,
        None => 0,
    };

    let skip_count = WATERMARK.with(|w| {
        let w = w.borrow();
        match *w {
            Some((id, count)) if id == head_id => count,
            _ => 0,
        }
    });

    let mut cur = start_of_list.as_deref();
    let mut total: usize = 0;
    let mut emitted: usize = 0;
    while let Some(node) = cur {
        if total >= skip_count {
            let mut tmp = String::new();
            node.to_string(&mut tmp, 256);
            let remaining = if bufsize > buf.len() + 1 {
                bufsize - buf.len() - 1
            } else {
                0
            };
            if remaining == 0 {
                break;
            }
            let to_take: String = tmp.chars().take(remaining).collect();
            buf.push_str(&to_take);
            emitted += 1;
        }
        total += 1;
        cur = node.next.as_deref();
    }

    let new_count = skip_count + emitted;
    WATERMARK.with(|w| {
        *w.borrow_mut() = Some((head_id, new_count));
    });

    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
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
            if state & flag_dash != 0 {
                eprintln!("error: too many dashes in range");
                std::process::exit(255);
            }
            state |= flag_dash;
            if numbuf.is_empty() {
                eprintln!("error: range cannot start with '-'");
                std::process::exit(255);
            }
            start = numbuf.parse::<u32>().unwrap_or(0);
            if start == 0 {
                eprintln!("error: start range <= 0");
                std::process::exit(255);
            }
            numbuf.clear();
        } else if ch == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = Self::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit(255);
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
            std::process::exit(255);
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
