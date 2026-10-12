use chess_core::prelude::*;

pub const EVAL_NONE: i16 = 30_001;
pub const INFINITY: i16 = 30_000;
pub const MATE_THRESHOLD: i16 = 29_000;

// Material values calibrated to NNUE scale (~400 / pawn)
const PIECE_VALUES: [i16; Piece::NB] = [
    400,  // Pawn
    1350, // Knight
    1450, // Bishop
    1750, // Rook
    2570, // Queen
    0,    // King
];

#[inline(always)]
pub const fn piece_value(piece: Piece) -> i16 {
    PIECE_VALUES[piece as usize]
}

pub const MAX_PLY: u16 = 64;
pub(super) const MAX_KILLER_MOVES: usize = 2;
pub(super) const MAX_HISTORY: i32 = 10_000;

pub(super) const TIME_CHECK_MASK: u64 = 4095;

pub(super) const NMP_EVAL_DIVISOR: i16 = 250;
pub(super) const NMP_MIN_REDUCTION: u8 = 3;
pub(super) const NMP_DEPTH_DIVISOR: u8 = 4;
pub(super) const NMP_MAX_EVAL_BONUS: u8 = 3;

// Margins are in NNUE eval units, where ~1 pawn ≈ 400 (the net's SCALE), not
// classical centipawns.
pub(super) const FUTILITY_MARGIN: i16 = piece_value(Piece::Pawn);
pub(super) const FUTILITY_MAX_DEPTH: u8 = 8;
pub(super) const FUTILITY_MIN_LEGAL_MOVES: usize = 3;

pub(super) const RFP_DEPTH: u8 = 6;
pub(super) const RFP_MARGIN_SLOPE: i16 = 200;
pub(super) const RFP_IMPROVING_BONUS: i16 = 200;
pub(super) const RFP_NO_TT_MARGIN: i16 = 50;

pub(super) const LMP_MAX_DEPTH: u8 = 5;
pub(super) const LMP_BASE: u16 = 5;
pub(super) const LMP_SLOPE: u16 = 2;

#[inline(always)]
pub(super) const fn lmp_threshold(depth: u8, improving: bool) -> usize {
    ((LMP_BASE + LMP_SLOPE * depth as u16 * depth as u16) / (2 - improving as u16)) as usize
}

pub(super) const HISTORY_PRUNING_DEPTH: u8 = 8;
pub(super) const HISTORY_PRUNING_MARGIN: i16 = 2250;

pub(super) const LMR_MIN_DEPTH: u8 = 2;
pub(super) const LMR_MIN_LEGAL_MOVES: usize = 1;
pub(super) const LMR_HISTORY_DIVISOR: i32 = 12000;

pub(super) const DELTA_MARGIN: i16 = 2 * piece_value(Piece::Pawn);

pub(super) const SE_DOUBLE_MARGIN: i16 = 8;
pub(super) const SE_TRIPLE_MARGIN: i16 = 76;
pub(super) const GLOBAL_DELTA_MARGIN: i16 = piece_value(Piece::Queen);

pub(super) const ASPIRATION_INITIAL_DELTA: i16 = 100;
pub(super) const ASPIRATION_FLUCTUATION: i16 = 3 * piece_value(Piece::Pawn);
pub(super) const ASPIRATION_MIN_DEPTH: u8 = 5;

// These are not based on piece values but on the PIECE_VALUES_SEE array
pub(super) const SEE_CAPTURE_MARGIN: i32 = -100;
pub(super) const CAPTURE_SEE_MAX_DEPTH: u8 = 8;
pub(super) const SEE_QSEARCH_MARGIN: i32 = -100;
pub(super) const QUIET_SEE_MAX_DEPTH: u8 = 4;
pub(super) const QUIET_SEE_COEFF: i32 = -20;

pub(super) static LMR_TABLE: std::sync::LazyLock<LmrTable> = std::sync::LazyLock::new(|| {
    let mut table = [[(0, 0); MAX_PLY as usize]; MAX_PLY as usize];

    let mut depth = 1;
    while depth < MAX_PLY {
        let mut moves = 1;
        while moves < MAX_PLY {
            let r = (0.87 + (depth as f64).ln() * (moves as f64).ln() / 1.68) as u8;
            table[depth as usize][moves as usize] = (r, r.saturating_sub(1));
            moves += 1;
        }

        depth += 1;
    }

    table
});

pub(super) type LmrTable = [[(u8, u8); MAX_PLY as usize]; MAX_PLY as usize];
