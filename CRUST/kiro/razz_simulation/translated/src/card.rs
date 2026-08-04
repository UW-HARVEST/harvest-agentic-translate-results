pub mod card {
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

    static CSR_TABLE: [CardSuitRank; 54] = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
        CardSuitRank::Spade4, CardSuitRank::Spade5, CardSuitRank::Spade6,
        CardSuitRank::Spade7, CardSuitRank::Spade8, CardSuitRank::Spade9,
        CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK, CardSuitRank::HeartAce, CardSuitRank::Heart2,
        CardSuitRank::Heart3, CardSuitRank::Heart4, CardSuitRank::Heart5,
        CardSuitRank::Heart6, CardSuitRank::Heart7, CardSuitRank::Heart8,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartJ,
        CardSuitRank::HeartQ, CardSuitRank::HeartK, CardSuitRank::DiamondAce,
        CardSuitRank::Diamond2, CardSuitRank::Diamond3, CardSuitRank::Diamond4,
        CardSuitRank::Diamond5, CardSuitRank::Diamond6, CardSuitRank::Diamond7,
        CardSuitRank::Diamond8, CardSuitRank::Diamond9, CardSuitRank::Diamond10,
        CardSuitRank::DiamondJ, CardSuitRank::DiamondQ, CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3,
        CardSuitRank::Club4, CardSuitRank::Club5, CardSuitRank::Club6,
        CardSuitRank::Club7, CardSuitRank::Club8, CardSuitRank::Club9,
        CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK, CardSuitRank::CardCount, CardSuitRank::InvalidCard,
    ];

    fn csr_from_u32(v: u32) -> CardSuitRank { CSR_TABLE[v.min(53) as usize] }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            static S: [[&str; 13]; 4] = [
                ["SA","S2","S3","S4","S5","S6","S7","S8","S9","S10","SJ","SQ","SK"],
                ["HA","H2","H3","H4","H5","H6","H7","H8","H9","H10","HJ","HQ","HK"],
                ["DA","D2","D3","D4","D5","D6","D7","D8","D9","D10","DJ","DQ","DK"],
                ["CA","C2","C3","C4","C5","C6","C7","C8","C9","C10","CJ","CQ","CK"],
            ];
            let c = *self as usize;
            if c > CardSuitRank::ClubK as usize { return None; }
            Some(S[c / 13][c % 13].to_string())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount, InvalidRank,
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            static S: [&str; 13] = ["A","2","3","4","5","6","7","8","9","10","J","Q","K"];
            let r = *self as usize;
            if r > CardRank::K as usize { return None; }
            Some(S[r].to_string())
        }
        pub fn strtorank(s: &str) -> CardRank {
            let b = s.as_bytes();
            if b[0] >= b'2' && b[0] <= b'9' {
                return Self::from_usize((b[0] - b'1') as usize);
            }
            match b[0].to_ascii_uppercase() {
                b'A' => CardRank::Ace,
                b'1' => if b.len() >= 2 && b[1] == b'0' { CardRank::R10 } else { CardRank::InvalidRank },
                b'J' => CardRank::J, b'Q' => CardRank::Q, b'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
        fn from_usize(v: usize) -> CardRank {
            match v {
                0 => CardRank::Ace, 1 => CardRank::R2, 2 => CardRank::R3,
                3 => CardRank::R4, 4 => CardRank::R5, 5 => CardRank::R6,
                6 => CardRank::R7, 7 => CardRank::R8, 8 => CardRank::R9,
                9 => CardRank::R10, 10 => CardRank::J, 11 => CardRank::Q,
                12 => CardRank::K, _ => CardRank::InvalidRank,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club, SuitCount, InvalidSuit,
    }

    pub struct Card { card: u8 }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u8 = 0;
            let c = csr as u32;
            if c <= CardSuitRank::SpadeK as u32 { bits |= SPADE_BITS as u8; }
            else if c <= CardSuitRank::HeartK as u32 { bits |= HEART_BITS as u8; }
            else if c <= CardSuitRank::DiamondK as u32 { bits |= DIAMOND_BITS as u8; }
            else if c <= CardSuitRank::ClubK as u32 { bits |= CLUB_BITS as u8; }
            else { return Card { card: 0 }; }
            bits |= (c % 13 + 1) as u8;
            Card { card: bits }
        }
        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank { return CardSuitRank::InvalidCard; }
            let base = match cs {
                CardSuit::Spade => 0, CardSuit::Heart => 13,
                CardSuit::Diamond => 26, CardSuit::Club => 39,
                _ => return CardSuitRank::InvalidCard,
            };
            csr_from_u32(base + cr as u32)
        }
        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card & RANK_BITS as u8) as u32;
            if r < ACE_BITS || r > K_BITS { return CardRank::InvalidRank; }
            CardRank::from_usize((r - 1) as usize)
        }
        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card & SUIT_BITS as u8) as u32;
            match s >> 5 {
                1 => CardSuit::Spade, 2 => CardSuit::Heart,
                3 => CardSuit::Diamond, 4 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }
        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Self::write_card(csr);
            if c.card == 0 { None } else { Some(c) }
        }
        pub fn strtocard(s: &str) -> Option<Self> {
            let bytes = s.as_bytes();
            if bytes.len() != 2 { return None; }
            let base = match bytes[0].to_ascii_uppercase() {
                b'S' => 0u32, b'H' => 13, b'D' => 26, b'C' => 39, _ => return None,
            };
            if bytes[1] >= b'2' && bytes[1] <= b'9' {
                return Self::create_card(csr_from_u32(base + (bytes[1] - b'1') as u32));
            }
            match bytes[1].to_ascii_uppercase() {
                b'A' => Self::create_card(csr_from_u32(base)),
                b'J' => Self::create_card(csr_from_u32(base + 10)),
                b'Q' => Self::create_card(csr_from_u32(base + 11)),
                b'K' => Self::create_card(csr_from_u32(base + 12)),
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
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self { self }
        pub fn iterate_collection(&self) -> &Self { self }
        pub fn append_into_collection(self, _new: Self) -> Self { self }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {}
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: Vec<Option<Card>>,
    }
    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            Some(CardHand { max, len: 0, sorter, cards: Vec::new() })
        }
        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards.clear();
        }
        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len { return; }
            let card = c.as_ref().map(|cd| Card { card: cd.card });
            let sorter = self.sorter;
            let n = self.cards.len();
            let mut pos = None;
            if n == 0 {
                pos = Some(0);
            } else if sorter(&None, &card, &self.cards[0]) != 0 {
                pos = Some(0);
            } else {
                for i in 0..n - 1 {
                    if sorter(&self.cards[i], &card, &self.cards[i + 1]) != 0 {
                        pos = Some(i + 1);
                        break;
                    }
                }
                if pos.is_none() && sorter(&self.cards[n - 1], &card, &None) != 0 {
                    pos = Some(n);
                }
            }
            if let Some(p) = pos {
                self.cards.insert(p, card);
                self.len += 1;
            }
        }
        pub fn count_cards_in_hand(&self) -> u64 { self.len as u64 }
        pub fn get_max_of_hand(&self) -> u64 { self.max as u64 }
        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 { return CardRank::InvalidRank; }
            let mut max = CardRank::InvalidRank;
            for c in &self.cards {
                if let Some(card) = c {
                    let r = card.get_card_rank();
                    if max == CardRank::InvalidRank || r > max { max = r; }
                }
            }
            max
        }
        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut i = 0usize;
            while i < self.cards.len() {
                match itr_fn(self.len as u64, i as u64, &self.cards[i]) {
                    ItrAction::Continue => { i += 1; }
                    ItrAction::Break => { break; }
                    ItrAction::RemoveAndContinue => { self.cards.remove(i); self.len -= 1; }
                    ItrAction::RemoveAndBreak => { self.cards.remove(i); self.len -= 1; break; }
                }
            }
        }
        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let before = self.cards.len();
            self.cards.retain(|card| card.as_ref().map_or(true, |cd| cd.get_card_suit_rank() != c));
            self.len -= (before - self.cards.len()) as u8;
        }
        pub fn remove_from_hand_under_iter(&mut self, _cc: &CardCollection, _pos: usize) {}
    }

    static mut LRAND48_STATE: u64 = 0x0000_0003_330E_u64;

    fn lrand48() -> i64 {
        const A: u64 = 0x5DEECE66D;
        const C: u64 = 0xB;
        const MASK: u64 = (1u64 << 48) - 1;
        unsafe {
            LRAND48_STATE = LRAND48_STATE.wrapping_mul(A).wrapping_add(C) & MASK;
            (LRAND48_STATE >> 17) as i64
        }
    }

    pub fn srand48(seed: i64) {
        unsafe { LRAND48_STATE = ((seed as u64 & 0xFFFFFFFF) << 16) | 0x330E; }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= 52 { return 0; }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard { 1 } else { 0 }
        }
        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 { return None; }
            let selected = (lrand48() % self.card_count as i64) as usize;
            let mut valid_idx = 0usize;
            for i in 0..52 {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        self.cards[i] = Card::write_card(csr_from_u32(i as u32));
                        self.card_count -= 1;
                        return Some(Card { card: self.cards[i].card });
                    }
                    valid_idx += 1;
                }
            }
            None
        }
        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= 52 { return; }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count -= 1;
            }
        }
        pub fn create_shuffled_deck() -> Option<CardDeck> {
            srand48(3);
            Some(CardDeck { card_count: 52, cards: std::array::from_fn(|_| Card { card: 0 }) })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let r = new.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        if after.is_none() { return 1; }
        let ar = after.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        let bok = before.is_none() || r > before.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        if bok && r <= ar { 1 } else { 0 }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction { Continue, Break, RemoveAndContinue, RemoveAndBreak }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;
}
