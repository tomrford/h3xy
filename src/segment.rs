#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_address: u32,
    pub data: Vec<u8>,
}

impl Segment {
    pub fn new(start_address: u32, data: Vec<u8>) -> Self {
        debug_assert!(
            data.len() <= u32::MAX as usize,
            "segment data exceeds u32::MAX bytes"
        );
        Self {
            start_address,
            data,
        }
    }

    pub fn end_address(&self) -> u32 {
        if self.data.is_empty() {
            self.start_address
        } else {
            self.start_address + self.data.len() as u32 - 1
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_contiguous_with(&self, other: &Segment) -> bool {
        self.end_address() + 1 == other.start_address
    }

    pub fn merge(&mut self, other: Segment) {
        debug_assert!(self.is_contiguous_with(&other));
        self.data.extend(other.data);
    }
}
