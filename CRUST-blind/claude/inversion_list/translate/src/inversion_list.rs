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
            return Ok(Self {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort();
        let max_val = *sorted.last().unwrap();
        if max_val >= capacity {
            return Err(InversionListError::ValueOutOfRange(max_val, capacity));
        }
        // Deduplicate
        let mut dedup: Vec<u32> = Vec::with_capacity(sorted.len());
        for v in &sorted {
            if dedup.last() != Some(v) {
                dedup.push(*v);
            }
        }
        let support = dedup.len() as u32;
        // Build intervals: each contiguous run becomes one (start, end_exclusive)
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut start = dedup[0];
        let mut prev = dedup[0];
        for &v in &dedup[1..] {
            if v == prev + 1 {
                prev = v;
            } else {
                intervals.push((start, prev + 1));
                start = v;
                prev = v;
            }
        }
        intervals.push((start, prev + 1));
        Ok(Self {
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
        for &(lo, hi) in &self.intervals {
            if value >= lo && value < hi {
                return true;
            }
            if value < lo {
                return false;
            }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        // Complement w.r.t. [0, capacity)
        let mut new_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev_end: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if lo > prev_end {
                new_intervals.push((prev_end, lo));
            }
            prev_end = hi;
        }
        if prev_end < self.capacity {
            new_intervals.push((prev_end, self.capacity));
        }
        let new_support = self.capacity.saturating_sub(self.support);
        Self {
            capacity: self.capacity,
            support: new_support,
            intervals: new_intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&v.to_string());
            }
        }
        s.push(']');
        s
    }
    pub fn equal(&self, other: &Self) -> bool {
        if self.support != other.support {
            return false;
        }
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        for (a, b) in self.intervals.iter().zip(other.intervals.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Replicate C inversion_list_less semantics
        if self.support >= other.support {
            return false;
        }
        let max = match self.intervals.last() {
            Some(&(_, hi)) => hi,
            None => return false,
        };
        let mut i: u32 = 0;
        while i < max {
            if other.contains(i) {
                return true;
            }
            i = match i.checked_add(1) {
                Some(v) => v,
                None => break,
            };
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Replicate C inversion_list_disjoint semantics (returns not_equal)
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        if self.intervals.is_empty() && other.intervals.is_empty() {
            return Self {
                capacity: cap,
                support: 0,
                intervals: Vec::new(),
            };
        }
        let max1 = self.intervals.last().map(|p| p.1).unwrap_or(0);
        let max2 = other.intervals.last().map(|p| p.1).unwrap_or(0);
        let max = max1.max(max2);
        let min1 = self.intervals.first().map(|p| p.0).unwrap_or(u32::MAX);
        let min2 = other.intervals.first().map(|p| p.0).unwrap_or(u32::MAX);
        let mut i = min1.min(min2);
        let mut values: Vec<u32> = Vec::new();
        while i <= max {
            if self.contains(i) || other.contains(i) {
                values.push(i);
            }
            match i.checked_add(1) {
                Some(v) => i = v,
                None => break,
            }
        }
        Self::new(cap, &values).unwrap_or(Self {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let max1 = self.intervals.last().map(|p| p.1).unwrap_or(0);
        let max2 = other.intervals.last().map(|p| p.1).unwrap_or(0);
        let max = max1.min(max2);
        let mut values: Vec<u32> = Vec::new();
        let mut i: u32 = 0;
        while i < max {
            if self.contains(i) && other.contains(i) {
                values.push(i);
            }
            match i.checked_add(1) {
                Some(v) => i = v,
                None => break,
            }
        }
        Self::new(cap, &values).unwrap_or(Self {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let max = self.intervals.last().map(|p| p.1).unwrap_or(0);
        let mut values: Vec<u32> = Vec::new();
        let mut i: u32 = 0;
        while i < max {
            if self.contains(i) && !other.contains(i) {
                values.push(i);
            }
            match i.checked_add(1) {
                Some(v) => i = v,
                None => break,
            }
        }
        Self::new(cap, &values).unwrap_or(Self {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
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
        f.write_str(&self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = list.intervals.first().map(|p| p.0).unwrap_or(0);
        Self {
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
            let (lo, hi) = self.list.intervals[self.interval_index];
            if self.current_value < lo {
                self.current_value = lo;
            }
            if self.current_value < hi {
                let v = self.current_value;
                self.current_value = self.current_value.saturating_add(1);
                if self.current_value >= hi {
                    self.interval_index += 1;
                    if self.interval_index < self.list.intervals.len() {
                        self.current_value = self.list.intervals[self.interval_index].0;
                    }
                }
                return Some(v);
            } else {
                self.interval_index += 1;
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
        Self {
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
        let v = self.list.intervals[self.couple_index];
        self.couple_index += 1;
        Some(v)
    }
}
