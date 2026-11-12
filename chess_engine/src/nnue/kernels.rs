//! SIMD kernels for the feature transformer: accumulator updates fused with the
//! SCReLU output dot product.

use std::mem::MaybeUninit;

use super::{
    constants::*,
    network::{NNUE, SideAccumulator},
};
use fearless_simd::{Simd, i8x64, i16x32, i32x16, prelude::*};

#[cfg(target_feature = "avx512bw")]
const PARALLEL_SUMS: usize = 4;
#[cfg(not(target_feature = "avx512bw"))]
const PARALLEL_SUMS: usize = 2;

pub(super) struct Delta<const N: usize> {
    adds: [MaybeUninit<usize>; N],
    subs: [MaybeUninit<usize>; N],
    n_adds: usize,
    n_subs: usize,
}

impl<const N: usize> Delta<N> {
    #[inline(always)]
    pub(super) fn new() -> Self {
        Self {
            adds: [const { MaybeUninit::uninit() }; N],
            subs: [const { MaybeUninit::uninit() }; N],
            n_adds: 0,
            n_subs: 0,
        }
    }

    #[inline(always)]
    pub(super) fn add(&mut self, index: usize) {
        debug_assert!(self.n_adds < self.adds.len());
        // SAFETY: callers never exceed N adds: a move adds at most 2 features, and a
        // Finny refresh at most 32 (one per piece). Checked by the debug_assert above.
        unsafe {
            self.adds.get_unchecked_mut(self.n_adds).write(index);
        }
        self.n_adds += 1;
    }

    #[inline(always)]
    pub(super) fn sub(&mut self, index: usize) {
        debug_assert!(self.n_subs < self.subs.len());
        // SAFETY: as in `add`, at most 2 subs per move and 32 per Finny refresh.
        unsafe {
            self.subs.get_unchecked_mut(self.n_subs).write(index);
        }
        self.n_subs += 1;
    }

    #[inline(always)]
    pub(super) fn inverted(self) -> Self {
        Self {
            adds: self.subs,
            subs: self.adds,
            n_adds: self.n_subs,
            n_subs: self.n_adds,
        }
    }

    #[inline(always)]
    fn adds(&self) -> &[usize] {
        // SAFETY: n_adds <= N (see `add`), and entries 0..n_adds were written by `add`.
        unsafe { self.adds.get_unchecked(..self.n_adds).assume_init_ref() }
    }

    #[inline(always)]
    fn subs(&self) -> &[usize] {
        // SAFETY: n_subs <= N (see `sub`), and entries 0..n_subs were written by `sub`.
        unsafe { self.subs.get_unchecked(..self.n_subs).assume_init_ref() }
    }
}

#[inline(always)]
fn screlu_accumulate<S: Simd>(simd: S, v: i16x32<S>, out_w: &[i16], sum: &mut i32x16<S>) {
    let clamped = v.clamp(i16x32::splat(simd, 0), i16x32::splat(simd, QA as i16));
    let w = i16x32::from_slice(simd, out_w);
    *sum += clamped.pairwise_dot_product(clamped * w);
}

#[inline(always)]
fn blocks<S: Simd>() -> impl Iterator<Item = usize> {
    (0..HIDDEN_SIZE).step_by(PARALLEL_SUMS * i16x32::<S>::LEN)
}

#[inline(always)]
fn lanes<S: Simd>(b: usize, k: usize) -> std::ops::Range<usize> {
    let n = i16x32::<S>::LEN;
    b + k * n..b + (k + 1) * n
}

#[inline(always)]
fn load<S: Simd>(simd: S, w: &[i16], b: usize) -> [i16x32<S>; PARALLEL_SUMS] {
    std::array::from_fn(|k| i16x32::from_slice(simd, &w[lanes::<S>(b, k)]))
}

/// Adds and subtracts the delta's feature rows for the block starting at `b`
#[inline(always)]
fn apply_delta<S: Simd, const N: usize>(
    simd: S,
    acc: &mut [i16x32<S>; PARALLEL_SUMS],
    delta: &Delta<N>,
    bucket: usize,
    b: usize,
) {
    // One i8x64 load covers two accumulator registers
    let widen = |index: usize, k: usize| {
        let row = &NNUE.feature_weights(index, bucket)[b + k * 32..b + (k + 2) * 32];
        simd.widen_i8x64(i8x64::from_slice(simd, row))
    };
    for &add in delta.adds() {
        for k in (0..PARALLEL_SUMS).step_by(2) {
            let (lo, hi) = widen(add, k);
            acc[k] += lo;
            acc[k + 1] += hi;
        }
    }
    for &sub in delta.subs() {
        for k in (0..PARALLEL_SUMS).step_by(2) {
            let (lo, hi) = widen(sub, k);
            acc[k] -= lo;
            acc[k + 1] -= hi;
        }
    }
}

#[inline(always)]
pub(super) fn screlu_dot<S: Simd>(
    simd: S,
    acc: &SideAccumulator,
    out_w: &[i16; HIDDEN_SIZE],
) -> i32x16<S> {
    let mut sums = [i32x16::splat(simd, 0); PARALLEL_SUMS];
    for b in blocks::<S>() {
        let v = load(simd, acc, b);
        for k in 0..PARALLEL_SUMS {
            screlu_accumulate(simd, v[k], &out_w[lanes::<S>(b, k)], &mut sums[k]);
        }
    }
    sums[1..].iter().fold(sums[0], |a, &b| a + b)
}

/// `dst = src + delta`
#[inline(always)]
pub(super) fn apply_delta_copy<S: Simd, const N: usize>(
    simd: S,
    src: &SideAccumulator,
    dst: &mut SideAccumulator,
    delta: &Delta<N>,
    bucket: usize,
) {
    for b in blocks::<S>() {
        let mut acc = load(simd, src, b);
        apply_delta(simd, &mut acc, delta, bucket, b);
        for k in 0..PARALLEL_SUMS {
            acc[k].store_slice(&mut dst[lanes::<S>(b, k)]);
        }
    }
}

/// `dst = src + delta`, fused with the SCReLU dot of `dst`
#[inline(always)]
pub(super) fn apply_delta_screlu_dot<S: Simd, const N: usize>(
    simd: S,
    src: &SideAccumulator,
    dst: &mut SideAccumulator,
    delta: &Delta<N>,
    bucket: usize,
    out_w: &[i16; HIDDEN_SIZE],
) -> i32x16<S> {
    let mut sum = i32x16::splat(simd, 0);
    for b in blocks::<S>() {
        let mut acc = load(simd, src, b);
        apply_delta(simd, &mut acc, delta, bucket, b);
        for k in 0..PARALLEL_SUMS {
            acc[k].store_slice(&mut dst[lanes::<S>(b, k)]);
            screlu_accumulate(simd, acc[k], &out_w[lanes::<S>(b, k)], &mut sum);
        }
    }
    sum
}

/// `acc += delta` in place and copied to `copy`, fused with the SCReLU dot of the result
#[inline(always)]
pub(super) fn apply_delta_in_place_screlu_dot<S: Simd, const N: usize>(
    simd: S,
    acc: &mut SideAccumulator,
    copy: &mut SideAccumulator,
    delta: &Delta<N>,
    bucket: usize,
    out_w: &[i16; HIDDEN_SIZE],
) -> i32x16<S> {
    let mut sum = i32x16::splat(simd, 0);
    for b in blocks::<S>() {
        let mut v = load(simd, acc, b);
        apply_delta(simd, &mut v, delta, bucket, b);
        for k in 0..PARALLEL_SUMS {
            v[k].store_slice(&mut acc[lanes::<S>(b, k)]);
            v[k].store_slice(&mut copy[lanes::<S>(b, k)]);
            screlu_accumulate(simd, v[k], &out_w[lanes::<S>(b, k)], &mut sum);
        }
    }
    sum
}
