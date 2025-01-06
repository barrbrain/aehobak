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

#![no_main]

use aehobak::{encode, gen_bspatch, patch};
use libfuzzer_sys::fuzz_target;
use std::collections::LinkedList;
use std::io::ErrorKind::{InvalidData, UnexpectedEof};

fuzz_target!(|data: (LinkedList<(u8, u8, i8)>, u8, u8, i8, i8)| {
    let (skeleton, period, phase, old_slop, new_slop) = data;
    let (bspatch, old_len, new_len) = gen_bspatch(skeleton, period, phase);
    let old_len = (old_len as isize).saturating_add(old_slop as isize).max(1) as usize;
    let new_len = (new_len as isize).saturating_add(new_slop as isize).max(1) as usize;
    let mut encoded = Vec::new();
    let mut result = Vec::with_capacity(new_len);
    let old = vec![0; old_len];
    encode(&bspatch, &mut encoded).unwrap();
    let property = match patch(&old, &encoded, &mut result) {
        Err(e) if e.kind() == InvalidData => true,
        Err(e) if e.kind() == UnexpectedEof => true,
        Ok(_) => {
            let mut reference = Vec::new();
            bsdiff::patch(&old, &mut bspatch.as_slice(), &mut reference).unwrap();
            reference == result
        }
        _ => false,
    };
    assert!(property);
});
