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

pub use decode::decode;
pub use encode::encode;

#[cfg(test)]
mod tests {
    use super::*;
    use bsdiff;
    use quickcheck::quickcheck;

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
    }
}
