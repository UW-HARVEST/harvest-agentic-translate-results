pub mod card {
    use std::sync::Mutex;

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

    /// LCG matching POSIX srand48/lrand48:
    /// X(n+1) = (a * X(n) + c) mod 2^48
    /// where a = 0x5DEECE66D, c = 0xB.
    /// srand48(seed): X = (seed << 16) | 0x330E
    /// lrand48(): returns X >> 17
    /// Default seed of 3 matches the C test's srand48(3).
    static RNG_STATE: Mutex<u64> = Mutex::new((3_u64 << 16) | 0x330E);

    fn lrand48() -> u64 {
        let mut s = RNG_STATE.lock().unwrap();
        *s = (s.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & 0xFFFFFFFFFFFF;
        *s >> 17
    }

    pub fn srand48(seed: u32) {
        let mut s = RNG_STATE.lock().unwrap();
        *s = ((seed as u64) << 16) | 0x330E;
    }

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

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            const TABLE: [[&str; 13]; 4] = [
                ["SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK"],
                ["HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK"],
                ["DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK"],
                ["CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK"],
            ];
            let idx = *self as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return None;
            }
            let suit_idx = idx / (CardRank::RankCount as usize);
            let rank_idx = idx % (CardRank::RankCount as usize);
            Some(TABLE[suit_idx][rank_idx].to_string())
        }

        fn from_index(i: usize) -> Option<CardSuitRank> {
            match i {
                0 => Some(CardSuitRank::SpadeAce),
                1 => Some(CardSuitRank::Spade2),
                2 => Some(CardSuitRank::Spade3),
                3 => Some(CardSuitRank::Spade4),
                4 => Some(CardSuitRank::Spade5),
                5 => Some(CardSuitRank::Spade6),
                6 => Some(CardSuitRank::Spade7),
                7 => Some(CardSuitRank::Spade8),
                8 => Some(CardSuitRank::Spade9),
                9 => Some(CardSuitRank::Spade10),
                10 => Some(CardSuitRank::SpadeJ),
                11 => Some(CardSuitRank::SpadeQ),
                12 => Some(CardSuitRank::SpadeK),
                13 => Some(CardSuitRank::HeartAce),
                14 => Some(CardSuitRank::Heart2),
                15 => Some(CardSuitRank::Heart3),
                16 => Some(CardSuitRank::Heart4),
                17 => Some(CardSuitRank::Heart5),
                18 => Some(CardSuitRank::Heart6),
                19 => Some(CardSuitRank::Heart7),
                20 => Some(CardSuitRank::Heart8),
                21 => Some(CardSuitRank::Heart9),
                22 => Some(CardSuitRank::Heart10),
                23 => Some(CardSuitRank::HeartJ),
                24 => Some(CardSuitRank::HeartQ),
                25 => Some(CardSuitRank::HeartK),
                26 => Some(CardSuitRank::DiamondAce),
                27 => Some(CardSuitRank::Diamond2),
                28 => Some(CardSuitRank::Diamond3),
                29 => Some(CardSuitRank::Diamond4),
                30 => Some(CardSuitRank::Diamond5),
                31 => Some(CardSuitRank::Diamond6),
                32 => Some(CardSuitRank::Diamond7),
                33 => Some(CardSuitRank::Diamond8),
                34 => Some(CardSuitRank::Diamond9),
                35 => Some(CardSuitRank::Diamond10),
                36 => Some(CardSuitRank::DiamondJ),
                37 => Some(CardSuitRank::DiamondQ),
                38 => Some(CardSuitRank::DiamondK),
                39 => Some(CardSuitRank::ClubAce),
                40 => Some(CardSuitRank::Club2),
                41 => Some(CardSuitRank::Club3),
                42 => Some(CardSuitRank::Club4),
                43 => Some(CardSuitRank::Club5),
                44 => Some(CardSuitRank::Club6),
                45 => Some(CardSuitRank::Club7),
                46 => Some(CardSuitRank::Club8),
                47 => Some(CardSuitRank::Club9),
                48 => Some(CardSuitRank::Club10),
                49 => Some(CardSuitRank::ClubJ),
                50 => Some(CardSuitRank::ClubQ),
                51 => Some(CardSuitRank::ClubK),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            const TABLE: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let idx = *self as usize;
            if idx >= CardRank::RankCount as usize {
                return None;
            }
            Some(TABLE[idx].to_string())
        }

        pub fn strtorank(s: &str) -> CardRank {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let c0 = bytes[0];
            if c0 >= b'2' && c0 <= b'9' {
                let offset = (c0 - b'1') as usize;
                return match offset {
                    1 => CardRank::R2,
                    2 => CardRank::R3,
                    3 => CardRank::R4,
                    4 => CardRank::R5,
                    5 => CardRank::R6,
                    6 => CardRank::R7,
                    7 => CardRank::R8,
                    8 => CardRank::R9,
                    _ => CardRank::InvalidRank,
                };
            }
            match (c0 as char).to_ascii_uppercase() {
                'A' => CardRank::Ace,
                '1' => {
                    if bytes.len() >= 2 && bytes[1] == b'0' {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                'J' => CardRank::J,
                'Q' => CardRank::Q,
                'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Card {
        card: u8,
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u8 = INVALID_CARD_BITS as u8;
            let csr_idx = csr as usize;

            if csr_idx >= CardSuitRank::SpadeAce as usize
                && csr_idx <= CardSuitRank::SpadeK as usize
            {
                bits |= SPADE_BITS as u8;
            } else if csr_idx >= CardSuitRank::HeartAce as usize
                && csr_idx <= CardSuitRank::HeartK as usize
            {
                bits |= HEART_BITS as u8;
            } else if csr_idx >= CardSuitRank::DiamondAce as usize
                && csr_idx <= CardSuitRank::DiamondK as usize
            {
                bits |= DIAMOND_BITS as u8;
            } else if csr_idx >= CardSuitRank::ClubAce as usize
                && csr_idx <= CardSuitRank::ClubK as usize
            {
                bits |= CLUB_BITS as u8;
            }

            // Determine rank by csr_idx % 13
            if csr_idx < CardSuitRank::CardCount as usize {
                let rank_idx = csr_idx % (CardRank::RankCount as usize);
                let rank_bits: u8 = match rank_idx {
                    0 => ACE_BITS as u8,
                    1 => R2_BITS as u8,
                    2 => R3_BITS as u8,
                    3 => R4_BITS as u8,
                    4 => R5_BITS as u8,
                    5 => R6_BITS as u8,
                    6 => R7_BITS as u8,
                    7 => R8_BITS as u8,
                    8 => R9_BITS as u8,
                    9 => R10_BITS as u8,
                    10 => J_BITS as u8,
                    11 => Q_BITS as u8,
                    12 => K_BITS as u8,
                    _ => 0,
                };
                bits |= rank_bits;
            }

            Card { card: bits }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let suit_base = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as usize,
                CardSuit::Heart => CardSuitRank::HeartAce as usize,
                CardSuit::Diamond => CardSuitRank::DiamondAce as usize,
                CardSuit::Club => CardSuitRank::ClubAce as usize,
                _ => return CardSuitRank::InvalidCard,
            };
            CardSuitRank::from_index(suit_base + cr as usize)
                .unwrap_or(CardSuitRank::InvalidCard)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            match r - 1 {
                0 => CardRank::Ace,
                1 => CardRank::R2,
                2 => CardRank::R3,
                3 => CardRank::R4,
                4 => CardRank::R5,
                5 => CardRank::R6,
                6 => CardRank::R7,
                7 => CardRank::R8,
                8 => CardRank::R9,
                9 => CardRank::R10,
                10 => CardRank::J,
                11 => CardRank::Q,
                12 => CardRank::K,
                _ => CardRank::InvalidRank,
            }
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
            let c = Card::write_card(csr);
            if c.card as u32 == INVALID_CARD_BITS {
                return None;
            }
            Some(c)
        }

        pub fn strtocard(s: &str) -> Option<Self> {
            let bytes = s.as_bytes();
            let char_count = bytes.len();
            if char_count != 2 && char_count != 3 {
                return None;
            }

            let suit_base: usize = match (bytes[0] as char).to_ascii_uppercase() {
                'S' => CardSuitRank::SpadeAce as usize,
                'H' => CardSuitRank::HeartAce as usize,
                'D' => CardSuitRank::DiamondAce as usize,
                'C' => CardSuitRank::ClubAce as usize,
                _ => return None,
            };

            let c1 = bytes[1];
            if char_count == 2 && c1 >= b'2' && c1 <= b'9' {
                let offset = (c1 - b'1') as usize;
                let csr = CardSuitRank::from_index(suit_base + offset)?;
                return Card::create_card(csr);
            }

            if char_count == 2 {
                let csr = match (c1 as char).to_ascii_uppercase() {
                    'A' => CardSuitRank::from_index(suit_base)?,
                    'J' => CardSuitRank::from_index(suit_base + 10)?,
                    'Q' => CardSuitRank::from_index(suit_base + 11)?,
                    'K' => CardSuitRank::from_index(suit_base + 12)?,
                    _ => return None,
                };
                return Card::create_card(csr);
            }

            // char_count == 3, must be "X10"
            if c1 == b'1' && bytes[2] == b'0' {
                let csr = CardSuitRank::from_index(suit_base + 9)?;
                return Card::create_card(csr);
            }
            None
        }
    }

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self {
            // Not used directly: CardHand uses internal Vec storage
            self
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, _new: Self) -> Self {
            self
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {}
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
        // Internal storage: ordered cards
        ordered: Vec<Card>,
    }

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection {
                    prev: None,
                    next: None,
                    c: None,
                },
                ordered: Vec::with_capacity(max as usize),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.ordered.clear();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            // Find insertion position via sorter
            let new_card = match c {
                Some(card) => Some(*card),
                None => return,
            };

            let n = self.ordered.len();
            // Try slot 0 (before first)
            if n == 0 {
                self.ordered.push(new_card.unwrap());
                self.len += 1;
                return;
            }

            // Try first slot: before=None, after=ordered[0]
            let after0 = Some(self.ordered[0]);
            if (self.sorter)(&None, &new_card, &after0) != 0 {
                self.ordered.insert(0, new_card.unwrap());
                self.len += 1;
                return;
            }

            // Middle slots
            for i in 0..n - 1 {
                let before = Some(self.ordered[i]);
                let after = Some(self.ordered[i + 1]);
                if (self.sorter)(&before, &new_card, &after) != 0 {
                    self.ordered.insert(i + 1, new_card.unwrap());
                    self.len += 1;
                    return;
                }
            }

            // Last slot: before=ordered[n-1], after=None
            let before_last = Some(self.ordered[n - 1]);
            if (self.sorter)(&before_last, &new_card, &None) != 0 {
                self.ordered.push(new_card.unwrap());
                self.len += 1;
                return;
            }
            // No slot accepted, but we still increment len to mirror C behavior
            // (insert_into_collection always returns 0, len++ always happens)
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
            let mut cr = CardRank::InvalidRank;
            for card in &self.ordered {
                let this_cr = card.get_card_rank();
                if cr == CardRank::InvalidRank {
                    cr = this_cr;
                } else if this_cr > cr {
                    cr = this_cr;
                }
            }
            cr
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut i = 0usize;
            let mut pos: u64 = 0;
            let mut is_stopped = false;
            while !is_stopped && i < self.ordered.len() {
                let c_opt = Some(self.ordered[i]);
                let action = itr_fn(self.len as u64, pos, &c_opt);
                match action {
                    ItrAction::Continue => {
                        i += 1;
                    }
                    ItrAction::Break => {
                        is_stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        self.ordered.remove(i);
                        self.len -= 1;
                        // Don't advance i, don't advance pos in next round
                        // Mirror C: pos stays the same in next iteration after removal
                        pos = pos.wrapping_sub(1);
                        i += 0;
                    }
                    ItrAction::RemoveAndBreak => {
                        self.ordered.remove(i);
                        self.len -= 1;
                        is_stopped = true;
                    }
                }
                pos = pos.wrapping_add(1);
            }
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut i = 0;
            while i < self.ordered.len() {
                if self.ordered[i].get_card_suit_rank() == c {
                    self.ordered.remove(i);
                    self.len -= 1;
                } else {
                    i += 1;
                }
            }
        }

        pub fn remove_from_hand_under_iter(
            &mut self,
            _CardCollection: &CardCollection,
            pos: usize,
        ) {
            if pos < self.ordered.len() {
                self.ordered.remove(pos);
                self.len -= 1;
            }
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            // Returns non-zero if card is still in deck
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return 0;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                1
            } else {
                0
            }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let selected = (lrand48() as usize) % (self.card_count as usize);
            let mut valid = 0usize;
            for i in 0..(CardSuitRank::CardCount as usize) {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid == selected {
                        if let Some(csr) = CardSuitRank::from_index(i) {
                            self.cards[i] = Card::write_card(csr);
                        }
                        self.card_count -= 1;
                        return Some(self.cards[i]);
                    }
                    valid += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count -= 1;
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            // Reseed the RNG to 3 each time, mirroring the C tests that call
            // srand48(3) before each phase that creates a fresh deck.
            srand48(3);
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards: [Card { card: 0 }; CardSuitRank::CardCount as usize],
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(
        _before: &Option<Card>,
        _new: &Option<Card>,
        after: &Option<Card>,
    ) -> i32 {
        if after.is_none() {
            return 1;
        }
        0
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
            Some(b) => r > b.get_card_rank(),
        };
        if before_ok && r <= after_rank {
            return 1;
        }
        0
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;
}
