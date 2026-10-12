use std::mem::MaybeUninit;

use chess_core::prelude::*;

pub const MAX_MOVES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[repr(C, align(4))]
pub struct ScoredMove {
    pub mov: Move,
    pub score: i16,
}

impl ScoredMove {
    #[inline(always)]
    pub const fn new(mov: Move) -> Self {
        Self { mov, score: 0 }
    }

    #[inline(always)]
    pub const fn is_bad_capture(&self) -> bool {
        self.score < 0
    }
}

impl std::ops::Deref for ScoredMove {
    type Target = Move;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.mov
    }
}

impl PartialEq<Move> for ScoredMove {
    #[inline(always)]
    fn eq(&self, other: &Move) -> bool {
        self.mov == *other
    }
}

pub struct MoveList {
    moves: [MaybeUninit<ScoredMove>; MAX_MOVES],
    size: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            moves: [const { MaybeUninit::uninit() }; MAX_MOVES],
            size: 0,
        }
    }
}

impl MoveList {
    #[inline(always)]
    pub const fn as_ptr(&mut self) -> MoveListPtr {
        MoveListPtr(self.current_ptr())
    }

    #[inline]
    pub const fn update_size(&mut self, new_position: MoveListPtr) {
        let size = unsafe { new_position.0.offset_from(self.current_ptr()) };
        self.size = size as usize;
    }

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.size = 0;
    }

    #[inline]
    pub const fn as_slice(&self) -> &[ScoredMove] {
        unsafe { core::slice::from_raw_parts(self.moves.as_ptr().cast::<ScoredMove>(), self.size) }
    }

    #[inline]
    pub const fn as_slice_mut(&mut self) -> &mut [ScoredMove] {
        unsafe {
            core::slice::from_raw_parts_mut(self.moves.as_mut_ptr().cast::<ScoredMove>(), self.size)
        }
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.size
    }

    #[must_use]
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    const fn current_ptr(&mut self) -> *mut ScoredMove {
        (unsafe { self.moves.as_mut_ptr().add(self.size) }) as *mut ScoredMove
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct MoveListPtr(pub *mut ScoredMove);

impl MoveListPtr {
    #[inline(always)]
    pub const fn push(&mut self, from: Sq, to: Sq, flags: MoveFlags) {
        unsafe {
            self.0.write(ScoredMove::new(Move::new(from, to, flags)));
            self.0 = self.0.add(1);
        }
    }

    #[inline(always)]
    pub const fn push_promotions(&mut self, from: Sq, to: Sq, is_capture: bool) {
        let moves = if is_capture {
            [
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoCaptureQueen)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoCaptureRook)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoCaptureBishop)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoCaptureKnight)),
            ]
        } else {
            [
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoQueen)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoRook)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoBishop)),
                ScoredMove::new(Move::new(from, to, MoveFlags::PromoKnight)),
            ]
        };

        unsafe {
            let ptr = self.0 as *mut [ScoredMove; 4];
            ptr.write(moves);
            self.0 = self.0.add(4);
        }
    }
}
