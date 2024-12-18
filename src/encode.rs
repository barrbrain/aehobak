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

use std::io;
use std::io::Write;
use streamvbyte64::{Coder, Coder0124};

/// Encode bsdiff output, returning a compact representation.
pub fn encode<T: Write>(patch: &[u8], writer: &mut T) -> io::Result<()> {
    encode_internal(patch, writer)
}

#[inline]
fn to_zigzag(buf: &[u8; 8]) -> u64 {
    let y = u64::from_le_bytes(*buf);
    if y == 1 << 63 {
        0
    } else {
        (y << 1) - (y >> 63)
    }
}

#[derive(Default)]
struct Buffer {
    tags: Vec<u8>,
    data: Vec<u8>,
    buffer: [u32; 12],
    pending: usize,
    count: usize,
}

impl Buffer {
    fn push(&mut self, coder: &Coder0124, n: u32) {
        self.buffer[self.pending] = n;
        self.pending += 1;
        if self.pending == self.buffer.len() {
            self.flush(coder);
        }
    }

    fn flush(&mut self, coder: &Coder0124) {
        self.buffer[self.pending..].fill(0);
        self.count += self.pending;
        self.pending = 0;
        let (tag_new, data_new) = Coder0124::max_compressed_bytes(self.buffer.len());
        let tags_len = self.tags.len();
        let data_len = self.data.len();
        self.tags.resize(tags_len + tag_new, 0);
        self.data.resize(data_len + data_new, 0);
        let data_new = coder.encode(
            &self.buffer,
            &mut self.tags[tags_len..],
            &mut self.data[data_len..],
        );
        self.data.truncate(data_len + data_new);
    }
}

fn encode_internal(mut patch: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let coder = Coder0124::new();
    let mut copies = Buffer::default();
    let mut edits = Buffer::default();
    let mut deltas = Vec::<u8>::new();
    let mut literals = Vec::<u8>::new();

    let mut out = 0;
    let mut base = 0;
    while 24 <= patch.len() {
        let mix = u64::from_le_bytes(patch[..8].try_into().unwrap());
        let copy = u64::from_le_bytes(patch[8..16].try_into().unwrap());
        let seek = to_zigzag(patch[16..24].try_into().unwrap());
        patch = &patch[24..];
        copies.push(&coder, mix as u32);
        copies.push(&coder, copy as u32);
        copies.push(&coder, seek as u32);
        let (mix, copy) = (mix as usize, copy as usize);
        for (i, &b) in patch[..mix].iter().enumerate() {
            let abs = (out + i) as u32;
            if b != 0 {
                deltas.push(b);
                edits.push(&coder, abs - base);
                base = abs + 1;
            }
        }
        patch = &patch[mix..];
        literals.extend(&patch[..copy]);
        patch = &patch[copy..];
        out += mix + copy;
    }
    copies.flush(&coder);
    edits.flush(&coder);

    let mut buf = [0u8; 16];
    buf[..4].copy_from_slice(&(copies.count as u32).to_le_bytes());
    buf[4..8].copy_from_slice(&(edits.count as u32).to_le_bytes());
    buf[8..12].copy_from_slice(&(copies.data.len() as u32).to_le_bytes());
    buf[12..].copy_from_slice(&(edits.data.len() as u32).to_le_bytes());
    writer.write_all(&buf)?;

    writer.write_all(&copies.tags)?;
    writer.write_all(&edits.tags)?;
    writer.write_all(&copies.data)?;
    writer.write_all(&edits.data)?;
    writer.write_all(&deltas)?;
    writer.write_all(&literals)?;

    Ok(())
}
