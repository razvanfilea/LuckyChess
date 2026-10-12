use std::cell::Cell;
use std::fmt;

pub mod fen;

use crate::attacks::{self, *};
use crate::move_gen::gen_all_moves;
use crate::zobrist::ZOBRIST_KEYS;
use chess_core::bitboard::{LIGHT_SQUARES, bb_line, bb_several};
use chess_core::{
    bitboard::{bb_between, bb_lsb, bb_only_one},
    for_each_bit,
    prelude::*,
};

const UNCOMPUTED: u64 = u64::MAX;

#[derive(Clone)]
pub struct Board {
    pub mailbox: [Option<ColoredPiece>; Sq::NB],
    pub bit_colors: [u64; Color::NB],
    pub bit_pieces: [u64; Piece::NB],

    pub to_play: Color,
    pub ply: u16,
    state_index: u16,
    state_array: [BoardState; 64],
}

#[derive(Clone)]
struct BoardState {
    hash: u64,
    captured_piece: Option<ColoredPiece>,
    castling_rights: CastlingRights,
    en_passant_target_sq: Option<Sq>,
    half_move_clock: u8, // 50 move draw rule
    checkers: u64,

    pinned: Cell<u64>,
    // Lazily filled by gives_check; discovered_blockers doubles as the computed flag
    check_squares: [Cell<u64>; Piece::NB],
    discovered_blockers: Cell<u64>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            hash: 0,
            captured_piece: None,
            castling_rights: CastlingRights::empty(),
            en_passant_target_sq: None,
            half_move_clock: 0,
            checkers: 0,
            pinned: Cell::new(UNCOMPUTED),
            check_squares: [const { Cell::new(0) }; Piece::NB],
            discovered_blockers: Cell::new(UNCOMPUTED),
        }
    }
}

// Public impl
impl Board {
    pub fn start_pos() -> Self {
        fen::parse_fen(fen::START_POS_FEN).expect("Initial position is valid")
    }

    #[inline(always)]
    pub fn from_fen(fen: &str) -> Option<Self> {
        fen::parse_fen(fen)
    }

    #[inline(always)]
    pub fn to_fen(&self) -> String {
        fen::format_fen(self)
    }

    #[inline(always)]
    pub fn checkers(&self) -> u64 {
        self.state().checkers
    }

    #[inline(always)]
    pub fn colors(&self, color: Color) -> u64 {
        self.bit_colors[color as usize]
    }

    #[inline(always)]
    pub fn colors_mut(&mut self, color: Color) -> &mut u64 {
        &mut self.bit_colors[color as usize]
    }

    #[inline(always)]
    pub fn pieces(&self, piece: Piece) -> u64 {
        self.bit_pieces[piece as usize]
    }

    #[inline(always)]
    pub fn pieces_mut(&mut self, piece: Piece) -> &mut u64 {
        &mut self.bit_pieces[piece as usize]
    }

    #[inline(always)]
    pub fn color_piece(&self, piece: Piece, color: Color) -> u64 {
        self.colors(color) & self.pieces(piece)
    }

    #[inline(always)]
    pub fn colored_piece(&self, piece: ColoredPiece) -> u64 {
        self.colors(piece.color()) & self.pieces(piece.piece())
    }

    #[inline(always)]
    pub fn empty(&self) -> u64 {
        !self.occupied()
    }

    #[inline(always)]
    pub fn occupied(&self) -> u64 {
        self.bit_colors[0] | self.bit_colors[1]
    }

    #[inline(always)]
    pub fn piece_at(&self, sq: Sq) -> Option<ColoredPiece> {
        self.mailbox[sq as usize]
    }

    /// # Safety
    /// The caller must ensure the piece exists at the given square
    #[inline(always)]
    pub unsafe fn piece_type_at(&self, sq: Sq) -> Piece {
        let piece = self.mailbox[sq as usize];
        debug_assert!(piece.is_some());
        unsafe { piece.unwrap_unchecked() }.piece()
    }

    #[inline(always)]
    pub fn en_passant_target_sq(&self) -> Option<Sq> {
        self.state().en_passant_target_sq
    }

    #[inline(always)]
    pub fn castling_rights(&self) -> CastlingRights {
        self.state().castling_rights
    }

    #[inline(always)]
    pub fn hash(&self) -> u64 {
        self.state().hash
    }

    #[inline(always)]
    pub fn half_move_clock(&self) -> u8 {
        self.state().half_move_clock
    }

    #[inline(always)]
    pub fn king_sq(&self, color: Color) -> Sq {
        let king_bb = self.color_piece(Piece::King, color);
        unsafe { bb_lsb(king_bb) }
    }

    #[inline(always)]
    pub fn in_check(&self) -> bool {
        self.state().checkers != 0
    }

    #[inline(always)]
    pub fn is_draw(&self) -> bool {
        if self.state().half_move_clock >= 100 {
            return true;
        }

        if self.occupied().count_ones() > 4 {
            return false;
        }

        self.has_insufficient_material()
    }

    #[inline]
    pub fn generate_attackers(
        &self,
        attacked_sq: Sq,
        attacking_color: Color,
        occupied: u64,
    ) -> u64 {
        let enemy = self.colors(attacking_color);

        let enemy_bishop = (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen)) & enemy;
        let enemy_rook = (self.pieces(Piece::Rook) | self.pieces(Piece::Queen)) & enemy;

        let enemy_pawns = self.pieces(Piece::Pawn) & enemy;
        let enemy_knights = self.pieces(Piece::Knight) & enemy;
        let enemy_kings = self.pieces(Piece::King) & enemy;

        (pawn_attacks(attacked_sq, !attacking_color) & enemy_pawns)
            | (knight_attacks(attacked_sq) & enemy_knights)
            | (bishop_attacks(attacked_sq, occupied) & enemy_bishop)
            | (rook_attacks(attacked_sq, occupied) & enemy_rook)
            | (king_attacks(attacked_sq) & enemy_kings)
    }

    #[inline]
    pub fn attackers_to(&self, sq: Sq, occupied: u64) -> u64 {
        let pawns = (pawn_attacks(sq, Color::Black) & self.colors(Color::White)
            | pawn_attacks(sq, Color::White) & self.colors(Color::Black))
            & self.pieces(Piece::Pawn);
        let knights = knight_attacks(sq) & self.pieces(Piece::Knight);
        let kings = king_attacks(sq) & self.pieces(Piece::King);
        let bishops =
            bishop_attacks(sq, occupied) & (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen));
        let rooks =
            rook_attacks(sq, occupied) & (self.pieces(Piece::Rook) | self.pieces(Piece::Queen));

        (pawns | knights | kings | bishops | rooks) & occupied
    }

    #[inline]
    pub fn pseudo_legal(&self, mov: Move) -> bool {
        if mov.is_none() {
            return false;
        }

        let from = mov.from();
        let to = mov.to();
        if from == to {
            return false;
        }

        let to_bb = to.bitboard();
        let occupied = self.occupied();
        let us = self.to_play;

        let Some(colored_piece) = self.piece_at(from).filter(|p| p.color() == us) else {
            return false;
        };
        let piece = colored_piece.piece();

        if to_bb & self.colors(us) != 0 {
            return false;
        }

        if mov.is_promotion() || mov.is_castle() || mov.flags() == MoveFlags::EnPassant {
            return gen_all_moves(self).as_slice().iter().any(|&sm| sm == mov);
        }

        let checkers = self.state().checkers;
        if checkers != 0 {
            if piece == Piece::King {
                if king_attacks(from) & to_bb == 0 {
                    return false;
                }
                return self.validate_capture_flag(mov, to_bb);
            }

            // Double check: only king moves can evade
            if bb_several(checkers) {
                return false;
            }

            // Single check: non-king piece must capture checker or interpose/block ray
            let checker_sq = unsafe { bb_lsb(checkers) };
            let king_sq = self.king_sq(us);
            let evasion_mask = checkers | bb_between(king_sq, checker_sq);

            let is_ep = mov.flags() == MoveFlags::EnPassant;
            if !is_ep && (to_bb & evasion_mask == 0) {
                return false;
            }
        }

        if !self.validate_capture_flag(mov, to_bb) {
            return false;
        }

        if mov.flags() == MoveFlags::DoublePawn && piece != Piece::Pawn {
            return false;
        }

        match piece {
            Piece::Pawn => {
                let promo_rank = if us == Color::White { 7 } else { 0 };
                if to.rank() == promo_rank {
                    return false;
                }

                let forward_dir = us.forward();
                let start_rank = if us == Color::White { 1 } else { 6 };
                let single_push_sq = unsafe { from.shift(forward_dir) };

                if mov.is_capture() {
                    pawn_attacks(from, us) & to_bb != 0
                } else if to == single_push_sq {
                    mov.flags() == MoveFlags::Quiet
                } else if from.rank() == start_rank {
                    let double_push_sq = unsafe { single_push_sq.shift(forward_dir) };
                    to == double_push_sq
                        && (single_push_sq.bitboard() & occupied) == 0
                        && mov.flags() == MoveFlags::DoublePawn
                } else {
                    false
                }
            }
            Piece::Knight => knight_attacks(from) & to_bb != 0,
            Piece::Bishop => bishop_attacks(from, occupied) & to_bb != 0,
            Piece::Rook => rook_attacks(from, occupied) & to_bb != 0,
            Piece::Queen => queen_attacks(from, occupied) & to_bb != 0,
            Piece::King => king_attacks(from) & to_bb != 0,
        }
    }

    /// Fills check_squares and discovered_blockers for the current position.
    /// Not inlined: inside gives_check it makes every call save six callee-saved registers
    #[inline(never)]
    fn compute_check_info(&self) {
        let us = self.to_play;
        let them = !us;
        let ksq = self.king_sq(them);
        let occ = self.occupied();

        let mut squares = [0; Piece::NB];
        squares[Piece::Pawn as usize] = pawn_attacks(ksq, them);
        squares[Piece::Knight as usize] = knight_attacks(ksq);
        squares[Piece::Bishop as usize] = bishop_attacks(ksq, occ);
        squares[Piece::Rook as usize] = rook_attacks(ksq, occ);
        squares[Piece::Queen as usize] =
            squares[Piece::Bishop as usize] | squares[Piece::Rook as usize];

        let discovered_blockers = self.blockers(ksq, us, us);
        let state = self.state();
        for (cell, bb) in state.check_squares.iter().zip(squares) {
            cell.set(bb);
        }
        state.discovered_blockers.set(discovered_blockers);
    }

    #[inline]
    pub fn legal(&self, mov: Move) -> bool {
        // Rare paths are tail calls and set_pinned runs before anything is decoded, so the common
        // path keeps nothing live across a call and saves no registers
        if unsafe { self.piece_type_at(mov.from()) } == Piece::King {
            return self.legal_king(mov);
        }
        if mov.flags() == MoveFlags::EnPassant {
            return self.legal_en_passant(mov);
        }

        if self.state().pinned.get() == UNCOMPUTED {
            self.set_pinned();
        }

        let from = mov.from();
        if from.bitboard() & self.state().pinned.get() == 0 {
            return true;
        }

        // It can only move legally along the pin ray
        let ray_mask = bb_line(self.king_sq(self.to_play), from);
        (mov.to().bitboard() & ray_mask) != 0
    }

    #[inline]
    pub fn gives_check(&self, mov: Move) -> bool {
        // Check first: values computed before the call would be kept in callee-saved registers,
        // which every call then has to save and restore
        let state = self.state();
        if state.discovered_blockers.get() == UNCOMPUTED {
            self.compute_check_info();
        }

        let from = mov.from();
        let to = mov.to();
        let from_bb = from.bitboard();
        let to_bb = to.bitboard();
        let us = self.to_play;
        let enemy_king_sq = self.king_sq(!us);
        let enemy_king_bb = enemy_king_sq.bitboard();

        let moved_piece = unsafe { self.piece_type_at(from) };
        if state.check_squares[moved_piece as usize].get() & to_bb != 0 {
            return true;
        }

        if state.discovered_blockers.get() & from_bb != 0
            && bb_line(from, enemy_king_sq) & to_bb == 0
        {
            return true;
        }

        let flags = mov.flags();

        if mov.is_castle() {
            let (rook_from, rook_to) = flags.castling_rook_squares(us);
            let occupied = self.occupied();
            let occ_after =
                (occupied ^ (from_bb | rook_from.bitboard())) | (to_bb | rook_to.bitboard());
            return (attacks::rook_attacks(rook_to, occ_after) & enemy_king_bb) != 0;
        }

        if flags == MoveFlags::EnPassant {
            let captured_pawn_sq = mov.capture_square(us);
            let occupied = self.occupied();
            let occ_after = (occupied ^ from_bb ^ captured_pawn_sq.bitboard()) | to_bb;
            let our_rooks_queens =
                (self.pieces(Piece::Rook) | self.pieces(Piece::Queen)) & self.colors(us) & !from_bb;
            let our_bishops_queens = (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen))
                & self.colors(us)
                & !from_bb;
            return ((attacks::rook_attacks(enemy_king_sq, occ_after) & our_rooks_queens)
                | (attacks::bishop_attacks(enemy_king_sq, occ_after) & our_bishops_queens))
                != 0;
        }

        if mov.is_promotion() {
            let occ_after = (self.occupied() ^ from_bb) | to_bb;
            let promo_piece = unsafe { mov.promotion_piece().unwrap_unchecked() };
            return (attacks::piece_attack(promo_piece, to, us, occ_after) & enemy_king_bb) != 0;
        }

        false
    }

    /// Makes the current state the root, so permanent moves never overflow the stack.
    /// Earlier states are discarded and can no longer be undone
    #[inline]
    pub fn adopt_state(&mut self) {
        self.state_array.swap(0, self.state_index as usize);
        self.state_index = 0;
    }

    pub fn play_move(&mut self, mov: Move) {
        self.make_move(mov);
        self.adopt_state();
    }

    #[inline]
    pub fn make_move(&mut self, mov: Move) {
        self.make_move_fast(mov, self.gives_check(mov));
    }

    #[inline]
    pub fn make_move_fast(&mut self, mov: Move, mov_gives_check: bool) {
        let from = mov.from();
        let to = mov.to();
        let flags = mov.flags();
        let us = self.to_play;
        let them = !us;

        debug_assert!(self.piece_at(from).is_some());

        self.push_state();

        let is_capture = mov.is_capture();
        let captured_piece = if is_capture {
            let piece = unsafe {
                self.remove_piece::<true>(mov.capture_square(us))
                    .unwrap_unchecked()
            };
            Some(piece)
        } else {
            None
        };

        let mut piece = self.move_piece::<true>(from, to);
        let is_pawn = piece.piece() == Piece::Pawn;

        if mov.is_promotion() {
            let promo_piece = unsafe { mov.promotion_piece().unwrap_unchecked() };

            self.remove_piece::<true>(to);

            piece = ColoredPiece::new(promo_piece, us);
            self.add_piece::<true>(to, piece);
        }

        if mov.is_castle() {
            let (rook_from, rook_to) = flags.castling_rook_squares(us);
            self.move_piece::<true>(rook_from, rook_to);
        }

        let ep_target = (flags == MoveFlags::DoublePawn)
            .then(|| unsafe { to.shift(us.backward()) })
            .filter(|&sq| pawn_attacks(sq, us) & self.color_piece(Piece::Pawn, them) != 0);
        let mut hash = self.state().hash;
        let state = self.state_mut();

        hash ^= ZOBRIST_KEYS.castling(state.castling_rights);
        state.castling_rights &= CastlingRights::mask_for_move(from, to);
        hash ^= ZOBRIST_KEYS.castling(state.castling_rights);

        if let Some(en_passsant) = state.en_passant_target_sq.take() {
            hash ^= ZOBRIST_KEYS.en_passant(en_passsant);
        }
        if let Some(sq) = ep_target {
            hash ^= ZOBRIST_KEYS.en_passant(sq);
            state.en_passant_target_sq = Some(sq);
        }

        if is_pawn || is_capture {
            state.half_move_clock = 0;
        } else {
            state.half_move_clock += 1;
        }

        state.hash = hash ^ ZOBRIST_KEYS.side();
        state.captured_piece = captured_piece;
        self.ply += 1;
        self.to_play = them;
        if mov_gives_check {
            self.set_checkers();
        }
    }

    #[inline]
    pub fn undo_move(&mut self, mov: Move) {
        let from = mov.from();
        let to = mov.to();
        let flags = mov.flags();
        let us = !self.to_play;

        debug_assert!(self.piece_at(to).is_some());

        if mov.is_promotion() {
            self.remove_piece::<false>(to);
            self.add_piece::<false>(from, ColoredPiece::new(Piece::Pawn, us));
        } else {
            self.move_piece::<false>(to, from);
        }

        if mov.is_castle() {
            let (rook_from, rook_to) = flags.castling_rook_squares(us);
            self.move_piece::<false>(rook_to, rook_from);
        }

        let is_capture = mov.is_capture();
        if is_capture {
            debug_assert!(self.state().captured_piece.is_some());
            // Safety: this is a capture
            let captured_piece = unsafe { self.state().captured_piece.unwrap_unchecked() };
            self.add_piece::<false>(mov.capture_square(us), captured_piece);
        }

        self.pop_state();

        self.ply -= 1;
        self.to_play = us;
    }

    #[inline]
    pub fn make_null_move(&mut self) {
        debug_assert!(self.state().checkers == 0);

        self.push_state();

        self.ply += 1;
        self.to_play = !self.to_play;
        let state = self.state_mut();
        state.half_move_clock += 1;
        state.hash ^= ZOBRIST_KEYS.side();
        if let Some(en_passant) = state.en_passant_target_sq.take() {
            state.hash ^= ZOBRIST_KEYS.en_passant(en_passant);
        }

        self.set_pinned();
    }

    #[inline]
    pub fn undo_null_move(&mut self) {
        self.pop_state();

        self.ply -= 1;
        self.to_play = !self.to_play;
    }

    #[inline]
    pub fn has_non_pawn_material(&self, color: Color) -> bool {
        (self.colors(color) & !(self.pieces(Piece::Pawn) | self.pieces(Piece::King))) != 0
    }

    pub fn has_insufficient_material(&self) -> bool {
        // If there are pawns, rooks, or queens, mate is possible
        let majors_and_pawns =
            self.pieces(Piece::Pawn) | self.pieces(Piece::Rook) | self.pieces(Piece::Queen);

        if majors_and_pawns != 0 {
            return false;
        }

        let knights = self.pieces(Piece::Knight);
        let bishops = self.pieces(Piece::Bishop);
        let minors = knights | bishops;

        // King vs King
        if minors == 0 {
            return true;
        }

        // King + Minor vs King (K+N vs K or K+B vs K)
        if bb_only_one(minors) {
            return true;
        }

        // King + Bishop vs King + Bishop on the same color squares
        if knights == 0 && bb_only_one(minors & minors.wrapping_sub(1)) {
            let white_bishops = bishops & self.colors(Color::White);
            let black_bishops = bishops & self.colors(Color::Black);
            if bb_only_one(white_bishops) && bb_only_one(black_bishops) {
                return (white_bishops & LIGHT_SQUARES != 0)
                    == (black_bishops & LIGHT_SQUARES != 0);
            }
        }

        false
    }
}

impl Board {
    #[inline(always)]
    fn state(&self) -> &BoardState {
        unsafe { self.state_array.get_unchecked(self.state_index as usize) }
    }

    #[inline(always)]
    fn state_mut(&mut self) -> &mut BoardState {
        unsafe {
            self.state_array
                .get_unchecked_mut(self.state_index as usize)
        }
    }

    fn push_state(&mut self) {
        debug_assert!(self.state_index + 1 < self.state_array.len() as u16);

        let prev = self.state();
        let (hash, castling_rights, en_passant_target_sq, half_move_clock) = (
            prev.hash,
            prev.castling_rights,
            prev.en_passant_target_sq,
            prev.half_move_clock,
        );

        self.state_index += 1;
        let st = self.state_mut();
        // Copied then updated by make_move
        st.hash = hash;
        st.castling_rights = castling_rights;
        st.en_passant_target_sq = en_passant_target_sq;
        st.half_move_clock = half_move_clock;
        // Per-position, recomputed
        st.captured_piece = None;
        st.checkers = 0;
        st.pinned.set(UNCOMPUTED);
        st.discovered_blockers.set(UNCOMPUTED);
    }

    #[inline(always)]
    fn pop_state(&mut self) {
        debug_assert!(self.state_index > 0);
        self.state_index -= 1;
    }

    #[inline(always)]
    fn validate_capture_flag(&self, mov: Move, to_bb: u64) -> bool {
        if mov.is_capture() {
            (to_bb & self.colors(!self.to_play)) != 0
        } else {
            (to_bb & self.occupied()) == 0
        }
    }

    fn blockers(&self, king_sq: Sq, slider_color: Color, blocker_color: Color) -> u64 {
        let slider_pieces = self.colors(slider_color);
        let rooks = (self.pieces(Piece::Rook) | self.pieces(Piece::Queen)) & slider_pieces;
        let bishops = (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen)) & slider_pieces;

        let snipers =
            (rook_xray_attacks(king_sq) & rooks) | (bishop_xray_attacks(king_sq) & bishops);
        let occupancy = self.occupied() ^ snipers;

        let mut result = 0;
        for_each_bit!(sq in snipers => {
            let between = bb_between(king_sq, sq) & occupancy;
            if bb_only_one(between) {
                result |= between & self.colors(blocker_color);
            }
        });
        result
    }

    #[inline]
    pub(super) fn set_checkers(&mut self) {
        self.state_mut().checkers =
            self.generate_attackers(self.king_sq(self.to_play), !self.to_play, self.occupied());
    }

    #[inline(never)]
    pub(super) fn set_pinned(&self) {
        let us = self.to_play;
        let king_sq = self.king_sq(us);

        let enemy = self.colors(!us);
        let our = self.colors(us);
        let occupied = self.occupied();

        let enemy_rooks = (self.pieces(Piece::Rook) | self.pieces(Piece::Queen)) & enemy;
        let enemy_bishops = (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen)) & enemy;

        // generate attacks stopping ONLY at enemy pieces
        let potential_pinners = (rook_attacks(king_sq, enemy) & enemy_rooks)
            | (bishop_attacks(king_sq, enemy) & enemy_bishops);

        let mut pinned = 0;

        for_each_bit!(pinner_sq in potential_pinners => {
            let ray = bb_between(king_sq, pinner_sq);
            let blockers_on_ray = ray & occupied;

            // If there is exactly one piece on the ray and it's ours
            if bb_only_one(blockers_on_ray) {
                pinned |= blockers_on_ray & our;
            }
        });

        self.state().pinned.set(pinned);
    }

    // HASH = false skips the Zobrist update, for undo_move where the state is discarded
    #[inline]
    pub(super) fn add_piece<const HASH: bool>(&mut self, sq: Sq, piece: ColoredPiece) {
        self.mailbox[sq as usize] = Some(piece);
        let bitboard = sq.bitboard();
        *self.colors_mut(piece.color()) |= bitboard;
        *self.pieces_mut(piece.piece()) |= bitboard;
        if HASH {
            self.state_mut().hash ^= ZOBRIST_KEYS.piece(sq, piece);
        }
    }

    #[inline(always)]
    fn move_piece<const HASH: bool>(&mut self, from: Sq, to: Sq) -> ColoredPiece {
        let piece = unsafe { self.mailbox[from as usize].take().unwrap_unchecked() };
        self.mailbox[to as usize] = Some(piece);

        let move_bb = from.bitboard() ^ to.bitboard();
        *self.colors_mut(piece.color()) ^= move_bb;
        *self.pieces_mut(piece.piece()) ^= move_bb;
        if HASH {
            self.state_mut().hash ^=
                ZOBRIST_KEYS.piece(from, piece) ^ ZOBRIST_KEYS.piece(to, piece);
        }

        piece
    }

    #[inline]
    fn remove_piece<const HASH: bool>(&mut self, sq: Sq) -> Option<ColoredPiece> {
        let piece = self.mailbox[sq as usize].take()?;
        let bitboard = sq.bitboard();
        *self.colors_mut(piece.color()) &= !bitboard;
        *self.pieces_mut(piece.piece()) &= !bitboard;
        if HASH {
            self.state_mut().hash ^= ZOBRIST_KEYS.piece(sq, piece);
        }

        Some(piece)
    }

    #[inline(never)]
    fn legal_en_passant(&self, mov: Move) -> bool {
        let us = self.to_play;
        let captured_pawn_sq = mov.capture_square(us);
        let occ = ((self.occupied() ^ mov.from().bitboard()) ^ captured_pawn_sq.bitboard())
            | mov.to().bitboard();
        let attackers = self.generate_attackers(self.king_sq(us), !us, occ);

        // Mask out the captured pawn
        (attackers & !captured_pawn_sq.bitboard()) == 0
    }

    #[inline(never)]
    fn legal_king(&self, mov: Move) -> bool {
        let occupied = self.occupied();
        let from = mov.from();
        let to = mov.to();
        let them = !self.to_play;

        if mov.is_castle() {
            for sq in mov.flags().castling_king_path(self.to_play) {
                if self.generate_attackers(sq, them, occupied) != 0 {
                    return false;
                }
            }
        }

        self.generate_attackers(to, them, (occupied ^ from.bitboard()) | to.bitboard()) == 0
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        for rank in (0..8u8).rev() {
            write!(f, "{} |", rank + 1)?;
            for file in 0..8u8 {
                let sq = Sq::new(file, rank).unwrap();
                let ch = self.mailbox[sq as usize].map_or(' ', |p| p.to_char());
                write!(f, " {} |", ch)?;
            }
            writeln!(f)?;
            writeln!(f, "  +---+---+---+---+---+---+---+---+")?;
        }
        writeln!(f, "    a   b   c   d   e   f   g   h")?;
        writeln!(f, "Side to move: {:?}", self.to_play)?;
        writeln!(f, "Castling:     {:?}", self.castling_rights())?;
        writeln!(
            f,
            "En passant:   {}",
            self.en_passant_target_sq()
                .map_or_else(|| "-".to_string(), |sq| sq.to_string()),
        )?;
        writeln!(f, "Half-move:    {}", self.half_move_clock())?;
        write!(f, "Ply:          {}", self.ply)
    }
}
