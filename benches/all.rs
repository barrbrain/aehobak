/*-
 * Copyright 2025 David Michael Barr
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted providing that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
 * STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
 * IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#[path = "../tests/data.rs"]
mod data;

use gungraun::{library_benchmark, library_benchmark_group, main};
use rand_xoshiro::rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use std::hint::black_box;

fn patch_inputs() -> (Vec<u8>, Box<[u8]>, Vec<u8>, Vec<u8>) {
    let patch = data::ref_patch();
    let old = vec![0u8; 524288];
    let new = Vec::with_capacity(524288);
    let bspatch = Vec::with_capacity(524288);
    (old, patch, new, bspatch)
}

#[library_benchmark]
#[bench::small(setup = patch_inputs)]
fn memcpy(images: (Vec<u8>, Box<[u8]>, Vec<u8>, Vec<u8>)) {
    let (old, _patch, mut new, _bspatch) = images;
    black_box(new.extend(&old));
}

#[library_benchmark]
#[bench::small(setup = patch_inputs)]
fn aehobak_patch(images: (Vec<u8>, Box<[u8]>, Vec<u8>, Vec<u8>)) {
    let (old, patch, mut new, _bspatch) = images;
    black_box(aehobak::patch(&old, &patch, &mut new).unwrap());
}

#[library_benchmark]
#[bench::small(setup = patch_inputs)]
fn aehobak_decode_bspatch_patch(images: (Vec<u8>, Box<[u8]>, Vec<u8>, Vec<u8>)) {
    let (old, patch, mut new, mut bspatch) = images;
    black_box({
        aehobak::decode(&mut &*patch, &mut bspatch).unwrap();
        bsdiff::patch(&old, &mut &*bspatch, &mut new).unwrap();
    });
}

library_benchmark_group!(
    name = patch;
    compare_by_id = true;
    benchmarks = memcpy, aehobak_patch, aehobak_decode_bspatch_patch
);

fn diff_inputs() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let patch = data::ref_patch();
    let mut new = Vec::with_capacity(524288);
    let mut rng = Xoshiro256Plus::seed_from_u64(0xeba2fa67e5a81121);
    let mut old = vec![0u8; 524288];
    rng.fill_bytes(&mut old);
    aehobak::patch(&old, &mut &*patch, &mut new).unwrap();
    let bspatch = Vec::with_capacity(524288);
    let patch = Vec::with_capacity(524288);
    (old, new, bspatch, patch)
}

#[library_benchmark]
#[bench::small(setup = diff_inputs)]
fn aehobak_diff(images: (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)) -> usize {
    let (old, new, _bspatch, mut patch) = images;
    black_box(aehobak::diff(&old, &new, &mut patch).unwrap());
    patch.len()
}

#[library_benchmark]
#[bench::small(setup = diff_inputs)]
fn bsdiff_diff_aehobak_encode(images: (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)) -> usize {
    let (old, new, mut bspatch, mut patch) = images;
    black_box({
        bsdiff::diff(&old, &new, &mut bspatch).unwrap();
        aehobak::encode(&bspatch, &mut patch).unwrap();
    });
    patch.len()
}

library_benchmark_group!(
    name = diff;
    compare_by_id = true;
    benchmarks = aehobak_diff, bsdiff_diff_aehobak_encode
);

main!(library_benchmark_groups = patch, diff);
