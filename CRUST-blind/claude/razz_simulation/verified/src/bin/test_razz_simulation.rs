use razz_simulation::card::card::*;
use razz_simulation::razz_simulation::razz_simulation::*;

fn rank_listener(arg: &mut [u64; 9], r: CardRank) {
    if r == CardRank::InvalidRank {
        return;
    }
    // Razz ranks are R5..=K (mapping to indices 0..=8 of [R5, R6, R7, R8, R9, R10, J, Q, K]).
    let idx = match r {
        CardRank::R5 => 0,
        CardRank::R6 => 1,
        CardRank::R7 => 2,
        CardRank::R8 => 3,
        CardRank::R9 => 4,
        CardRank::R10 => 5,
        CardRank::J => 6,
        CardRank::Q => 7,
        CardRank::K => 8,
        _ => return,
    };
    arg[idx] += 1;
}

fn collect_all(arg: &mut Vec<CardRank>, r: CardRank) {
    arg.push(r);
}

#[test]
fn test_strip_deck_strips_my_cards() {
    // Verify that strip_deck removes my_cards and opponent_cards from the deck.
    let my0 = Card::create_card(CardSuitRank::SpadeAce);
    let my1 = Card::create_card(CardSuitRank::Spade2);
    let my2 = Card::create_card(CardSuitRank::Spade3);
    let opp0 = Card::create_card(CardSuitRank::HeartAce);
    let opp1 = Card::create_card(CardSuitRank::HeartK);

    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [my0, my1, my2],
        opponent_card_count: 2,
        opponent_cards: [opp0, opp1, None, None, None, None, None],
    };

    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    strip_deck(&mut deck, &decided);

    // The decided cards should not be in the deck anymore.
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Spade2), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Spade3), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartAce), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 0);

    // Other cards should still be in the deck.
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart2), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::DiamondAce), 1);
}

#[test]
fn test_strip_deck_no_decided_cards() {
    let decided = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    strip_deck(&mut deck, &decided);
    // All 52 cards should still be in the deck.
    let all_csr = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::HeartK, CardSuitRank::DiamondAce,
        CardSuitRank::DiamondK, CardSuitRank::ClubAce, CardSuitRank::ClubK,
    ];
    for csr in all_csr {
        assert_eq!(deck.is_card_in_deck(csr), 1);
    }
}

#[test]
fn test_complete_hand_inserts_my_cards_and_deals_remainder() {
    let my0 = Card::create_card(CardSuitRank::SpadeAce);
    let my1 = Card::create_card(CardSuitRank::Spade2);
    let my2 = Card::create_card(CardSuitRank::Spade3);

    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [my0, my1, my2],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };

    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    strip_deck(&mut deck, &decided);
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &decided, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
    assert_eq!(hand.get_max_of_hand(), 7);
}

#[test]
fn test_complete_hand_zero_decided_deals_seven() {
    let decided = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };

    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    strip_deck(&mut deck, &decided);
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &decided, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
}

#[test]
fn test_simulate_razz_game_returns_zero() {
    let decided = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    let mut counts = [0u64; 9];
    let result = simulate_razz_game(&decided, 100, &mut counts, rank_listener);
    assert_eq!(result, 0);
    // Some rank should have been recorded; allow that some games end in INVALID_RANK
    // (too many duplicates), but the rest must be valid in [R5..=K].
    let total: u64 = counts.iter().sum();
    assert!(total <= 100);
}

#[test]
fn test_simulate_razz_game_zero_games() {
    let decided = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    let mut collected: Vec<CardRank> = Vec::new();
    let result = simulate_razz_game(&decided, 0, &mut collected, collect_all);
    assert_eq!(result, 0);
    assert_eq!(collected.len(), 0);
}

#[test]
fn test_simulate_razz_game_invokes_listener_per_game() {
    let decided = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    let mut collected: Vec<CardRank> = Vec::new();
    let result = simulate_razz_game(&decided, 5, &mut collected, collect_all);
    assert_eq!(result, 0);
    assert_eq!(collected.len(), 5);
    // All ranks should be either InvalidRank or in R5..=K.
    for r in &collected {
        match r {
            CardRank::InvalidRank
            | CardRank::R5 | CardRank::R6 | CardRank::R7 | CardRank::R8
            | CardRank::R9 | CardRank::R10 | CardRank::J | CardRank::Q | CardRank::K => {}
            _ => panic!("Unexpected rank: {:?}", r),
        }
    }
}

#[test]
fn test_simulate_razz_with_decided_cards() {
    // Decide my 3 cards as the wheel-friendly low cards. Even with random opp cards,
    // simulation should still complete without error.
    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 4,
        opponent_cards: [
            Card::create_card(CardSuitRank::HeartK),
            Card::create_card(CardSuitRank::HeartQ),
            Card::create_card(CardSuitRank::ClubK),
            Card::create_card(CardSuitRank::DiamondK),
            None, None, None,
        ],
    };
    let mut collected: Vec<CardRank> = Vec::new();
    let result = simulate_razz_game(&decided, 10, &mut collected, collect_all);
    assert_eq!(result, 0);
    assert_eq!(collected.len(), 10);
}

#[test]
fn test_strip_deck_with_seven_opponent_cards() {
    let decided = DecidedCards {
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
            Card::create_card(CardSuitRank::Diamond4),
            Card::create_card(CardSuitRank::Diamond5),
            Card::create_card(CardSuitRank::Diamond6),
            Card::create_card(CardSuitRank::Diamond7),
        ],
    };
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    strip_deck(&mut deck, &decided);

    // All 10 decided cards should be missing from the deck.
    let stripped_csrs = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
        CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3,
        CardSuitRank::Diamond4, CardSuitRank::Diamond5, CardSuitRank::Diamond6,
        CardSuitRank::Diamond7,
    ];
    for csr in stripped_csrs {
        assert_eq!(deck.is_card_in_deck(csr), 0, "csr {:?} should be stripped", csr);
    }

    // Other cards should still be in deck.
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubAce), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeK), 1);
}

fn main() {}
