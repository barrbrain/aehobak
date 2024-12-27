/*-
 * Copyright 2024 David Michael Barr
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

#![doc = include_str!("../README.md")]

mod control;
mod decode;
mod encode;
mod patch;

pub use decode::decode;
pub use encode::encode;
pub use patch::patch;

#[cfg(test)]
mod tests {
    use super::*;
    use bsdiff;
    use quickcheck::quickcheck;
    use std::collections::LinkedList;

    quickcheck! {
        fn round_trip(old: Vec<u8>, new: Vec<u8>) -> bool {
            let mut patch = Vec::new();
            let mut encoded = Vec::new();
            let mut decoded = Vec::new();
            bsdiff::diff(&old, &new, &mut patch).unwrap();
            encode(&patch, &mut encoded).unwrap();
            decode(&mut encoded.as_slice(), &mut decoded).unwrap();
            decoded == patch
        }

        fn replace_one(old: Vec<u8>, idx: usize) -> bool {
            let mut new = old.clone();
            if !new.is_empty() {
                let idx = idx % new.len();
                new[idx] = new[idx].wrapping_add(1);
            }
            let mut patch = Vec::new();
            let mut encoded = Vec::new();
            let mut decoded = Vec::new();
            bsdiff::diff(&old, &new, &mut patch).unwrap();
            encode(&patch, &mut encoded).unwrap();
            decode(&mut encoded.as_slice(), &mut decoded).unwrap();
            decoded == patch
        }

        fn direct_patch(old: Vec<u8>, idx: usize) -> bool {
            let mut new = old.clone();
            if !new.is_empty() {
                let idx = idx % new.len();
                new[idx] = new[idx].wrapping_add(1);
            }
            let mut bspatch = Vec::new();
            let mut encoded = Vec::new();
            let mut result = Vec::new();
            bsdiff::diff(&old, &new, &mut bspatch).unwrap();
            encode(&bspatch, &mut encoded).unwrap();
            patch(&old, &encoded, &mut result).unwrap();
            result == new
        }

        fn arbitrary_patch(skeleton: LinkedList<(u8,u8,i8)>, period: u8) -> bool {
            use std::io::ErrorKind::{InvalidData, UnexpectedEof};
            let (bspatch, old_len) = gen_bspatch(skeleton, period);
            let mut encoded = Vec::new();
            let mut result = Vec::new();
            let old = vec![0; old_len];
            encode(&bspatch, &mut encoded).unwrap();
            match patch(&old, &encoded, &mut result) {
                Err(e) if e.kind() == InvalidData => true,
                Err(e) if e.kind() == UnexpectedEof => true,
                Ok(_) => true,
                _ => false,
            }
        }
    }

    fn gen_bspatch(skeleton: LinkedList<(u8, u8, i8)>, period: u8) -> (Vec<u8>, usize) {
        use crate::control::{Aehobak, Bsdiff};
        let mut bspatch = Vec::new();
        let mut diffs = 0;
        let mut old_len = 0;
        let mut cursor = 0;
        for (add, copy, seek) in skeleton {
            let (add, copy, seek) = (add as u32, copy as u32, seek as i32);
            let seek = (seek << 1 ^ seek >> 31) as u32;
            let control: Bsdiff =
                (&Aehobak::try_from([add, copy, seek].as_slice()).unwrap()).into();
            control.encode(&mut bspatch);
            for _ in 0..add {
                bspatch.push((diffs % (1 + period as usize) == 0) as u8);
                diffs += 1;
            }
            cursor += add as usize;
            old_len = old_len.max(cursor);
            cursor = (cursor as i64 + seek as i64).max(0) as usize;
            bspatch.resize(bspatch.len() + copy as usize, 0);
        }
        (bspatch, old_len)
    }
}
