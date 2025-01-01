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

fn encode_internal(mut patch: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let mut literals = Vec::<u8>::new();
    let mut seeks = Vec::<u32>::new();
    let mut adds = Vec::<u32>::new();
    let mut copies = Vec::<u32>::new();
    let mut delta_skips = Vec::<u32>::new();
    let mut delta_diffs = Vec::<u8>::new();

    let mut add_cursor = 0;
    let mut delta_cursor = 0;
    while 24 <= patch.len() {
        let control: AehobakControl = BsdiffControl::try_from(&patch[..24])
            .unwrap()
            .try_into()
            .unwrap();
        control.encode((&mut adds, &mut copies, &mut seeks));
        patch = &patch[24..];
        let (add, copy) = (control.add as usize, control.copy as usize);
        for (idx, &delta) in patch[..add].iter().enumerate() {
            if delta != 0 {
                let skip = add_cursor + idx - delta_cursor;
                delta_skips.push(skip.try_into().unwrap());
                delta_diffs.push(delta);
                delta_cursor += skip + 1;
            }
        }
        add_cursor += add;
        patch = &patch[add..];
        literals.extend(&patch[..copy]);
        patch = &patch[copy..];
    }

    let coder = Coder0124::new();

    let controls = adds.len();
    let padding = controls.wrapping_neg() % 4;
    seeks.resize(controls + padding, 0);
    adds.resize(controls + padding, 0);
    copies.resize(controls + padding, 0);

    let padding = delta_skips.len().wrapping_neg() % 4;
    delta_skips.resize(delta_skips.len() + padding, 0);

    let mut u32_seq = seeks;
    u32_seq.extend(&adds);
    u32_seq.extend(&copies);
    u32_seq.extend(&delta_skips);

    let (tag_len, data_len) = Coder0124::max_compressed_bytes(u32_seq.len());
    let mut encoded = vec![0u8; tag_len + data_len];
    let (tags, data) = encoded.split_at_mut(tag_len);
    let data_len = coder.encode(&u32_seq, tags, data);
    let data = &data[..data_len];

    let mut prefix = [0u8; 17];
    let prefix_len = 1 + {
        let (tag, data) = prefix.as_mut_slice().split_at_mut(1);
        coder.encode(
            &[
                literals.len() as u32,
                controls as u32,
                delta_diffs.len() as u32,
                data_len as u32,
            ],
            tag,
            data,
        )
    };

    writer.write_all(&prefix[..prefix_len])?;
    writer.write_all(&literals)?;
    writer.write_all(tags)?;
    writer.write_all(&delta_diffs)?;
    writer.write_all(data)?;

    Ok(())
}
