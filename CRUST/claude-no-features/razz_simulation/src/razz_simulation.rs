use crate::card::card::{
    sort_card_by_rank, Card, CardDeck, CardHand, CardRank, ItrAction,
};
pub mod razz_simulation {
    use super::*;

    pub struct DecidedCards {
        pub my_card_count: u8,
        pub my_cards: [Option<Card>; 3],
        pub opponent_card_count: u8,
        pub opponent_cards: [Option<Card>; 7],
    }

    pub type RankListener<T> = fn(&mut T, CardRank);

    const RAZZ_CARD_IN_HAND_COUNT: u8 = 7;

    // Static for "duplicated_rank_remover" must be reset each invocation,
    // because the C version uses a static local variable that resets at pos==0.
    // We model the remover with a closure that holds state via a shared cell.

    // We can't easily pass state to a fn pointer, so we implement the razz logic directly
    // without using the iterate_hand machinery for these stateful helpers.

    fn duplicated_rank_remove(hand: &mut CardHand) {
        // Remove cards whose rank equals the previous (sorted) card's rank.
        // Use iterate_hand with a workaround: iterate all and remove duplicates.
        // Since we can't use closures with fn pointers, let's manually implement.
        // We'll remove duplicates by ranks; since the hand is sorted by rank, scan
        // and remove cards with same rank as previous.
        // Approach: get the sorted ranks/CSRs, find duplicates.

        // Repeatedly find a duplicate-rank card and remove it via remove_from_hand.
        loop {
            let mut to_remove: Option<crate::card::card::CardSuitRank> = None;
            let mut found = false;
            // Use iterate_hand with a static helper that records first duplicate via
            // a thread_local. To avoid that complexity, instead iterate via removing
            // entries whose rank duplicates a previous one — but we need to enumerate
            // first. Use length_capture iterator to gather ranks via a thread-local,
            // or do this differently:
            // We'll use iterate_hand with a fn pointer that uses thread-local state.
            use std::cell::RefCell;
            thread_local! {
                static SCAN_STATE: RefCell<ScanState> = RefCell::new(ScanState::default());
            }
            #[derive(Default)]
            struct ScanState {
                prev_rank: Option<CardRank>,
                dup_csr: Option<crate::card::card::CardSuitRank>,
                stop: bool,
            }
            fn scan_iter(_len: u64, _pos: u64, c: &Option<Card>) -> ItrAction {
                SCAN_STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    if st.stop {
                        return ItrAction::Break;
                    }
                    let card = match c {
                        Some(c) => c,
                        None => return ItrAction::Continue,
                    };
                    let rank = card.get_card_rank();
                    if let Some(pr) = st.prev_rank {
                        if pr == rank {
                            st.dup_csr = Some(card.get_card_suit_rank());
                            st.stop = true;
                            return ItrAction::Break;
                        }
                    }
                    st.prev_rank = Some(rank);
                    ItrAction::Continue
                })
            }
            SCAN_STATE.with(|s| {
                *s.borrow_mut() = ScanState::default();
            });
            hand.iterate_hand(scan_iter);
            SCAN_STATE.with(|s| {
                let st = s.borrow();
                to_remove = st.dup_csr;
                if st.dup_csr.is_some() {
                    found = true;
                }
            });
            if !found {
                break;
            }
            if let Some(csr) = to_remove {
                hand.remove_from_hand(csr);
            }
        }
    }

    fn length_trim_to_5(hand: &mut CardHand) {
        // Keep only first 5 cards in hand. Equivalent to length_trimmer.
        fn trim_iter(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
            if pos >= 5 {
                ItrAction::RemoveAndContinue
            } else {
                ItrAction::Continue
            }
        }
        hand.iterate_hand(trim_iter);
    }

    fn get_razz_rank(hand: &mut CardHand) -> CardRank {
        duplicated_rank_remove(hand);
        let cards_count = hand.count_cards_in_hand();
        if cards_count < 5 {
            return CardRank::InvalidRank;
        }
        length_trim_to_5(hand);
        hand.get_max_rank_of_hand()
    }

    pub fn simulate_razz_game<T>(
        decided_cards: &DecidedCards,
        game_count: u64,
        arg: &mut T,
        listener: RankListener<T>,
    ) -> i32 {
        let mut my_hand = match CardHand::create_hand(RAZZ_CARD_IN_HAND_COUNT, sort_card_by_rank) {
            Some(h) => h,
            None => return 1,
        };
        for _ in 0..game_count {
            let mut deck = match CardDeck::create_shuffled_deck() {
                Some(d) => d,
                None => return 1,
            };
            strip_deck(&mut deck, decided_cards);
            complete_hand(&mut my_hand, decided_cards, &mut deck);
            let r = get_razz_rank(&mut my_hand);
            listener(arg, r);
            my_hand.reset_hand();
        }
        0
    }

    pub fn strip_deck(deck: &mut CardDeck, decided_cards: &DecidedCards) {
        for i in 0..decided_cards.my_card_count as usize {
            if let Some(card) = &decided_cards.my_cards[i] {
                deck.strip_card_from_deck(card.get_card_suit_rank());
            }
        }
        for i in 0..decided_cards.opponent_card_count as usize {
            if let Some(card) = &decided_cards.opponent_cards[i] {
                deck.strip_card_from_deck(card.get_card_suit_rank());
            }
        }
    }

    pub fn complete_hand(
        my_hand: &mut CardHand,
        decided_cards: &DecidedCards,
        deck: &mut CardDeck,
    ) {
        let end = decided_cards.my_card_count as usize;
        for i in 0..end {
            my_hand.insert_into_hand(&decided_cards.my_cards[i]);
        }
        let remaining = (RAZZ_CARD_IN_HAND_COUNT as usize).saturating_sub(end);
        for _ in 0..remaining {
            let dealt = deck.deal_from_deck();
            my_hand.insert_into_hand(&dealt);
        }
    }
}
