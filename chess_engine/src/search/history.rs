use std::mem::MaybeUninit;

use super::params::{MAX_HISTORY, MAX_KILLER_MOVES};
use chess_core::prelude::*;

#[inline(always)]
fn gravity(entry: &mut i16, bonus: i32) {
    let current = *entry as i32;
    let abs = bonus.abs().min(MAX_HISTORY);
    *entry =
        (current + bonus - current * abs / MAX_HISTORY).clamp(-MAX_HISTORY, MAX_HISTORY) as i16;
}

#[inline(always)]
pub fn history_depth_bonus(depth: u8) -> i32 {
    (depth as i32 * depth as i32).min(MAX_HISTORY)
}

pub struct HistoryTable([[[i16; Sq::NB]; Sq::NB]; Color::NB]);

impl Default for HistoryTable {
    fn default() -> Self {
        Self([[[0; Sq::NB]; Sq::NB]; Color::NB])
    }
}

impl HistoryTable {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Decays entries carried over from the previous search.
    pub fn halve(&mut self) {
        for entry in self.0.as_flattened_mut().as_flattened_mut() {
            *entry /= 2;
        }
    }

    #[inline(always)]
    pub fn get(&self, side: Color, from: Sq, to: Sq) -> i16 {
        self.0[side as usize][from as usize][to as usize]
    }

    #[inline(always)]
    pub fn update_bonus(&mut self, side: Color, from: Sq, to: Sq, depth: u8) {
        gravity(
            &mut self.0[side as usize][from as usize][to as usize],
            history_depth_bonus(depth),
        );
    }

    #[inline(always)]
    pub fn update_malus(&mut self, side: Color, from: Sq, to: Sq, depth: u8) {
        gravity(
            &mut self.0[side as usize][from as usize][to as usize],
            -history_depth_bonus(depth),
        );
    }
}

pub type KillerMoves = [Move; MAX_KILLER_MOVES];

pub type ContHistKey = Option<(Piece, Sq)>;
pub const CONTHIST_LAYERS: usize = 2;
pub type ContHistKeys = [ContHistKey; CONTHIST_LAYERS];

pub struct ContinuationHistoryTable(pub [[[[i16; Sq::NB]; Piece::NB]; Sq::NB]; Piece::NB]);

impl Default for ContinuationHistoryTable {
    fn default() -> Self {
        Self([[[[0; Sq::NB]; Piece::NB]; Sq::NB]; Piece::NB])
    }
}

impl ContinuationHistoryTable {
    pub fn clear(&mut self) {
        self.0
            .as_flattened_mut()
            .as_flattened_mut()
            .as_flattened_mut()
            .fill(0);
    }

    pub fn halve(&mut self) {
        for entry in self
            .0
            .as_flattened_mut()
            .as_flattened_mut()
            .as_flattened_mut()
        {
            *entry /= 2;
        }
    }

    #[inline(always)]
    pub fn get(&self, key: ContHistKey, piece: Piece, to: Sq) -> i16 {
        match key {
            Some((prev_p, prev_to)) => {
                self.0[prev_p as usize][prev_to as usize][piece as usize][to as usize]
            }
            None => 0,
        }
    }

    #[inline(always)]
    pub fn score(&self, keys: &ContHistKeys, piece: Piece, to: Sq) -> i16 {
        let mut score: i32 = 0;
        for &key in keys {
            score += self.get(key, piece, to) as i32;
        }
        score.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }

    #[inline(always)]
    pub fn update(&mut self, key: ContHistKey, piece: Piece, to: Sq, bonus: i32) {
        if let Some((prev_p, prev_to)) = key {
            gravity(
                &mut self.0[prev_p as usize][prev_to as usize][piece as usize][to as usize],
                bonus,
            );
        }
    }
}

pub struct QuietsTried {
    arr: [MaybeUninit<Move>; 64],
    size: usize,
}

impl Default for QuietsTried {
    fn default() -> Self {
        Self {
            arr: [const { MaybeUninit::uninit() }; 64],
            size: 0,
        }
    }
}

impl QuietsTried {
    #[inline(always)]
    pub const fn as_slice(&self) -> &[Move] {
        unsafe { core::slice::from_raw_parts(self.arr.as_ptr().cast::<Move>(), self.size) }
    }

    #[inline(always)]
    pub const fn push_move(&mut self, mov: Move) {
        if self.size < self.arr.len() {
            self.arr[self.size].write(mov);
            self.size += 1;
        }
    }
}
