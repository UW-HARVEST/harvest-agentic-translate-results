use crate::card::card::{Card, CardDeck, CardHand, CardRank, ItrAction, sort_card_by_rank};

pub mod razz_simulation {
    use super::*;

    const RAZZ_CARD_IN_HAND_COUNT: u8 = 7;

    pub struct DecidedCards {
        pub my_card_count: u8,
        pub my_cards: [Option<Card>; 3],
        pub opponent_card_count: u8,
        pub opponent_cards: [Option<Card>; 7],
    }

    pub type RankListener<T> = fn(&mut T, CardRank);

    fn get_razz_rank(hand: &mut CardHand) -> CardRank {
        hand.iterate_hand(duplicated_rank_remover);

        if hand.count_cards_in_hand() < 5 {
            return CardRank::InvalidRank;
        }

        hand.iterate_hand(length_trimmer);
        hand.get_max_rank_of_hand()
    }

    fn duplicated_rank_remover(_len: u64, pos: u64, c: &Option<Card>) -> ItrAction {
        static PREV_RANK: std::sync::Mutex<CardRank> = std::sync::Mutex::new(CardRank::InvalidRank);
        let curr_rank = c.as_ref().map(Card::get_card_rank).unwrap_or(CardRank::InvalidRank);

        if pos == 0 {
            *PREV_RANK.lock().unwrap() = curr_rank;
            return ItrAction::Continue;
        }

        let mut prev_rank = PREV_RANK.lock().unwrap();
        if *prev_rank == curr_rank {
            return ItrAction::RemoveAndContinue;
        }

        *prev_rank = curr_rank;
        ItrAction::Continue
    }

    fn length_trimmer(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
        if pos >= 5 {
            ItrAction::RemoveAndContinue
        } else {
            ItrAction::Continue
        }
    }

    pub fn simulate_razz_game<T>(decided_cards: &DecidedCards, game_count: u64, arg: &mut T, listener: RankListener<T>) -> i32 {
        let Some(mut my_hand) = CardHand::create_hand(RAZZ_CARD_IN_HAND_COUNT, sort_card_by_rank) else {
            return 1;
        };

        for _ in 0..game_count {
            let Some(mut deck) = CardDeck::create_shuffled_deck() else {
                return 1;
            };

            strip_deck(&mut deck, decided_cards);
            complete_hand(&mut my_hand, decided_cards, &mut deck);
            listener(arg, get_razz_rank(&mut my_hand));
            my_hand.reset_hand();
        }

        0
    }

    pub fn strip_deck(deck: &mut CardDeck, decided_cards: &DecidedCards) {
        for i in 0..decided_cards.my_card_count as usize {
            if let Some(card) = decided_cards.my_cards[i] {
                deck.strip_card_from_deck(card.get_card_suit_rank());
            }
        }

        for i in 0..decided_cards.opponent_card_count as usize {
            if let Some(card) = decided_cards.opponent_cards[i] {
                deck.strip_card_from_deck(card.get_card_suit_rank());
            }
        }
    }

    pub fn complete_hand(my_hand: &mut CardHand, decided_cards: &DecidedCards, deck: &mut CardDeck) {
        for i in 0..decided_cards.my_card_count as usize {
            my_hand.insert_into_hand(&decided_cards.my_cards[i]);
        }

        let remaining = RAZZ_CARD_IN_HAND_COUNT.saturating_sub(decided_cards.my_card_count);
        for _ in 0..remaining {
            let dealt = deck.deal_from_deck();
            my_hand.insert_into_hand(&dealt);
        }
    }
}
