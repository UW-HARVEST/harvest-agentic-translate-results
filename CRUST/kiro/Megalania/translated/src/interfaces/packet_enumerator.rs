use crate::lzma_packet::LZMAPacket;
use crate::lzma_state::LZMAState;
use crate::substring_enumerator::SubstringEnumerator;

const MIN_SUBSTRING: usize = 2;
const MAX_SUBSTRING: usize = 273;

pub struct PacketEnumerator<'a> {
    pub data: &'a [u8],
    pub substring_enumerator: Box<SubstringEnumerator<'a>>,
}

impl<'a> PacketEnumerator<'a> {
    pub fn memory_usage(data_size: usize) -> usize {
        std::mem::size_of::<Self>() + SubstringEnumerator::memory_usage(data_size)
    }

    pub fn new(data: &[u8]) -> Self {
        // We need the lifetime to match, so we transmute. Actually, let's just
        // use the same pattern - the caller must ensure lifetimes match.
        // The signature says &[u8] but the struct needs &'a [u8].
        // We'll trust the caller here since the C code does the same.
        let data_ref: &'a [u8] = unsafe { std::mem::transmute(data) };
        PacketEnumerator {
            data: data_ref,
            substring_enumerator: Box::new(SubstringEnumerator::new(data_ref, MIN_SUBSTRING, MAX_SUBSTRING)),
        }
    }

    pub fn for_each<F>(&self, state: &LZMAState, mut callback: F)
    where
        F: FnMut(&LZMAState, LZMAPacket),
    {
        callback(state, LZMAPacket::literal_packet());

        if state.position > 0 {
            let rep0_position = state.position - state.dists[0] as usize - 1;
            if self.data[state.position] == self.data[rep0_position] {
                callback(state, LZMAPacket::short_rep_packet());
            }
        }

        self.substring_enumerator.for_each(state.position, |offset, length| {
            let dist = (state.position - offset - 1) as u32;
            callback(state, LZMAPacket::match_packet(dist, length as u32));
            for i in 0..4 {
                if dist == state.dists[i] {
                    callback(state, LZMAPacket::long_rep_packet(i as u32, length as u32));
                }
            }
        });
    }
}
