pub mod card {
    use rand::Rng;

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

    fn csr_from_usize(i: usize) -> CardSuitRank {
        match i {
            0 => CardSuitRank::SpadeAce,
            1 => CardSuitRank::Spade2,
            2 => CardSuitRank::Spade3,
            3 => CardSuitRank::Spade4,
            4 => CardSuitRank::Spade5,
            5 => CardSuitRank::Spade6,
            6 => CardSuitRank::Spade7,
            7 => CardSuitRank::Spade8,
            8 => CardSuitRank::Spade9,
            9 => CardSuitRank::Spade10,
            10 => CardSuitRank::SpadeJ,
            11 => CardSuitRank::SpadeQ,
            12 => CardSuitRank::SpadeK,
            13 => CardSuitRank::HeartAce,
            14 => CardSuitRank::Heart2,
            15 => CardSuitRank::Heart3,
            16 => CardSuitRank::Heart4,
            17 => CardSuitRank::Heart5,
            18 => CardSuitRank::Heart6,
            19 => CardSuitRank::Heart7,
            20 => CardSuitRank::Heart8,
            21 => CardSuitRank::Heart9,
            22 => CardSuitRank::Heart10,
            23 => CardSuitRank::HeartJ,
            24 => CardSuitRank::HeartQ,
            25 => CardSuitRank::HeartK,
            26 => CardSuitRank::DiamondAce,
            27 => CardSuitRank::Diamond2,
            28 => CardSuitRank::Diamond3,
            29 => CardSuitRank::Diamond4,
            30 => CardSuitRank::Diamond5,
            31 => CardSuitRank::Diamond6,
            32 => CardSuitRank::Diamond7,
            33 => CardSuitRank::Diamond8,
            34 => CardSuitRank::Diamond9,
            35 => CardSuitRank::Diamond10,
            36 => CardSuitRank::DiamondJ,
            37 => CardSuitRank::DiamondQ,
            38 => CardSuitRank::DiamondK,
            39 => CardSuitRank::ClubAce,
            40 => CardSuitRank::Club2,
            41 => CardSuitRank::Club3,
            42 => CardSuitRank::Club4,
            43 => CardSuitRank::Club5,
            44 => CardSuitRank::Club6,
            45 => CardSuitRank::Club7,
            46 => CardSuitRank::Club8,
            47 => CardSuitRank::Club9,
            48 => CardSuitRank::Club10,
            49 => CardSuitRank::ClubJ,
            50 => CardSuitRank::ClubQ,
            51 => CardSuitRank::ClubK,
            52 => CardSuitRank::CardCount,
            _ => CardSuitRank::InvalidCard,
        }
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) ->  Option<String> {
            const S: [[&str; 13]; 4] = [
                ["SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK"],
                ["HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK"],
                ["DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK"],
                ["CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK"],
            ];
            let i = *self as usize;
            if i >= CardSuitRank::CardCount as usize {
                return None;
            }
            Some(S[i / 13][i % 13].to_string())
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
            const S: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let i = *self as usize;
            if i >= CardRank::RankCount as usize {
                return None;
            }
            Some(S[i].to_string())
        }
        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let c0 = bytes[0];
            let cr;
            if c0 >= b'2' && c0 <= b'9' {
                let offset = (c0 - b'1') as usize;
                cr = match offset {
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
            } else {
                let upper = (c0 as char).to_ascii_uppercase();
                cr = match upper {
                    'A' => CardRank::Ace,
                    '1' => {
                        if bytes.len() >= 2 && bytes[1] == b'0' {
                            CardRank::R10
                        } else {
                            return CardRank::InvalidRank;
                        }
                    }
                    'J' => CardRank::J,
                    'Q' => CardRank::Q,
                    'K' => CardRank::K,
                    _ => return CardRank::InvalidRank,
                };
            }
            cr
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
        card: u8
    }
    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut byte: u8 = INVALID_CARD_BITS as u8;
            let csr_i = csr as usize;
            // Suit
            if csr_i >= CardSuitRank::SpadeAce as usize && csr_i <= CardSuitRank::SpadeK as usize {
                byte |= SPADE_BITS as u8;
            } else if csr_i >= CardSuitRank::HeartAce as usize && csr_i <= CardSuitRank::HeartK as usize {
                byte |= HEART_BITS as u8;
            } else if csr_i >= CardSuitRank::DiamondAce as usize && csr_i <= CardSuitRank::DiamondK as usize {
                byte |= DIAMOND_BITS as u8;
            } else if csr_i >= CardSuitRank::ClubAce as usize && csr_i <= CardSuitRank::ClubK as usize {
                byte |= CLUB_BITS as u8;
            }
            // Rank
            match csr {
                CardSuitRank::SpadeAce | CardSuitRank::HeartAce
                | CardSuitRank::DiamondAce | CardSuitRank::ClubAce => byte |= ACE_BITS as u8,
                CardSuitRank::Spade2 | CardSuitRank::Heart2
                | CardSuitRank::Diamond2 | CardSuitRank::Club2 => byte |= R2_BITS as u8,
                CardSuitRank::Spade3 | CardSuitRank::Heart3
                | CardSuitRank::Diamond3 | CardSuitRank::Club3 => byte |= R3_BITS as u8,
                CardSuitRank::Spade4 | CardSuitRank::Heart4
                | CardSuitRank::Diamond4 | CardSuitRank::Club4 => byte |= R4_BITS as u8,
                CardSuitRank::Spade5 | CardSuitRank::Heart5
                | CardSuitRank::Diamond5 | CardSuitRank::Club5 => byte |= R5_BITS as u8,
                CardSuitRank::Spade6 | CardSuitRank::Heart6
                | CardSuitRank::Diamond6 | CardSuitRank::Club6 => byte |= R6_BITS as u8,
                CardSuitRank::Spade7 | CardSuitRank::Heart7
                | CardSuitRank::Diamond7 | CardSuitRank::Club7 => byte |= R7_BITS as u8,
                CardSuitRank::Spade8 | CardSuitRank::Heart8
                | CardSuitRank::Diamond8 | CardSuitRank::Club8 => byte |= R8_BITS as u8,
                CardSuitRank::Spade9 | CardSuitRank::Heart9
                | CardSuitRank::Diamond9 | CardSuitRank::Club9 => byte |= R9_BITS as u8,
                CardSuitRank::Spade10 | CardSuitRank::Heart10
                | CardSuitRank::Diamond10 | CardSuitRank::Club10 => byte |= R10_BITS as u8,
                CardSuitRank::SpadeJ | CardSuitRank::HeartJ
                | CardSuitRank::DiamondJ | CardSuitRank::ClubJ => byte |= J_BITS as u8,
                CardSuitRank::SpadeQ | CardSuitRank::HeartQ
                | CardSuitRank::DiamondQ | CardSuitRank::ClubQ => byte |= Q_BITS as u8,
                CardSuitRank::SpadeK | CardSuitRank::HeartK
                | CardSuitRank::DiamondK | CardSuitRank::ClubK => byte |= K_BITS as u8,
                _ => {}
            }
            Card { card: byte }
        }
        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let base: usize = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as usize,
                CardSuit::Heart => CardSuitRank::HeartAce as usize,
                CardSuit::Diamond => CardSuitRank::DiamondAce as usize,
                CardSuit::Club => CardSuitRank::ClubAce as usize,
                _ => return CardSuitRank::InvalidCard,
            };
            csr_from_usize(base + cr as usize)
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
        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            if bytes.len() != 2 {
                return None;
            }
            let c0_upper = (bytes[0] as char).to_ascii_uppercase();
            let csr_base: usize = match c0_upper {
                'S' => CardSuitRank::SpadeAce as usize,
                'H' => CardSuitRank::HeartAce as usize,
                'D' => CardSuitRank::DiamondAce as usize,
                'C' => CardSuitRank::ClubAce as usize,
                _ => return None,
            };
            let c1 = bytes[1];
            if c1 >= b'2' && c1 <= b'9' {
                let offset = (c1 - b'1') as usize;
                return Card::create_card(csr_from_usize(csr_base + offset));
            }
            let c1_upper = (c1 as char).to_ascii_uppercase();
            match c1_upper {
                'A' => Card::create_card(csr_from_usize(csr_base)),
                '1' => None, // length must be 2, '10' would need length 3
                'J' => Card::create_card(csr_from_usize(csr_base + 10)),
                'Q' => Card::create_card(csr_from_usize(csr_base + 11)),
                'K' => Card::create_card(csr_from_usize(csr_base + 12)),
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
        pub fn new_empty() -> Self {
            CardCollection { prev: None, next: None, c: None }
        }

        // Helper: serialize chain to vec of cards (head -> tail).
        pub fn to_vec(&self) -> Vec<Card> {
            let mut result = Vec::new();
            if let Some(card) = self.c {
                result.push(card);
                let mut cur = &self.next;
                while let Some(node) = cur {
                    if let Some(card) = node.c {
                        result.push(card);
                    }
                    cur = &node.next;
                }
            }
            result
        }

        // Helper: build a chain from a vec of cards.
        pub fn from_vec(cards: Vec<Card>) -> Self {
            if cards.is_empty() {
                return Self::new_empty();
            }
            let mut next: Option<Box<CardCollection>> = None;
            // Iterate from tail to second card
            for i in (1..cards.len()).rev() {
                let node = CardCollection {
                    prev: None,
                    next,
                    c: Some(cards[i]),
                };
                next = Some(Box::new(node));
            }
            CardCollection {
                prev: None,
                next,
                c: Some(cards[0]),
            }
        }

        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self {
            let card = match c {
                Some(x) => x,
                None => return self,
            };
            let mut cards = self.to_vec();
            let new_card = Some(card);
            let n = cards.len();
            if n == 0 {
                cards.push(card);
            } else if sorter(&None, &new_card, &Some(cards[0])) != 0 {
                cards.insert(0, card);
            } else {
                let mut inserted = false;
                for i in 0..(n.saturating_sub(1)) {
                    if sorter(&Some(cards[i]), &new_card, &Some(cards[i + 1])) != 0 {
                        cards.insert(i + 1, card);
                        inserted = true;
                        break;
                    }
                }
                if !inserted && sorter(&Some(cards[n - 1]), &new_card, &None) != 0 {
                    cards.push(card);
                }
            }
            Self::from_vec(cards)
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, new: Self) -> Self {
            let mut cards = self.to_vec();
            cards.extend(new.to_vec());
            Self::from_vec(cards)
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // No-op - vec-based implementation handles detachment via iterate_hand
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
            // C uses sort_card_after when sorter is null; in Rust, sorter is required
            // but a function pointer can be passed.
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection::new_empty(),
            })
        }
        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection::new_empty();
        }
        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            if c.is_none() {
                return;
            }
            let cards = std::mem::replace(&mut self.cards, CardCollection::new_empty());
            self.cards = cards.insert_into_collection(*c, self.sorter);
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
            for card in self.cards.to_vec() {
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
            let cards = self.cards.to_vec();
            let n = cards.len();
            let mut to_remove = vec![false; n];
            let mut current_len: u64 = n as u64;
            let mut pos: u64 = 0;
            let mut is_stopped = false;
            let mut idx: usize = 0;
            while !is_stopped && idx < n {
                let opt = Some(cards[idx]);
                let action = itr_fn(current_len, pos, &opt);
                match action {
                    ItrAction::Continue => {}
                    ItrAction::Break => {
                        is_stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        to_remove[idx] = true;
                        current_len = current_len.saturating_sub(1);
                        pos = pos.wrapping_sub(1);
                    }
                    ItrAction::RemoveAndBreak => {
                        to_remove[idx] = true;
                        current_len = current_len.saturating_sub(1);
                        pos = pos.wrapping_sub(1);
                        is_stopped = true;
                    }
                }
                pos = pos.wrapping_add(1);
                idx += 1;
            }
            // Rebuild
            let new_cards: Vec<Card> = cards
                .into_iter()
                .enumerate()
                .filter(|(i, _)| !to_remove[*i])
                .map(|(_, c)| c)
                .collect();
            self.len = new_cards.len() as u8;
            self.cards = CardCollection::from_vec(new_cards);
        }
        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let cards = self.cards.to_vec();
            let new_cards: Vec<Card> = cards
                .into_iter()
                .filter(|card| card.get_card_suit_rank() != c)
                .collect();
            self.len = new_cards.len() as u8;
            self.cards = CardCollection::from_vec(new_cards);
        }
        pub fn remove_from_hand_under_iter(&mut self, _CardCollection: &CardCollection, _pos: usize) {
            // Helper for iterate_hand - in our vec-based impl this isn't used
        }

        // Helper for razz simulation: get cards as a vec
        pub fn cards_as_vec(&self) -> Vec<Card> {
            self.cards.to_vec()
        }
    }
    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
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
            let mut rng = rand::thread_rng();
            let selected_card_idx: usize = rng.gen_range(0..self.card_count as usize);
            let mut valid_card_idx: usize = 0;
            for i in 0..(CardSuitRank::CardCount as usize) {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_card_idx == selected_card_idx {
                        let csr = csr_from_usize(i);
                        self.cards[i] = Card::write_card(csr);
                        self.card_count -= 1;
                        return Some(self.cards[i]);
                    }
                    valid_card_idx += 1;
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
            let cards: [Card; CardSuitRank::CardCount as usize] =
                std::array::from_fn(|_| Card { card: 0 });
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards,
            })
        }
    }
    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;
    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }
    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let r = match new {
            Some(c) => c.get_card_rank(),
            None => return 0,
        };
        let after_some = match after {
            None => return 1,
            Some(a) => a,
        };
        let after_r = after_some.get_card_rank();
        let before_ok = match before {
            None => true,
            Some(b) => r > b.get_card_rank(),
        };
        if before_ok && r <= after_r {
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
