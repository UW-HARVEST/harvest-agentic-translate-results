use crate::card::card::{sort_card_by_rank, Card, CardDeck, CardHand, CardRank};
pub mod razz_simulation {
    use super::*;

    const RAZZ_CARD_IN_HAND_COUNT: usize = 7;

    pub struct DecidedCards {
        pub my_card_count: u8,
        pub my_cards: [Option<Card>; 3],
        pub opponent_card_count: u8,
        pub opponent_cards: [Option<Card>; 7],
    }
    pub type RankListener<T> = fn(&mut T, CardRank);
    pub fn simulate_razz_game<T>(decided_cards: &DecidedCards, game_count: u64, arg: &mut T, listener: RankListener<T>) -> i32 {
        let Some(mut my_hand) = CardHand::create_hand(RAZZ_CARD_IN_HAND_COUNT as u8, sort_card_by_rank) else {
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
        for card in decided_cards
            .my_cards
            .iter()
            .take(decided_cards.my_card_count as usize)
            .flatten()
        {
            deck.strip_card_from_deck(card.get_card_suit_rank());
        }

        for card in decided_cards
            .opponent_cards
            .iter()
            .take(decided_cards.opponent_card_count as usize)
            .flatten()
        {
            deck.strip_card_from_deck(card.get_card_suit_rank());
        }
    }
    pub fn complete_hand(my_hand: &mut CardHand, decided_cards: &DecidedCards, deck: &mut CardDeck) {
        for card in decided_cards
            .my_cards
            .iter()
            .take(decided_cards.my_card_count as usize)
        {
            my_hand.insert_into_hand(card);
        }

        let remaining = RAZZ_CARD_IN_HAND_COUNT.saturating_sub(decided_cards.my_card_count as usize);
        for _ in 0..remaining {
            let dealt = deck.deal_from_deck();
            my_hand.insert_into_hand(&dealt);
        }
    }

    fn get_razz_rank(hand: &mut CardHand) -> CardRank {
        let mut cards = hand.snapshot_cards();

        let mut deduped = Vec::with_capacity(cards.len());
        let mut prev_rank = None;
        for card in cards.drain(..) {
            let rank = card.get_card_rank();
            if prev_rank == Some(rank) {
                continue;
            }
            prev_rank = Some(rank);
            deduped.push(card);
        }

        if deduped.len() < 5 {
            hand.replace_cards(deduped);
            return CardRank::InvalidRank;
        }

        deduped.truncate(5);
        hand.replace_cards(deduped);
        hand.get_max_rank_of_hand()
    }
}
