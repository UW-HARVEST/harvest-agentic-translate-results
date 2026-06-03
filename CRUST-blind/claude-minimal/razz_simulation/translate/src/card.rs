pub mod card {
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

    pub const CLUB_BITS: u32 = 4 << 5;
    pub const ACE_BITS: u32 = 1;
    pub const J_BITS: u32 = 11;
    pub const R8_BITS: u32 = 8;
    pub const R10_BITS: u32 = 10;
    pub const HEART_BITS: u32 = 2 << 5;
    pub const R4_BITS: u32 = 4;
    pub const SPADE_BITS: u32 = 1 << 5;
    pub const Q_BITS: u32 = 12;
    pub const R7_BITS: u32 = 7;
    pub const R5_BITS: u32 = 5;
    pub const K_BITS: u32 = 13;
    pub const R3_BITS: u32 = 3;
    pub const RANK_BITS: u32 = 0x1F;
    pub const R9_BITS: u32 = 9;
    pub const SUIT_BITS: u32 = 0x7 << 5;
    pub const R6_BITS: u32 = 6;
    pub const INVALID_CARD_BITS: u32 = 0;
    pub const DIAMOND_BITS: u32 = 3 << 5;
    pub const R2_BITS: u32 = 2;

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuitRank {
        SpadeAce, Spade2, Spade3, Spade4, Spade5, Spade6, Spade7, Spade8,
        Spade9, Spade10, SpadeJ, SpadeQ, SpadeK,
        HeartAce, Heart2, Heart3, Heart4, Heart5, Heart6, Heart7, Heart8,
        Heart9, Heart10, HeartJ, HeartQ, HeartK,
        DiamondAce, Diamond2, Diamond3, Diamond4, Diamond5, Diamond6,
        Diamond7, Diamond8, Diamond9, Diamond10, DiamondJ, DiamondQ, DiamondK,
        ClubAce, Club2, Club3, Club4, Club5, Club6, Club7, Club8, Club9,
        Club10, ClubJ, ClubQ, ClubK,
        CardCount,
        InvalidCard,
    }

    const ALL_CSR: [CardSuitRank; 52] = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
        CardSuitRank::Spade4, CardSuitRank::Spade5, CardSuitRank::Spade6,
        CardSuitRank::Spade7, CardSuitRank::Spade8, CardSuitRank::Spade9,
        CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3,
        CardSuitRank::Heart4, CardSuitRank::Heart5, CardSuitRank::Heart6,
        CardSuitRank::Heart7, CardSuitRank::Heart8, CardSuitRank::Heart9,
        CardSuitRank::Heart10, CardSuitRank::HeartJ, CardSuitRank::HeartQ,
        CardSuitRank::HeartK,
        CardSuitRank::DiamondAce, CardSuitRank::Diamond2, CardSuitRank::Diamond3,
        CardSuitRank::Diamond4, CardSuitRank::Diamond5, CardSuitRank::Diamond6,
        CardSuitRank::Diamond7, CardSuitRank::Diamond8, CardSuitRank::Diamond9,
        CardSuitRank::Diamond10, CardSuitRank::DiamondJ, CardSuitRank::DiamondQ,
        CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3,
        CardSuitRank::Club4, CardSuitRank::Club5, CardSuitRank::Club6,
        CardSuitRank::Club7, CardSuitRank::Club8, CardSuitRank::Club9,
        CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK,
    ];

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            const STRS: [&str; 52] = [
                "SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK",
                "HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK",
                "DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK",
                "CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK",
            ];
            let idx = *self as u32;
            if idx >= 52 {
                None
            } else {
                Some(STRS[idx as usize].to_string())
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    const ALL_RANKS: [CardRank; 13] = [
        CardRank::Ace, CardRank::R2, CardRank::R3, CardRank::R4, CardRank::R5,
        CardRank::R6, CardRank::R7, CardRank::R8, CardRank::R9, CardRank::R10,
        CardRank::J, CardRank::Q, CardRank::K,
    ];

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            const STRS: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let idx = *self as u32;
            if idx >= 13 {
                None
            } else {
                Some(STRS[idx as usize].to_string())
            }
        }

        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let mut idx: u32 = 0; // ACE
            let c0 = bytes[0] as char;
            if c0 >= '2' && c0 <= '9' {
                idx += (c0 as u32) - ('1' as u32);
            } else {
                match c0.to_ascii_uppercase() {
                    'A' => {}
                    '1' => {
                        if bytes.len() >= 2 && bytes[1] == b'0' {
                            idx += 9;
                        } else {
                            return CardRank::InvalidRank;
                        }
                    }
                    'J' => idx += 10,
                    'Q' => idx += 11,
                    'K' => idx += 12,
                    _ => return CardRank::InvalidRank,
                }
            }
            if idx < 13 {
                ALL_RANKS[idx as usize]
            } else {
                CardRank::InvalidRank
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Card {
        card: u8,
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let csr_idx = csr as u32;
            let mut card_bits: u8 = INVALID_CARD_BITS as u8;

            if csr_idx <= CardSuitRank::SpadeK as u32 {
                card_bits |= SPADE_BITS as u8;
            } else if csr_idx >= CardSuitRank::HeartAce as u32
                && csr_idx <= CardSuitRank::HeartK as u32
            {
                card_bits |= HEART_BITS as u8;
            } else if csr_idx >= CardSuitRank::DiamondAce as u32
                && csr_idx <= CardSuitRank::DiamondK as u32
            {
                card_bits |= DIAMOND_BITS as u8;
            } else if csr_idx >= CardSuitRank::ClubAce as u32
                && csr_idx <= CardSuitRank::ClubK as u32
            {
                card_bits |= CLUB_BITS as u8;
            } else {
                return Card { card: 0 };
            }

            // rank bit = (csr_idx % 13) + 1
            let rank_bits: u8 = ((csr_idx % 13) as u8) + 1;
            card_bits |= rank_bits;

            Card { card: card_bits }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();

            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }

            let base: u32 = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as u32,
                CardSuit::Heart => CardSuitRank::HeartAce as u32,
                CardSuit::Diamond => CardSuitRank::DiamondAce as u32,
                CardSuit::Club => CardSuitRank::ClubAce as u32,
                _ => return CardSuitRank::InvalidCard,
            };

            let idx = base + (cr as u32);
            if (idx as usize) < ALL_CSR.len() {
                ALL_CSR[idx as usize]
            } else {
                CardSuitRank::InvalidCard
            }
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            ALL_RANKS[(r - 1) as usize]
        }

        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card as u32) & SUIT_BITS;
            if s < SPADE_BITS || s > CLUB_BITS {
                return CardSuit::InvalidSuit;
            }
            match (s >> 5) - 1 {
                0 => CardSuit::Spade,
                1 => CardSuit::Heart,
                2 => CardSuit::Diamond,
                3 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Self::write_card(csr);
            if c.card == INVALID_CARD_BITS as u8 {
                None
            } else {
                Some(c)
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            let char_count = bytes.len();
            if char_count != 2 {
                return None;
            }
            let csr_base: u32 = match (bytes[0] as char).to_ascii_uppercase() {
                'S' => CardSuitRank::SpadeAce as u32,
                'H' => CardSuitRank::HeartAce as u32,
                'D' => CardSuitRank::DiamondAce as u32,
                'C' => CardSuitRank::ClubAce as u32,
                _ => return None,
            };
            let c2 = bytes[1] as char;
            if c2 >= '2' && c2 <= '9' {
                let offset = (c2 as u32) - ('1' as u32);
                return Self::create_card(ALL_CSR[(csr_base + offset) as usize]);
            }
            match c2.to_ascii_uppercase() {
                'A' => Self::create_card(ALL_CSR[csr_base as usize]),
                '1' => {
                    if char_count == 3 {
                        Self::create_card(ALL_CSR[(csr_base + 9) as usize])
                    } else {
                        None
                    }
                }
                'J' => Self::create_card(ALL_CSR[(csr_base + 10) as usize]),
                'Q' => Self::create_card(ALL_CSR[(csr_base + 11) as usize]),
                'K' => Self::create_card(ALL_CSR[(csr_base + 12) as usize]),
                _ => None,
            }
        }
    }

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        fn empty() -> Self {
            CardCollection { prev: None, next: None, c: None }
        }

        fn to_vec(&self) -> Vec<Card> {
            let mut out = Vec::new();
            let mut node: Option<&CardCollection> = Some(self);
            while let Some(n) = node {
                if let Some(card) = n.c {
                    out.push(card);
                }
                node = n.next.as_deref();
            }
            out
        }

        fn build_from_vec(cards: &[Card]) -> Self {
            if cards.is_empty() {
                return Self::empty();
            }
            let mut tail: Option<Box<CardCollection>> = None;
            for i in (1..cards.len()).rev() {
                let node = Box::new(CardCollection {
                    prev: None,
                    next: tail,
                    c: Some(cards[i]),
                });
                tail = Some(node);
            }
            CardCollection {
                prev: None,
                next: tail,
                c: Some(cards[0]),
            }
        }

        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self {
            let cards = self.to_vec();
            let new_card = match c {
                Some(card) => card,
                None => return Self::build_from_vec(&cards),
            };

            let len = cards.len();
            let mut insert_pos = len;
            for i in 0..=len {
                let before = if i == 0 { None } else { Some(cards[i - 1]) };
                let after = if i == len { None } else { Some(cards[i]) };
                if sorter(&before, &c, &after) != 0 {
                    insert_pos = i;
                    break;
                }
            }

            let mut new_cards = Vec::with_capacity(len + 1);
            new_cards.extend_from_slice(&cards[..insert_pos]);
            new_cards.push(new_card);
            new_cards.extend_from_slice(&cards[insert_pos..]);

            Self::build_from_vec(&new_cards)
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, new: Self) -> Self {
            let mut cards = self.to_vec();
            cards.extend(new.to_vec());
            Self::build_from_vec(&cards)
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // The Box-based doubly linked structure cannot be mutated by an external
            // entry pointer in safe Rust; this is intentionally a no-op as the
            // higher-level operations rebuild the collection when needed.
        }
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            let s: CardSorter = if (sorter as usize) == 0 {
                sort_card_after
            } else {
                sorter
            };
            Some(CardHand {
                max,
                len: 0,
                sorter: s,
                cards: CardCollection::empty(),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection::empty();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            if c.is_none() {
                return;
            }
            let old = std::mem::replace(&mut self.cards, CardCollection::empty());
            self.cards = old.insert_into_collection(*c, self.sorter);
            self.len += 1;
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 {
                return CardRank::InvalidRank;
            }
            let mut max_rank: Option<CardRank> = None;
            let mut node: Option<&CardCollection> = Some(&self.cards);
            while let Some(n) = node {
                if let Some(card) = n.c {
                    let r = card.get_card_rank();
                    match max_rank {
                        None => max_rank = Some(r),
                        Some(mr) => {
                            if (r as u32) > (mr as u32) {
                                max_rank = Some(r);
                            }
                        }
                    }
                }
                node = n.next.as_deref();
            }
            max_rank.unwrap_or(CardRank::InvalidRank)
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let cards = self.cards.to_vec();
            let mut to_keep: Vec<Card> = Vec::with_capacity(cards.len());
            let mut pos: u64 = 0;
            let mut current_len: u64 = self.len as u64;
            let mut is_stopped = false;
            let mut i = 0usize;

            while i < cards.len() && !is_stopped {
                let action = itr_fn(current_len, pos, &Some(cards[i]));
                match action {
                    ItrAction::Continue => {
                        to_keep.push(cards[i]);
                        pos += 1;
                    }
                    ItrAction::Break => {
                        to_keep.push(cards[i]);
                        is_stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        if current_len > 0 {
                            current_len -= 1;
                        }
                        // pos stays the same (in C: pos--, then pos++ at end of loop)
                    }
                    ItrAction::RemoveAndBreak => {
                        if current_len > 0 {
                            current_len -= 1;
                        }
                        is_stopped = true;
                    }
                }
                i += 1;
            }

            // Keep any remaining cards untouched if loop was stopped.
            while i < cards.len() {
                to_keep.push(cards[i]);
                i += 1;
            }

            self.cards = CardCollection::build_from_vec(&to_keep);
            self.len = to_keep.len() as u8;
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let cards = self.cards.to_vec();
            let kept: Vec<Card> = cards
                .into_iter()
                .filter(|card| card.get_card_suit_rank() != c)
                .collect();
            self.len = kept.len() as u8;
            self.cards = CardCollection::build_from_vec(&kept);
        }

        pub fn remove_from_hand_under_iter(
            &mut self,
            _CardCollection: &CardCollection,
            _pos: usize,
        ) {
            // The iterate_hand function performs in-place removal via the Vec
            // approach; this entry point is preserved for API compatibility but
            // is not used directly here.
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            if self.cards[c as usize].get_card_suit_rank() == CardSuitRank::InvalidCard {
                1
            } else {
                0
            }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let selected = (lrand48() % (self.card_count as u64)) as u32;
            let mut valid_idx: u32 = 0;
            for i in 0..(CardSuitRank::CardCount as usize) {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        let csr = ALL_CSR[i];
                        self.cards[i] = Card::write_card(csr);
                        self.card_count -= 1;
                        return Some(self.cards[i]);
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= (CardSuitRank::CardCount as usize) {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                if self.card_count > 0 {
                    self.card_count -= 1;
                }
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards: [Card { card: 0 }; CardSuitRank::CardCount as usize],
            })
        }
    }

    pub type CardSorter =
        fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(
        _before: &Option<Card>,
        _new: &Option<Card>,
        after: &Option<Card>,
    ) -> i32 {
        if after.is_none() {
            1
        } else {
            0
        }
    }

    pub fn sort_card_by_rank(
        before: &Option<Card>,
        new: &Option<Card>,
        after: &Option<Card>,
    ) -> i32 {
        let r = match new {
            Some(c) => c.get_card_rank(),
            None => return 0,
        };

        if after.is_none() {
            return 1;
        }
        let after_rank = after.as_ref().unwrap().get_card_rank();

        let before_ok = match before {
            None => true,
            Some(c) => (r as u32) > (c.get_card_rank() as u32),
        };

        if before_ok && (r as u32) <= (after_rank as u32) {
            1
        } else {
            0
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;

    // ---------------------------------------------------------------------
    // Internal pseudo-random number generator (glibc-style lrand48).
    // ---------------------------------------------------------------------
    static LRAND_STATE: AtomicU64 = AtomicU64::new(0);

    fn lrand48() -> u64 {
        // Standard glibc lrand48 LCG: X_{n+1} = (a * X_n + c) mod 2^48
        // where a = 0x5DEECE66D and c = 0xB.
        let prev = LRAND_STATE.load(Ordering::Relaxed);
        let next =
            prev.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & 0x0000_FFFF_FFFF_FFFF;
        LRAND_STATE.store(next, Ordering::Relaxed);
        // The high 31 bits are returned as the random output.
        next >> 17
    }

    // Suppress dead_code warnings for the atomic when not used elsewhere.
    #[allow(dead_code)]
    static _ATOMIC_PROBE: AtomicU32 = AtomicU32::new(0);
}
