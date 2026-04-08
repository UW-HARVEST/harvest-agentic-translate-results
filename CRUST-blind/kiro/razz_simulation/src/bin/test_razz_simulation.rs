use razz_simulation::card::card::*;
use razz_simulation::razz_simulation::razz_simulation::*;

// --- DecidedCards construction ---
#[test]
fn test_decided_cards_creation() {
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 1,
        opponent_cards: [
            Card::create_card(CardSuitRank::Heart5),
            None, None, None, None, None, None,
        ],
    };
    assert_eq!(dc.my_card_count, 3);
    assert_eq!(dc.opponent_card_count, 1);
}

// --- strip_deck ---
#[test]
fn test_strip_deck() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 2,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart5),
            None,
        ],
        opponent_card_count: 1,
        opponent_cards: [
            Card::create_card(CardSuitRank::ClubK),
            None, None, None, None, None, None,
        ],
    };
    // Before stripping, cards should be in deck
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart5), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 1);

    strip_deck(&mut deck, &dc);

    // After stripping, cards should not be in deck
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart5), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 0);
    // Other cards still in deck
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Diamond2), 1);
}

// --- complete_hand ---
#[test]
fn test_complete_hand() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    strip_deck(&mut deck, &dc);

    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &dc, &mut deck);
    // Should have 7 cards: 3 decided + 4 dealt
    assert_eq!(hand.count_cards_in_hand(), 7);
}

#[test]
fn test_complete_hand_with_opponents() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 2,
        opponent_cards: [
            Card::create_card(CardSuitRank::HeartAce),
            Card::create_card(CardSuitRank::Heart2),
            None, None, None, None, None,
        ],
    };
    strip_deck(&mut deck, &dc);

    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &dc, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
}

// --- simulate_razz_game ---
#[test]
fn test_simulate_razz_game_returns_zero() {
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };

    fn listener(ranks: &mut Vec<CardRank>, r: CardRank) {
        ranks.push(r);
    }

    let mut results: Vec<CardRank> = Vec::new();
    let ret = simulate_razz_game(&dc, 10, &mut results, listener);
    assert_eq!(ret, 0);
    assert_eq!(results.len(), 10);
}

#[test]
fn test_simulate_razz_game_rank_values() {
    // Each game should produce either a valid rank (R5..K) or InvalidRank
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 1,
        opponent_cards: [
            Card::create_card(CardSuitRank::HeartAce),
            None, None, None, None, None, None,
        ],
    };

    fn listener(ranks: &mut Vec<CardRank>, r: CardRank) {
        ranks.push(r);
    }

    let mut results: Vec<CardRank> = Vec::new();
    let ret = simulate_razz_game(&dc, 100, &mut results, listener);
    assert_eq!(ret, 0);
    assert_eq!(results.len(), 100);

    for r in &results {
        // Should be InvalidRank or between R5 and K inclusive
        assert!(
            *r == CardRank::InvalidRank || (*r >= CardRank::R5 && *r <= CardRank::K),
            "Unexpected rank: {:?}", r
        );
    }
}

#[test]
fn test_simulate_razz_game_zero_games() {
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };

    fn listener(count: &mut u64, _r: CardRank) {
        *count += 1;
    }

    let mut count: u64 = 0;
    let ret = simulate_razz_game(&dc, 0, &mut count, listener);
    assert_eq!(ret, 0);
    assert_eq!(count, 0);
}

#[test]
fn test_simulate_razz_game_with_max_opponents() {
    // 3 my cards + 7 opponent cards = 10 decided cards
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 7,
        opponent_cards: [
            Card::create_card(CardSuitRank::HeartAce),
            Card::create_card(CardSuitRank::Heart2),
            Card::create_card(CardSuitRank::Heart3),
            Card::create_card(CardSuitRank::Heart4),
            Card::create_card(CardSuitRank::Heart5),
            Card::create_card(CardSuitRank::Heart6),
            Card::create_card(CardSuitRank::Heart7),
        ],
    };

    fn listener(count: &mut u64, _r: CardRank) {
        *count += 1;
    }

    let mut count: u64 = 0;
    let ret = simulate_razz_game(&dc, 5, &mut count, listener);
    assert_eq!(ret, 0);
    assert_eq!(count, 5);
}

// --- Test that strip_deck with zero cards is safe ---
#[test]
fn test_strip_deck_zero_cards() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    strip_deck(&mut deck, &dc);
    // All 52 cards should still be in deck
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 1);
}

fn main() {}
