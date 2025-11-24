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
use std::io::Read;
use streamvbyte64::{Coder, Coder0124};

/// Decode a compact representation of bsdiff output.
#[allow(clippy::ptr_arg)]
pub fn decode<T: Read>(reader: &mut T, patch: &mut Vec<u8>) -> io::Result<()> {
    let mut prefix = [0u8; 17];
    reader.read_exact(&mut prefix[..1])?;

    let coder = Coder0124::new();

    let prefix_len = coder.data_len(&prefix[..1]);
    reader.read_exact(&mut prefix[1..1 + prefix_len])?;

    let (deltas_len, literals_len, controls, data_len) = {
        let mut v = [0u32; 4];
        let (tag, data) = prefix.as_mut_slice().split_at_mut(1);
        coder.decode(tag, data, &mut v);
        (v[0] as usize, v[1] as usize, v[2] as usize, v[3] as usize)
    };

    let tags_len = controls.div_ceil(4) * 3 + deltas_len.div_ceil(4);

    let mut delta_diffs = vec![0; deltas_len];
    let mut literals = vec![0; literals_len];
    let mut tags = vec![0; tags_len];
    let mut data = vec![0; data_len];

    reader.read_exact(&mut delta_diffs)?;
    reader.read_exact(&mut literals)?;
    reader.read_exact(&mut tags)?;
    reader.read_exact(&mut data)?;

    let mut u32_seq = vec![0; 4 * tags_len];
    let _ = coder.decode(&tags, &data, &mut u32_seq);
    let controls_padded = controls.div_ceil(4) * 4;
    let deltas_padded = deltas_len.div_ceil(4) * 4;
    let delta_pos = &mut u32_seq[controls_padded..][..deltas_padded];
    let mut delta_cursor: u32 = 0;
    for skip in delta_pos {
        let pos = delta_cursor.wrapping_add(*skip);
        delta_cursor = delta_cursor.wrapping_add(*skip).wrapping_add(1);
        *skip = pos;
    }
    let copies = &u32_seq[..controls];
    let mut delta_pos = &u32_seq[controls_padded..][..deltas_len];
    let seeks = &u32_seq[controls_padded + deltas_padded..][..controls];
    let adds = &u32_seq[controls_padded * 2 + deltas_padded..][..controls];

    let mut literals = literals.as_slice();
    let mut delta_diffs = delta_diffs.as_slice();

    let mut delta_buf = Vec::new();
    let mut add_cursor = 0;

    for (&add, (&copy, &seek)) in adds.iter().zip(copies.iter().zip(seeks)) {
        let control: BsdiffControl =
            (&AehobakControl::try_from(&[add, copy, seek][..]).unwrap()).into();
        let (add, copy) = (control.add as usize, control.copy as usize);
        control.encode(patch);
        delta_buf.clear();
        delta_buf.resize(add, 0);
        while !delta_pos.is_empty() && !delta_diffs.is_empty() {
            let delta_cursor = delta_pos[0] as usize;
            if delta_cursor >= add_cursor + add {
                break;
            }
            delta_buf[delta_cursor - add_cursor] = delta_diffs[0];
            delta_pos = &delta_pos[1..];
            delta_diffs = &delta_diffs[1..];
        }
        patch.extend(&delta_buf);
        patch.extend(&literals[..copy]);
        literals = &literals[copy..];
        add_cursor += add;
    }
    Ok(())
}
