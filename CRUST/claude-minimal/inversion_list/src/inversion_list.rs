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
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();

        // If the number of distinct values exceeds the capacity, the set is
        // fundamentally impossible to construct. Return an explicit error.
        if sorted.len() as u32 > capacity {
            let max = *sorted.last().unwrap();
            return Err(InversionListError::ValueOutOfRange(max, capacity));
        }

        // If any value exceeds the capacity, mimic the C library's silent
        // failure (where `inversion_list_create` returns NULL on EINVAL) by
        // producing an "empty" inversion list. Downstream binary operations
        // treat empty intervals as the identity, matching the C library's
        // NULL-handling behavior in `_union`, `_intersection`, and
        // `_difference`.
        if let Some(&max) = sorted.last() {
            if max >= capacity {
                return Ok(InversionList {
                    capacity,
                    support: 0,
                    intervals: Vec::new(),
                });
            }
        }

        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut support: u32 = 0;
        let mut prev: Option<u32> = None;
        for &v in &sorted {
            match prev {
                None => {
                    intervals.push((v, v + 1));
                    support += 1;
                }
                Some(p) => {
                    if v == p + 1 {
                        // extend last interval
                        let last = intervals.last_mut().unwrap();
                        last.1 = v + 1;
                        support += 1;
                    } else {
                        intervals.push((v, v + 1));
                        support += 1;
                    }
                }
            }
            prev = Some(v);
        }

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
        for &(start, end) in &self.intervals {
            if value >= start && value < end {
                return true;
            }
            if value < start {
                return false;
            }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut new_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev_end: u32 = 0;
        for &(start, end) in &self.intervals {
            if start > prev_end {
                new_intervals.push((prev_end, start));
            }
            prev_end = end;
        }
        if prev_end < self.capacity {
            new_intervals.push((prev_end, self.capacity));
        }
        InversionList {
            capacity: self.capacity,
            support: self.capacity - self.support,
            intervals: new_intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(start, end) in &self.intervals {
            for v in start..end {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
        }
        s.push(']');
        s
    }
    pub fn equal(&self, other: &Self) -> bool {
        if self.support != other.support || self.intervals.len() != other.intervals.len() {
            return false;
        }
        self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Mirror the C `inversion_list_less` logic:
        // - require self.support < other.support
        // - require that `other` has at least one element strictly less than
        //   self's largest stored value.
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let self_max = self.intervals.last().unwrap().1; // exclusive end
        for i in 0..self_max {
            if other.contains(i) {
                return true;
            }
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Mirror the C `inversion_list_disjoint` -> `not_equal` mapping.
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        // Treat empty intervals as the C-NULL identity.
        if other.intervals.is_empty() {
            return self.clone();
        }
        if self.intervals.is_empty() {
            return other.clone();
        }
        let cap = std::cmp::max(self.capacity, other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                values.push(v);
            }
        }
        for &(s, e) in &other.intervals {
            for v in s..e {
                values.push(v);
            }
        }
        InversionList::new(cap, &values).expect("union: failed to build set")
    }
    pub fn intersection(&self, other: &Self) -> Self {
        // Treat empty intervals as the C-NULL identity.
        if other.intervals.is_empty() {
            return self.clone();
        }
        if self.intervals.is_empty() {
            return other.clone();
        }
        let cap = std::cmp::max(self.capacity, other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                if other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("intersection: failed to build set")
    }
    pub fn difference(&self, other: &Self) -> Self {
        // Treat empty `other` as identity (matches the C-NULL pattern).
        if other.intervals.is_empty() {
            return self.clone();
        }
        let cap = std::cmp::max(self.capacity, other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                if !other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("difference: failed to build set")
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
        let current_value = list.intervals.first().map(|&(s, _)| s).unwrap_or(0);
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
        while self.interval_index < self.list.intervals.len() {
            let (start, end) = self.list.intervals[self.interval_index];
            if self.current_value < start {
                self.current_value = start;
            }
            if self.current_value >= end {
                self.interval_index += 1;
                if self.interval_index < self.list.intervals.len() {
                    self.current_value = self.list.intervals[self.interval_index].0;
                }
                continue;
            }
            let val = self.current_value;
            self.current_value += 1;
            return Some(val);
        }
        None
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
        if self.couple_index < self.list.intervals.len() {
            let pair = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(pair)
        } else {
            None
        }
    }
}
