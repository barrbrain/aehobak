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

use crate::control::Aehobak as AehobakControl;
use crate::control::Bsdiff as BsdiffControl;
use std::io;
use std::io::Write;
use streamvbyte64::{Coder, Coder0124};

/// Encode bsdiff output, returning a compact representation.
pub fn encode<T: Write>(patch: &[u8], writer: &mut T) -> io::Result<()> {
    encode_internal(patch, writer)
}

fn encode_internal(patch: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let mut encoder = EncoderState::parse(patch);
    let encoder = encoder.freeze();
    encoder.write(writer, 0)
}

fn to_perm4(idx: u32) -> u32 {
    let mut buffer = [0u8; 4];
    let mut lo = idx;
    (buffer[4 - 4 as usize], lo) = {
        let q = lo * 43 >> 8;
        (q as u8, (lo - q * 6))
    };
    (lo, buffer[4 - 1 as usize]) = (lo, 0);
    (lo, buffer[4 - 2 as usize]) = (lo >> 1, lo as u8 & 1);
    buffer[4 - 3 as usize] = lo as u8;

    let mut choices = 0x3210u32;
    let mut result = 0u32;
    for value in buffer {
        let shift = 4 * value as u32;
        let upper = choices >> shift;
        choices ^= (upper ^ (upper >> 4)) << shift;
        result = result << 2 | (upper & 15);
    }
    result
}

pub struct EncoderState {
    literals: Vec<u8>,
    seeks: Vec<u32>,
    adds: Vec<u32>,
    copies: Vec<u32>,
    delta_skips: Vec<u32>,
    delta_diffs: Vec<u8>,
    add_cursor: usize,
    delta_cursor: usize,
    controls: usize,
}

impl EncoderState {
    pub fn new(len: usize) -> Self {
        let ops = len / 16; // An initial approximation
        Self {
            literals: Vec::with_capacity(ops),
            seeks: Vec::with_capacity(ops),
            adds: Vec::with_capacity(ops),
            copies: Vec::with_capacity(ops),
            delta_skips: Vec::with_capacity(ops),
            delta_diffs: Vec::with_capacity(ops),
            add_cursor: 0,
            delta_cursor: 0,
            controls: 0,
        }
    }

    pub fn parse(mut patch: &[u8]) -> Self {
        let mut encoder = EncoderState::new(patch.len());
        while 24 <= patch.len() {
            let control: AehobakControl = BsdiffControl::try_from(&patch[..24])
                .unwrap()
                .try_into()
                .unwrap();
            let (add, copy) = (control.add as usize, control.copy as usize);
            encoder.control(control);
            patch = &patch[24..];
            encoder.add_diffed(&patch[..add]);
            patch = &patch[add..];
            encoder.copy(&patch[..copy]);
            patch = &patch[copy..];
        }
        encoder
    }

    pub fn control(&mut self, control: AehobakControl) {
        control.encode((&mut self.adds, &mut self.copies, &mut self.seeks));
        self.controls += 1;
    }

    pub fn add(&mut self, old: &[u8], new: &[u8]) {
        let add = old.len();
        assert_eq!(add, new.len());
        for (i, delta) in new
            .iter()
            .zip(old)
            .map(|(n, o)| n.wrapping_sub(*o))
            .enumerate()
        {
            if delta != 0 {
                let skip = self.add_cursor + i - self.delta_cursor;
                self.delta_skips.push(skip.try_into().unwrap());
                self.delta_diffs.push(delta);
                self.delta_cursor += skip + 1;
            }
        }
        self.add_cursor += add;
    }

    pub fn add_diffed(&mut self, deltas: &[u8]) {
        for (i, &delta) in deltas.iter().enumerate() {
            if delta != 0 {
                let skip = self.add_cursor + i - self.delta_cursor;
                self.delta_skips.push(skip.try_into().unwrap());
                self.delta_diffs.push(delta);
                self.delta_cursor += skip + 1;
            }
        }
        self.add_cursor += deltas.len();
    }

    pub fn copy(&mut self, new: &[u8]) {
        self.literals.extend(new);
    }

    pub fn freeze(&mut self) -> &Self {
        let controls = self.controls;
        let padding = controls.wrapping_neg() % 4;
        self.seeks.resize(controls + padding, 0);
        self.adds.resize(controls + padding, 0);
        self.copies.resize(controls + padding, 0);

        let padding = self.delta_skips.len().wrapping_neg() % 4;
        self.delta_skips.resize(self.delta_skips.len() + padding, 0);

        &*self
    }

    pub fn write(&self, writer: &mut dyn Write, rank: usize) -> io::Result<()> {
        let coder = Coder0124::new();

        let (perm_out, perm_in) = {
            let (rank_out, rank_in) = (rank / 24, rank % 24);
            (to_perm4(rank_out as u32), to_perm4(rank_in as u32))
        };

        let mut u32_seq = Vec::with_capacity(
            self.adds.len() + self.copies.len() + self.delta_skips.len() + self.seeks.len(),
        );
        for shift in (0..8).step_by(2).rev() {
            match (perm_in >> shift) & 3 {
                0 => u32_seq.extend(&self.adds),
                1 => u32_seq.extend(&self.copies),
                2 => u32_seq.extend(&self.delta_skips),
                3 => u32_seq.extend(&self.seeks),
                _ => unreachable!(),
            }
        }

        let (tag_len, data_len) = Coder0124::max_compressed_bytes(u32_seq.len());
        let mut encoded = vec![0u8; tag_len + data_len];
        let (tags, data) = encoded.split_at_mut(tag_len);
        let data_len = coder.encode(&u32_seq, tags, data);
        let data = &data[..data_len];

        let mut prefix_vec = Vec::new();
        for shift in (0..8).step_by(2).rev() {
            match (perm_out >> shift) & 3 {
                0 => prefix_vec.push(self.literals.len() as u32),
                1 => prefix_vec.push(self.controls as u32),
                2 => prefix_vec.push(self.delta_diffs.len() as u32),
                3 => prefix_vec.push(data_len as u32),
                _ => unreachable!(),
            }
        }
        let mut prefix = [0u8; 17];
        let prefix_len = 1 + {
            let (tag, data) = prefix.as_mut_slice().split_at_mut(1);
            coder.encode(&prefix_vec, tag, data)
        };

        writer.write_all(&prefix[..prefix_len])?;
        for shift in (0..8).step_by(2).rev() {
            match (perm_out >> shift) & 3 {
                0 => writer.write_all(&self.literals)?,
                1 => writer.write_all(tags)?,
                2 => writer.write_all(&self.delta_diffs)?,
                3 => writer.write_all(data)?,
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
