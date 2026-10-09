use chess_core::{bitboard::bb_lsb, prelude::*};

use crate::search::{ContHistKeys, ContinuationHistoryTable};
use crate::{
    attacks::{bishop_attacks, bishop_xray_attacks, rook_attacks, rook_xray_attacks},
    board::Board,
    search::{HistoryTable, KillerMoves},
};

const MVV_RANK: [i16; Piece::NB] = [1, 2, 3, 4, 5, 6]; // P, N, B, R, Q, K
const SEE_ORDERING_TIER: [i32; Piece::NB] = [100, 300, 300, 500, 900, 20000]; // P, N, B, R, Q, K

static MVV_LVA: [[i16; Piece::NB]; Piece::NB] = const {
    let mut table = [[0; Piece::NB]; Piece::NB];
    let mut victim = 0;
    while victim < Piece::NB {
        let mut attacker = 0;
        while attacker < Piece::NB {
            // e.g. victim * 10 - attacker
            table[victim][attacker] = MVV_RANK[victim] * 10 - MVV_RANK[attacker];
            attacker += 1;
        }
        victim += 1;
    }
    table
};
const GOOD_CAPTURES: i16 = 20_000;
const KILLER_1: i16 = 15_000;
const KILLER_2: i16 = 14_000;
const BAD_CAPTURES: i16 = -5_000;

#[inline(always)]
pub fn score_capture(mov: Move, board: &Board) -> i16 {
    let attacker = board
        .piece_at(mov.from())
        .map_or(Piece::Pawn, |p| p.piece());

    // if no victim, must be en-passant
    let victim = board.piece_at(mov.to()).map_or(Piece::Pawn, |p| p.piece());
    let mut score = MVV_LVA[victim as usize][attacker as usize];

    if let Some(promo) = mov.promotion_piece() {
        score += MVV_RANK[promo as usize] * 10;
    }

    let needs_see = (attacker as u8) > (victim as u8);
    score * 2 + needs_see as i16
}

#[inline(always)]
pub fn capture_tier(mov: Move, board: &Board, score: i16) -> i16 {
    let is_good = score & 1 == 0 || see_ge(mov, board, 0);
    let tier = if is_good { GOOD_CAPTURES } else { BAD_CAPTURES };
    tier + (score >> 1)
}

#[inline(always)]
pub fn score_quiet(
    mov: Move,
    board: &Board,
    killer_moves: KillerMoves,
    history: &HistoryTable,
    cont_history: &ContinuationHistoryTable,
    conthist_keys: &ContHistKeys,
) -> i16 {
    if mov == killer_moves[0] {
        KILLER_1
    } else if mov == killer_moves[1] {
        KILLER_2
    } else {
        let piece = unsafe { board.piece_type_at(mov.from()) };
        history.get(board.to_play, mov.from(), mov.to())
            + cont_history.score(conthist_keys, piece, mov.to())
    }
}

#[inline(always)]
fn get_least_valuable_attacker(board: &Board, attackers: u64, side: Color) -> Option<(Sq, Piece)> {
    let side_attackers = attackers & board.colors(side);
    if side_attackers == 0 {
        return None;
    }

    for piece in Piece::ALL {
        let subset = side_attackers & board.pieces(piece);
        if subset != 0 {
            if piece == Piece::King && (attackers & board.colors(!side)) != 0 {
                return None;
            }
            let sq = unsafe { bb_lsb(subset) };
            return Some((sq, piece));
        }
    }

    None
}

pub fn see_ge(mov: Move, board: &Board, threshold: i32) -> bool {
    let from = mov.from();
    let to = mov.to();
    let flags = mov.flags();
    let us = board.to_play;

    let Some(mut attacker) = board.piece_at(from).map(|p| p.piece()) else {
        return false;
    };

    let is_ep = flags == MoveFlags::EnPassant;

    let captured = if is_ep {
        Some(Piece::Pawn)
    } else {
        board.piece_at(to).map(|p| p.piece())
    };

    let cap_val = captured.map_or(0, |p| SEE_ORDERING_TIER[p as usize]);

    let promo_bonus = if let Some(promo) = mov.promotion_piece() {
        let bonus = SEE_ORDERING_TIER[promo as usize] - SEE_ORDERING_TIER[Piece::Pawn as usize];
        attacker = promo;
        bonus
    } else {
        0
    };

    // Initial deficit: if our immediate gain without recapture is below threshold, fail
    let mut swap = cap_val + promo_bonus - threshold;
    if swap < 0 {
        return false;
    }

    // Guaranteed gain: if losing our moving piece still leaves us >= threshold, succeed
    swap = SEE_ORDERING_TIER[attacker as usize] - swap;
    if swap <= 0 {
        return true;
    }

    let from_bb = from.bitboard();
    let to_bb = to.bitboard();

    let mut occupied = board.occupied() ^ from_bb;
    if is_ep {
        occupied ^= mov.capture_square(us).bitboard();
    }
    occupied |= to_bb;

    let mut attackers = board.attackers_to(to, occupied);

    let bishops_queens = board.pieces(Piece::Bishop) | board.pieces(Piece::Queen);
    let rooks_queens = board.pieces(Piece::Rook) | board.pieces(Piece::Queen);

    let mut side = !us;
    let mut res = 1;

    while let Some((next_from, next_attacker)) = get_least_valuable_attacker(board, attackers, side)
    {
        occupied ^= next_from.bitboard();

        if next_from.bitboard() & bishop_xray_attacks(to) != 0 {
            attackers |= bishop_attacks(to, occupied) & bishops_queens;
        }
        if next_from.bitboard() & rook_xray_attacks(to) != 0 {
            attackers |= rook_attacks(to, occupied) & rooks_queens;
        }

        attackers &= occupied;
        side = !side;
        res ^= 1;

        swap = SEE_ORDERING_TIER[next_attacker as usize] - swap;
        if swap < res {
            break;
        }
    }

    res != 0
}
