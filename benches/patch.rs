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

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let patch = data::ref_patch();
    let old = vec![0; 524288];
    let mut bspatch = Vec::with_capacity(524288);
    let mut new = Vec::with_capacity(524288);

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Bytes(new.capacity() as u64));
    group.bench_function("memcpy", |b| {
        b.iter(|| {
            new.clear();
            new.extend(black_box(&old));
        })
    });
    group.bench_function("patch", |b| {
        b.iter(|| {
            new.clear();
            aehobak::patch(
                black_box(&old),
                black_box(&mut &*patch),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.bench_function("bspatch", |b| {
        bspatch.clear();
        aehobak::decode(black_box(&mut &*patch), black_box(&mut bspatch)).unwrap();
        b.iter(|| {
            new.clear();
            bsdiff::patch(
                black_box(&old),
                black_box(&mut bspatch.as_slice()),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.bench_function("decode-bspatch", |b| {
        b.iter(|| {
            new.clear();
            bspatch.clear();
            aehobak::decode(black_box(&mut &*patch), black_box(&mut bspatch)).unwrap();
            bsdiff::patch(
                black_box(&old),
                black_box(&mut bspatch.as_slice()),
                black_box(&mut new),
            )
            .unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
