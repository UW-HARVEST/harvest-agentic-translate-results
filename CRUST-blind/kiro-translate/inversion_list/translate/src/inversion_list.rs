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
            return Ok(InversionList { capacity, support: 0, intervals: Vec::new() });
        }
        let mut sorted = values.to_vec();
        sorted.sort();
        if *sorted.last().unwrap() >= capacity {
            return Err(InversionListError::ValueOutOfRange(*sorted.last().unwrap(), capacity));
        }
        sorted.dedup();
        let support = sorted.len() as u32;
        let mut intervals = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0] + 1;
        for &v in &sorted[1..] {
            if v == end {
                end = v + 1;
            } else {
                intervals.push((start, end));
                start = v;
                end = v + 1;
            }
        }
        intervals.push((start, end));
        Ok(InversionList { capacity, support, intervals })
    }
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    pub fn support(&self) -> u32 {
        self.support
    }
    pub fn contains(&self, value: u32) -> bool {
        for &(lo, hi) in &self.intervals {
            if value < lo { return false; }
            if value < hi { return true; }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut intervals = Vec::new();
        let mut prev = 0u32;
        for &(lo, hi) in &self.intervals {
            if lo > prev {
                intervals.push((prev, lo));
            }
            prev = hi;
        }
        if prev < self.capacity {
            intervals.push((prev, self.capacity));
        }
        InversionList { capacity: self.capacity, support: self.capacity - self.support, intervals }
    }
    pub fn to_str(&self) -> String {
        self.to_string()
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        if self.support >= other.support { return false; }
        if self.intervals.is_empty() { return other.support > 0; }
        let max_self = self.intervals.last().unwrap().1;
        for i in 0..max_self {
            if other.contains(i) { return true; }
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut vals = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi { vals.push(v); }
        }
        for &(lo, hi) in &other.intervals {
            for v in lo..hi { vals.push(v); }
        }
        InversionList::new(cap, &vals).unwrap()
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut vals = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if other.contains(v) { vals.push(v); }
            }
        }
        InversionList::new(cap, &vals).unwrap()
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut vals = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if !other.contains(v) { vals.push(v); }
            }
        }
        InversionList::new(cap, &vals).unwrap()
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let u = self.union(other);
        let i = self.intersection(other);
        u.difference(&i)
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
        let vals: Vec<String> = self.intervals.iter()
            .flat_map(|&(lo, hi)| (lo..hi).map(|v| v.to_string()))
            .collect();
        write!(f, "[{}]", vals.join(", "))
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = list.intervals.first().map_or(0, |&(lo, _)| lo);
        InversionListIterator { list, interval_index: 0, current_value }
    }
}
impl<'a> Iterator for InversionListIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.interval_index >= self.list.intervals.len() {
            return None;
        }
        let (_, hi) = self.list.intervals[self.interval_index];
        if self.current_value < hi {
            let val = self.current_value;
            self.current_value += 1;
            if self.current_value >= hi {
                self.interval_index += 1;
                if self.interval_index < self.list.intervals.len() {
                    self.current_value = self.list.intervals[self.interval_index].0;
                }
            }
            Some(val)
        } else {
            self.interval_index += 1;
            if self.interval_index < self.list.intervals.len() {
                self.current_value = self.list.intervals[self.interval_index].0;
                self.next()
            } else {
                None
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
        InversionListCoupleIterator { list, couple_index: 0 }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index >= self.list.intervals.len() {
            return None;
        }
        let pair = self.list.intervals[self.couple_index];
        self.couple_index += 1;
        Some(pair)
    }
}
