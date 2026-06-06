use std::fmt;
#[derive(Debug)]
pub enum InversionListError {
    ValueOutOfRange(u32, u32),
    Generic(String),
}
#[derive(Clone, Debug)]
pub struct InversionList {
    capacity: u32,
    support: u32,
    pub intervals: Vec<(u32, u32)>,
}
impl InversionList {
    pub fn new(capacity: u32, values: &[u32]) -> Result<Self, InversionListError> {
        if values.is_empty() {
            return Ok(InversionList {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }

        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort();

        let last = sorted[sorted.len() - 1];
        if last >= capacity {
            return Err(InversionListError::ValueOutOfRange(last, capacity));
        }

        // Build intervals (half-open [a, b)) from sorted, deduplicated values.
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut support: u32 = 0;
        let mut start = sorted[0];
        let mut end = start + 1;
        support += 1;
        for i in 1..sorted.len() {
            let v = sorted[i];
            if v == sorted[i - 1] {
                continue;
            }
            support += 1;
            if v == end {
                end = v + 1;
            } else {
                intervals.push((start, end));
                start = v;
                end = v + 1;
            }
        }
        intervals.push((start, end));

        Ok(InversionList {
            capacity,
            support,
            intervals,
        })
    }
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    pub fn support(&self) -> u32 {
        self.support
    }
    pub fn contains(&self, value: u32) -> bool {
        // Binary search for the interval that may contain `value`.
        // Each interval (a, b) means a <= x < b is in the set.
        let mut lo = 0usize;
        let mut hi = self.intervals.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (a, b) = self.intervals[mid];
            if value < a {
                hi = mid;
            } else if value >= b {
                lo = mid + 1;
            } else {
                return true;
            }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev: u32 = 0;
        for &(a, b) in &self.intervals {
            if a > prev {
                intervals.push((prev, a));
            }
            prev = b;
        }
        if prev < self.capacity {
            intervals.push((prev, self.capacity));
        }
        InversionList {
            capacity: self.capacity,
            support: self.capacity.saturating_sub(self.support),
            intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
                v += 1;
            }
        }
        s.push(']');
        s
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        self.is_subset_of(other) && !self.equal(other)
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        // Every element of self must be in other.
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                if !other.contains(v) {
                    return false;
                }
                v += 1;
            }
        }
        true
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // No shared elements.
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                if other.contains(v) {
                    return false;
                }
                v += 1;
            }
        }
        true
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::with_capacity(
            (self.support as usize).saturating_add(other.support as usize),
        );
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                values.push(v);
                v += 1;
            }
        }
        for &(a, b) in &other.intervals {
            let mut v = a;
            while v < b {
                values.push(v);
                v += 1;
            }
        }
        InversionList::new(cap, &values).unwrap_or(InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                if other.contains(v) {
                    values.push(v);
                }
                v += 1;
            }
        }
        InversionList::new(cap, &values).unwrap_or(InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(a, b) in &self.intervals {
            let mut v = a;
            while v < b {
                if !other.contains(v) {
                    values.push(v);
                }
                v += 1;
            }
        }
        InversionList::new(cap, &values).unwrap_or(InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let union = self.union(other);
        let intersection = self.intersection(other);
        union.difference(&intersection)
    }
}
impl PartialEq for InversionList {
    fn eq(&self, other: &Self) -> bool {
        self.equal(other)
    }
}
impl Eq for InversionList {}
impl fmt::Display for InversionList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = if list.intervals.is_empty() {
            0
        } else {
            list.intervals[0].0
        };
        InversionListIterator {
            list,
            interval_index: 0,
            current_value,
        }
    }
}
impl<'a> Iterator for InversionListIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.interval_index >= self.list.intervals.len() {
                return None;
            }
            let (start, end) = self.list.intervals[self.interval_index];
            if self.current_value < start {
                self.current_value = start;
            }
            if self.current_value < end {
                let val = self.current_value;
                self.current_value += 1;
                if self.current_value >= end {
                    self.interval_index += 1;
                    if self.interval_index < self.list.intervals.len() {
                        self.current_value = self.list.intervals[self.interval_index].0;
                    }
                }
                return Some(val);
            } else {
                self.interval_index += 1;
                if self.interval_index < self.list.intervals.len() {
                    self.current_value = self.list.intervals[self.interval_index].0;
                }
            }
        }
    }
}
pub struct InversionListCoupleIterator<'a> {
    list: &'a InversionList,
    couple_index: usize,
}
impl<'a> InversionListCoupleIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        InversionListCoupleIterator {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index >= self.list.intervals.len() {
            return None;
        }
        let val = self.list.intervals[self.couple_index];
        self.couple_index += 1;
        Some(val)
    }
}
