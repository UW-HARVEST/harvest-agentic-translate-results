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
    // Mirrors rangeCreate: start/end set to "-1" (uint32 max) and Empty type.
    RangeElement {
        start: u32::MAX,
        end: u32::MAX,
        rangetype: RangeType::Empty,
        next: None,
    }
}
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    // assert(num > 0);
    assert!(num > 0);
    let new_elem = Box::new(RangeElement {
        start: num,
        end: u32::MAX,
        rangetype: RangeType::Single,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            // Walk to the end and append.
            {
                let mut ptr: &mut RangeElement = head.as_mut();
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
    let rangetype = if start == end { RangeType::Single } else { RangeType::StartEnd };
    let new_end = if start == end { u32::MAX } else { end };
    let new_elem = Box::new(RangeElement {
        start,
        end: new_end,
        rangetype,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = head.as_mut();
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
    let new_elem = Box::new(RangeElement {
        start: num,
        end: u32::MAX,
        rangetype: RangeType::GreaterEqual,
        next: None,
    });
    match start_of_list {
        None => Some(new_elem),
        Some(mut head) => {
            {
                let mut ptr: &mut RangeElement = head.as_mut();
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
    let mut cur = start_of_list.as_deref();
    while let Some(elem) = cur {
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
        cur = elem.next.as_deref();
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
    // Truncate to mimic C snprintf: at most bufsize-1 bytes (leaving room for nul).
    let max_len = if bufsize == 0 { 0 } else { bufsize - 1 };
    let take_len = formatted.len().min(max_len);
    // Take a UTF-8 safe prefix.
    let bytes = formatted.as_bytes();
    let mut take = take_len;
    while take > 0 && (bytes[take - 1] & 0b1100_0000) == 0b1000_0000 {
        take -= 1;
    }
    buf.push_str(&formatted[..take]);
    // Return empty &'static str to satisfy lifetime elision (return tied to &self).
    // Callers should read from `buf` directly.
    ""
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut cur = start_of_list.as_deref();
    while let Some(elem) = cur {
        let mut tmp = String::new();
        elem.to_string(&mut tmp, 256);
        let remaining = bufsize.saturating_sub(buf.len());
        if remaining == 0 {
            break;
        }
        // mimic C: copylen = ( (tblen+bufidx) >= bufsize ? bufsize-bufidx : tblen);
        // where tblen = sizeof(tmpbuf) = 256 (the buffer size, not strlen!).
        // The C strncat copies up to copylen bytes but stops at nul. So
        // effectively we copy up to min(strlen(tmp), bufsize - bufidx) bytes.
        let copylen = tmp.len().min(remaining);
        let bytes = tmp.as_bytes();
        let mut take = copylen;
        while take > 0 && (bytes[take - 1] & 0b1100_0000) == 0b1000_0000 {
            take -= 1;
        }
        buf.push_str(&tmp[..take]);
        cur = elem.next.as_deref();
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
            if (state & flag_dash) != 0 {
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
                if (state & flag_dash) != 0 {
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
        if (state & flag_dash) != 0 {
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
