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
                    ptr = ptr.next.as_mut().unwrap();
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
                    ptr = ptr.next.as_mut().unwrap();
                }
                ptr.next = Some(new_elem);
            }
            Some(head)
        }
    }
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    let mut ptr = start_of_list.as_deref();
    while let Some(elem) = ptr {
        match elem.rangetype {
            RangeType::Single => {
                if elem.start == num {
                    return true;
                }
            }
            RangeType::GreaterEqual => {
                if num >= elem.start {
                    return true;
                }
            }
            RangeType::StartEnd => {
                if num >= elem.start && num <= elem.end {
                    return true;
                }
            }
            RangeType::Empty => {}
        }
        ptr = elem.next.as_deref();
    }
    false
}
pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
    buf.clear();
    let formatted = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => String::from("[]"),
    };
    // emulate snprintf truncation: bufsize includes nul terminator,
    // so we keep at most bufsize-1 bytes when bufsize > 0
    if bufsize == 0 {
        return "";
    }
    let max = bufsize.saturating_sub(1);
    let bytes = formatted.as_bytes();
    let take = bytes.len().min(max);
    // ensure we don't split a UTF-8 char (all here are ASCII anyway)
    let mut take_safe = take;
    while take_safe > 0 && !formatted.is_char_boundary(take_safe) {
        take_safe -= 1;
    }
    buf.push_str(&formatted[..take_safe]);
    ""
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    if bufsize == 0 {
        return buf.as_str();
    }
    let mut ptr = start_of_list.as_deref();
    let mut bufidx: usize = 0;
    while let Some(elem) = ptr {
        let mut tmpbuf = String::new();
        elem.to_string(&mut tmpbuf, 256);
        let tblen = 256usize; // sizeof(tmpbuf) in C is the array size, 256
        let copylen = if tblen + bufidx >= bufsize {
            bufsize.saturating_sub(bufidx)
        } else {
            tblen
        };
        // strncat copies up to copylen bytes from tmpbuf (stops at NUL)
        let bytes = tmpbuf.as_bytes();
        let take = bytes.len().min(copylen);
        let mut take_safe = take;
        while take_safe > 0 && !tmpbuf.is_char_boundary(take_safe) {
            take_safe -= 1;
        }
        // ensure we don't exceed bufsize-1 (room for NUL)
        let remaining = bufsize.saturating_sub(1).saturating_sub(buf.len());
        let final_take = take_safe.min(remaining);
        buf.push_str(&tmpbuf[..final_take]);
        bufidx += copylen;
        ptr = elem.next.as_deref();
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

    for ch in text.chars() {
        if ch.is_ascii_digit() {
            numbuf.push(ch);
        } else if ch == '-' {
            if state & flag_dash != 0 {
                eprintln!("error: too many dashes in range");
                return rv;
            }
            state |= flag_dash;
            if numbuf.is_empty() {
                eprintln!("error: range cannot start with '-'");
                return rv;
            }
            start = numbuf.parse::<u32>().unwrap_or(0);
            if start == 0 {
                eprintln!("error: start range <= 0");
                return rv;
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
                    return rv;
                }
                rv = RangeElement::add_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                if start > 0 {
                    rv = RangeElement::add_single(start, rv);
                }
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
            return rv;
        }
        rv = RangeElement::add_start_end(start, end, rv);
    } else {
        start = numbuf.parse::<u32>().unwrap_or(0);
        if start > 0 {
            rv = RangeElement::add_single(start, rv);
        }
    }
    rv
}
}
}
