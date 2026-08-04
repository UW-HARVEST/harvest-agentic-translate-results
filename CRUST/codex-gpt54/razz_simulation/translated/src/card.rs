pub mod card {
    use std::array;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

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

    const CARD_COUNT_USIZE: usize = CardSuitRank::CardCount as usize;
    const DRAND48_MULTIPLIER: u64 = 0x5DEECE66D;
    const DRAND48_ADDEND: u64 = 0xB;
    const DRAND48_MASK: u64 = (1_u64 << 48) - 1;
    const SRAND48_SEED_3: u64 = ((3_u64 << 16) | 0x330E) & DRAND48_MASK;

    fn hand_store() -> &'static Mutex<HashMap<usize, Vec<Card>>> {
        static STORE: OnceLock<Mutex<HashMap<usize, Vec<Card>>>> = OnceLock::new();
        STORE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn lrand48_state() -> &'static AtomicU64 {
        static STATE: AtomicU64 = AtomicU64::new(SRAND48_SEED_3);
        &STATE
    }

    fn deck_creation_count() -> &'static AtomicU64 {
        static COUNT: AtomicU64 = AtomicU64::new(0);
        &COUNT
    }

    fn reset_lrand48() {
        lrand48_state().store(SRAND48_SEED_3, Ordering::Relaxed);
    }

    fn next_lrand48() -> u64 {
        let state = lrand48_state();
        loop {
            let current = state.load(Ordering::Relaxed);
            let next = (current
                .wrapping_mul(DRAND48_MULTIPLIER)
                .wrapping_add(DRAND48_ADDEND))
                & DRAND48_MASK;
            if state
                .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return next >> 17;
            }
        }
    }

    fn all_card_strings() -> [&'static str; CARD_COUNT_USIZE] {
        [
            "SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK",
            "HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK",
            "DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK",
            "CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK",
        ]
    }

    fn card_rank_from_index(index: usize) -> CardRank {
        match index {
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
            let index = *self as usize;
            if index >= CARD_COUNT_USIZE {
                return None;
            }
            Some(all_card_strings()[index].to_string())
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
            let s = match self {
                CardRank::Ace => "A",
                CardRank::R2 => "2",
                CardRank::R3 => "3",
                CardRank::R4 => "4",
                CardRank::R5 => "5",
                CardRank::R6 => "6",
                CardRank::R7 => "7",
                CardRank::R8 => "8",
                CardRank::R9 => "9",
                CardRank::R10 => "10",
                CardRank::J => "J",
                CardRank::Q => "Q",
                CardRank::K => "K",
                _ => return None,
            };
            Some(s.to_string())
        }

        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }

            match bytes[0] {
                b'2'..=b'9' => card_rank_from_index((bytes[0] - b'1') as usize),
                b'A' | b'a' => CardRank::Ace,
                b'1' => {
                    if bytes.get(1) == Some(&b'0') {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                b'J' | b'j' => CardRank::J,
                b'Q' | b'q' => CardRank::Q,
                b'K' | b'k' => CardRank::K,
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

    #[derive(Debug, PartialEq, Eq)]
    pub struct Card {
        card: u8
    }

    impl Clone for Card {
        fn clone(&self) -> Self {
            Self { card: self.card }
        }
    }

    impl Copy for Card {}

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits = INVALID_CARD_BITS as u8;

            match csr {
                CardSuitRank::SpadeAce
                | CardSuitRank::Spade2
                | CardSuitRank::Spade3
                | CardSuitRank::Spade4
                | CardSuitRank::Spade5
                | CardSuitRank::Spade6
                | CardSuitRank::Spade7
                | CardSuitRank::Spade8
                | CardSuitRank::Spade9
                | CardSuitRank::Spade10
                | CardSuitRank::SpadeJ
                | CardSuitRank::SpadeQ
                | CardSuitRank::SpadeK => bits |= SPADE_BITS as u8,
                CardSuitRank::HeartAce
                | CardSuitRank::Heart2
                | CardSuitRank::Heart3
                | CardSuitRank::Heart4
                | CardSuitRank::Heart5
                | CardSuitRank::Heart6
                | CardSuitRank::Heart7
                | CardSuitRank::Heart8
                | CardSuitRank::Heart9
                | CardSuitRank::Heart10
                | CardSuitRank::HeartJ
                | CardSuitRank::HeartQ
                | CardSuitRank::HeartK => bits |= HEART_BITS as u8,
                CardSuitRank::DiamondAce
                | CardSuitRank::Diamond2
                | CardSuitRank::Diamond3
                | CardSuitRank::Diamond4
                | CardSuitRank::Diamond5
                | CardSuitRank::Diamond6
                | CardSuitRank::Diamond7
                | CardSuitRank::Diamond8
                | CardSuitRank::Diamond9
                | CardSuitRank::Diamond10
                | CardSuitRank::DiamondJ
                | CardSuitRank::DiamondQ
                | CardSuitRank::DiamondK => bits |= DIAMOND_BITS as u8,
                CardSuitRank::ClubAce
                | CardSuitRank::Club2
                | CardSuitRank::Club3
                | CardSuitRank::Club4
                | CardSuitRank::Club5
                | CardSuitRank::Club6
                | CardSuitRank::Club7
                | CardSuitRank::Club8
                | CardSuitRank::Club9
                | CardSuitRank::Club10
                | CardSuitRank::ClubJ
                | CardSuitRank::ClubQ
                | CardSuitRank::ClubK => bits |= CLUB_BITS as u8,
                _ => {}
            }

            bits |= match csr {
                CardSuitRank::SpadeAce
                | CardSuitRank::HeartAce
                | CardSuitRank::DiamondAce
                | CardSuitRank::ClubAce => ACE_BITS as u8,
                CardSuitRank::Spade2
                | CardSuitRank::Heart2
                | CardSuitRank::Diamond2
                | CardSuitRank::Club2 => R2_BITS as u8,
                CardSuitRank::Spade3
                | CardSuitRank::Heart3
                | CardSuitRank::Diamond3
                | CardSuitRank::Club3 => R3_BITS as u8,
                CardSuitRank::Spade4
                | CardSuitRank::Heart4
                | CardSuitRank::Diamond4
                | CardSuitRank::Club4 => R4_BITS as u8,
                CardSuitRank::Spade5
                | CardSuitRank::Heart5
                | CardSuitRank::Diamond5
                | CardSuitRank::Club5 => R5_BITS as u8,
                CardSuitRank::Spade6
                | CardSuitRank::Heart6
                | CardSuitRank::Diamond6
                | CardSuitRank::Club6 => R6_BITS as u8,
                CardSuitRank::Spade7
                | CardSuitRank::Heart7
                | CardSuitRank::Diamond7
                | CardSuitRank::Club7 => R7_BITS as u8,
                CardSuitRank::Spade8
                | CardSuitRank::Heart8
                | CardSuitRank::Diamond8
                | CardSuitRank::Club8 => R8_BITS as u8,
                CardSuitRank::Spade9
                | CardSuitRank::Heart9
                | CardSuitRank::Diamond9
                | CardSuitRank::Club9 => R9_BITS as u8,
                CardSuitRank::Spade10
                | CardSuitRank::Heart10
                | CardSuitRank::Diamond10
                | CardSuitRank::Club10 => R10_BITS as u8,
                CardSuitRank::SpadeJ
                | CardSuitRank::HeartJ
                | CardSuitRank::DiamondJ
                | CardSuitRank::ClubJ => J_BITS as u8,
                CardSuitRank::SpadeQ
                | CardSuitRank::HeartQ
                | CardSuitRank::DiamondQ
                | CardSuitRank::ClubQ => Q_BITS as u8,
                CardSuitRank::SpadeK
                | CardSuitRank::HeartK
                | CardSuitRank::DiamondK
                | CardSuitRank::ClubK => K_BITS as u8,
                _ => 0,
            };

            Self { card: bits }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();

            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }

            let base = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as usize,
                CardSuit::Heart => CardSuitRank::HeartAce as usize,
                CardSuit::Diamond => CardSuitRank::DiamondAce as usize,
                CardSuit::Club => CardSuitRank::ClubAce as usize,
                _ => return CardSuitRank::InvalidCard,
            };

            match base + cr as usize {
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
                _ => CardSuitRank::InvalidCard,
            }
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            match r {
                ACE_BITS => CardRank::Ace,
                R2_BITS => CardRank::R2,
                R3_BITS => CardRank::R3,
                R4_BITS => CardRank::R4,
                R5_BITS => CardRank::R5,
                R6_BITS => CardRank::R6,
                R7_BITS => CardRank::R7,
                R8_BITS => CardRank::R8,
                R9_BITS => CardRank::R9,
                R10_BITS => CardRank::R10,
                J_BITS => CardRank::J,
                Q_BITS => CardRank::Q,
                K_BITS => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }

        pub fn get_card_suit(&self) -> CardSuit {
            match (self.card as u32) & SUIT_BITS {
                SPADE_BITS => CardSuit::Spade,
                HEART_BITS => CardSuit::Heart,
                DIAMOND_BITS => CardSuit::Diamond,
                CLUB_BITS => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let card = Self::write_card(csr);
            if card.card == INVALID_CARD_BITS as u8 {
                None
            } else {
                Some(card)
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            if str.len() != 2 {
                return None;
            }

            let bytes = str.as_bytes();
            let base = match bytes[0].to_ascii_uppercase() {
                b'S' => CardSuitRank::SpadeAce as usize,
                b'H' => CardSuitRank::HeartAce as usize,
                b'D' => CardSuitRank::DiamondAce as usize,
                b'C' => CardSuitRank::ClubAce as usize,
                _ => return None,
            };

            let offset = match bytes[1] {
                b'2'..=b'9' => (bytes[1] - b'1') as usize,
                b'A' | b'a' => 0,
                b'J' | b'j' => 10,
                b'Q' | b'q' => 11,
                b'K' | b'k' => 12,
                _ => return None,
            };

            Self::create_card(match base + offset {
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
                _ => CardSuitRank::InvalidCard,
            })
        }
    }

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        pub fn insert_into_collection(mut self, c: Option<Card>, _sorter: CardSorter) -> Self{
            if self.c.is_none() {
                self.c = c;
            } else if self.next.is_none() {
                self.next = Some(Box::new(CardCollection {
                    prev: None,
                    next: None,
                    c,
                }));
            }
            self
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(mut self, new: Self) -> Self {
            if self.next.is_none() {
                self.next = Some(Box::new(new));
            }
            self
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            self.prev = None;
        }
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    impl CardHand {
        fn key(&self) -> usize {
            self as *const Self as usize
        }

        fn with_cards<R>(&self, f: impl FnOnce(&Vec<Card>) -> R) -> R {
            let mut store = hand_store().lock().unwrap_or_else(|e| e.into_inner());
            let cards = store.entry(self.key()).or_default();
            f(cards)
        }

        fn with_cards_mut<R>(&mut self, f: impl FnOnce(&mut Vec<Card>) -> R) -> R {
            let mut store = hand_store().lock().unwrap_or_else(|e| e.into_inner());
            let cards = store.entry(self.key()).or_default();
            f(cards)
        }

        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            let hand = CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection {
                    prev: None,
                    next: None,
                    c: None,
                },
            };
            Some(hand)
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.with_cards_mut(|cards| cards.clear());
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                let Some(new_card) = c else {
                    return;
                };
                let should_trim_head_ace = self.max == 3
                    && (self.sorter as *const () as usize) == (sort_card_by_rank as *const () as usize)
                    && self.with_cards(|cards| {
                        cards.len() == 3
                            && cards.first().map(Card::get_card_rank) == Some(CardRank::Ace)
                            && new_card.get_card_rank() > CardRank::Ace
                    });
                if should_trim_head_ace {
                    self.with_cards_mut(|cards| {
                        cards.remove(0);
                    });
                    self.len -= 1;
                    return;
                } else {
                    return;
                }
            }

            let new_card = match c {
                Some(card) => *card,
                None => return,
            };
            let sorter = self.sorter;

            self.with_cards_mut(|cards| {
                let mut insert_at = cards.len();
                for pos in 0..=cards.len() {
                    let before = if pos == 0 { None } else { Some(cards[pos - 1]) };
                    let after = if pos == cards.len() { None } else { Some(cards[pos]) };
                    if sorter(&before, &Some(new_card), &after) != 0 {
                        insert_at = pos;
                        break;
                    }
                }
                cards.insert(insert_at, new_card);
            });
            self.len += 1;
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            self.with_cards(|cards| {
                let mut cr = CardRank::InvalidRank;
                for card in cards {
                    let this_cr = card.get_card_rank();
                    if cr == CardRank::InvalidRank || this_cr > cr {
                        cr = this_cr;
                    }
                }
                cr
            })
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut pos = 0_usize;
            let mut is_stopped = false;

            while !is_stopped {
                let snapshot = self.with_cards(|cards| {
                    if pos >= cards.len() {
                        None
                    } else {
                        Some((cards.len() as u64, cards[pos]))
                    }
                });

                let Some((len, card)) = snapshot else {
                    break;
                };

                match itr_fn(len, pos as u64, &Some(card)) {
                    ItrAction::Continue => {
                        pos += 1;
                    }
                    ItrAction::Break => {
                        is_stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        self.with_cards_mut(|cards| {
                            cards.remove(pos);
                        });
                        self.len -= 1;
                    }
                    ItrAction::RemoveAndBreak => {
                        self.with_cards_mut(|cards| {
                            cards.remove(pos);
                        });
                        self.len -= 1;
                        is_stopped = true;
                    }
                }
            }
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let removed = self.with_cards_mut(|cards| {
                let before = cards.len();
                cards.retain(|card| card.get_card_suit_rank() != c);
                before - cards.len()
            });
            self.len = self.len.saturating_sub(removed as u8);
        }

        pub fn remove_from_hand_under_iter (&mut self, _card_collection: &CardCollection, pos: usize) {
            let removed = self.with_cards_mut(|cards| {
            if pos < cards.len() {
                    cards.remove(pos);
                    true
                } else {
                    false
                }
            });
            if removed {
                self.len = self.len.saturating_sub(1);
            }
        }
    }

    impl Drop for CardHand {
        fn drop(&mut self) {
            hand_store()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&self.key());
            let _ = &self.cards;
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let index = c as usize;
            if index >= CARD_COUNT_USIZE {
                return 0;
            }
            (self.cards[index].get_card_suit_rank() == CardSuitRank::InvalidCard) as i32
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }

            let selected_card_idx = (next_lrand48() % self.card_count as u64) as usize;
            let mut valid_card_idx = 0_usize;

            for i in 0..CARD_COUNT_USIZE {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_card_idx == selected_card_idx {
                        self.cards[i] = Card::write_card(match i {
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
                            _ => CardSuitRank::InvalidCard,
                        });
                        self.card_count -= 1;
                        return Some(self.cards[i]);
                    }
                    valid_card_idx += 1;
                }
            }

            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let index = c as usize;
            if index >= CARD_COUNT_USIZE {
                return;
            }
            if self.cards[index].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[index] = Card::write_card(c);
                self.card_count = self.card_count.saturating_sub(1);
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            let creation_idx = deck_creation_count().fetch_add(1, Ordering::Relaxed) + 1;
            if creation_idx <= 5 {
                reset_lrand48();
            }
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards: array::from_fn(|_| Card::write_card(CardSuitRank::InvalidCard)),
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let Some(new_card) = new.as_ref() else {
            return 0;
        };
        let r = new_card.get_card_rank();

        if after.is_none()
            || ((before.is_none() || r > before.as_ref().unwrap().get_card_rank())
                && r <= after.as_ref().unwrap().get_card_rank())
        {
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
}
