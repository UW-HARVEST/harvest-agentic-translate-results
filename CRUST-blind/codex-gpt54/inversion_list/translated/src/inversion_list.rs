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
        let mut sorted = values.to_vec();
        sorted.sort_unstable();

        if let Some(&value) = sorted.iter().find(|&&value| value >= capacity) {
            return Err(InversionListError::ValueOutOfRange(value, capacity));
        }

        sorted.dedup();

        let support = u32::try_from(sorted.len()).map_err(|_| {
            InversionListError::Generic("support does not fit into u32".to_string())
        })?;

        let mut intervals = Vec::new();
        if let Some(&first) = sorted.first() {
            let mut start = first;
            let mut end = first + 1;

            for &value in &sorted[1..] {
                if value == end {
                    end += 1;
                } else {
                    intervals.push((start, end));
                    start = value;
                    end = value + 1;
                }
            }

            intervals.push((start, end));
        }

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
        let mut low = 0usize;
        let mut high = self.intervals.len();

        while low < high {
            let mid = low + (high - low) / 2;
            let (start, end) = self.intervals[mid];
            if value < start {
                high = mid;
            } else if value >= end {
                low = mid + 1;
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
        let mut intervals = Vec::new();
        let mut cursor = 0;

        for &(start, end) in &self.intervals {
            if cursor < start {
                intervals.push((cursor, start));
            }
            cursor = end;
        }

        if cursor < self.capacity {
            intervals.push((cursor, self.capacity));
        }

        Self {
            capacity: self.capacity,
            support: self.capacity - self.support,
            intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut result = String::from("[");
        let mut first = true;

        for &(start, end) in &self.intervals {
            for value in start..end {
                if !first {
                    result.push_str(", ");
                }
                first = false;
                result.push_str(&value.to_string());
            }
        }

        result.push(']');
        result
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        if self.support >= other.support {
            return false;
        }

        let upper = self.intervals.last().map(|&(_, end)| end).unwrap_or(0);
        (0..upper).any(|value| other.contains(value))
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        let mut intervals = Vec::new();
        let mut left = 0usize;
        let mut right = 0usize;
        let mut current: Option<(u32, u32)> = None;

        while left < self.intervals.len() || right < other.intervals.len() {
            let next = match (self.intervals.get(left), other.intervals.get(right)) {
                (Some(&lhs), Some(&rhs)) => {
                    if lhs.0 <= rhs.0 {
                        left += 1;
                        lhs
                    } else {
                        right += 1;
                        rhs
                    }
                }
                (Some(&lhs), None) => {
                    left += 1;
                    lhs
                }
                (None, Some(&rhs)) => {
                    right += 1;
                    rhs
                }
                (None, None) => break,
            };

            match current {
                Some((start, end)) if next.0 <= end => {
                    current = Some((start, end.max(next.1)));
                }
                Some(interval) => {
                    intervals.push(interval);
                    current = Some(next);
                }
                None => current = Some(next),
            }
        }

        if let Some(interval) = current {
            intervals.push(interval);
        }

        let support = intervals.iter().map(|(start, end)| end - start).sum();

        Self {
            capacity: self.capacity.max(other.capacity),
            support,
            intervals,
        }
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let mut intervals = Vec::new();
        let mut left = 0usize;
        let mut right = 0usize;

        while left < self.intervals.len() && right < other.intervals.len() {
            let (left_start, left_end) = self.intervals[left];
            let (right_start, right_end) = other.intervals[right];

            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            if start < end {
                intervals.push((start, end));
            }

            if left_end < right_end {
                left += 1;
            } else if left_end > right_end {
                right += 1;
            } else {
                left += 1;
                right += 1;
            }
        }

        let support = intervals.iter().map(|(start, end)| end - start).sum();

        Self {
            capacity: self.capacity.max(other.capacity),
            support,
            intervals,
        }
    }
    pub fn difference(&self, other: &Self) -> Self {
        let mut intervals = Vec::new();
        let mut right = 0usize;

        for &(left_start, left_end) in &self.intervals {
            while right < other.intervals.len() && other.intervals[right].1 <= left_start {
                right += 1;
            }

            let mut cursor = left_start;
            let mut scan = right;

            while scan < other.intervals.len() {
                let (right_start, right_end) = other.intervals[scan];
                if right_start >= left_end {
                    break;
                }

                if cursor < right_start {
                    intervals.push((cursor, right_start.min(left_end)));
                }

                cursor = cursor.max(right_end);
                if cursor >= left_end {
                    break;
                }

                scan += 1;
            }

            if cursor < left_end {
                intervals.push((cursor, left_end));
            }
        }

        let support = intervals.iter().map(|(start, end)| end - start).sum();

        Self {
            capacity: self.capacity.max(other.capacity),
            support,
            intervals,
        }
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.union(other).difference(&self.intersection(other))
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
        let current_value = list.intervals.first().map(|&(start, _)| start).unwrap_or(0);
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
        while let Some(&(start, end)) = self.list.intervals.get(self.interval_index) {
            if self.current_value < start {
                self.current_value = start;
            }

            if self.current_value < end {
                let value = self.current_value;
                self.current_value += 1;
                if self.current_value >= end {
                    self.interval_index += 1;
                    if let Some(&(next_start, _)) = self.list.intervals.get(self.interval_index) {
                        self.current_value = next_start;
                    }
                }
                return Some(value);
            }

            self.interval_index += 1;
            if let Some(&(next_start, _)) = self.list.intervals.get(self.interval_index) {
                self.current_value = next_start;
            }
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
        Self {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.list.intervals.get(self.couple_index).copied();
        if next.is_some() {
            self.couple_index += 1;
        }
        next
    }
}
