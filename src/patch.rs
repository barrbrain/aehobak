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

use std::hint::assert_unchecked;
use std::io;
use std::io::ErrorKind::{InvalidData, UnexpectedEof};
use streamvbyte64::{Coder, Coder0124};

/// Directly apply a compact representation of bsdiff output.
/// Attempts to fill `new` beyond its capacity will result in `Err`.
#[allow(clippy::ptr_arg)]
pub fn patch(old: &[u8], mut patch: &[u8], new: &mut Vec<u8>) -> io::Result<()> {
    let prefix_tag = patch.get(..1).ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[1..];

    let coder = Coder0124::new();
    let prefix_len = coder.data_len(prefix_tag);
    if patch.len() < prefix_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let (literals_len, controls, deltas_len, data_len) = {
        let mut v = [0u32; 4];
        coder.decode(prefix_tag, patch, &mut v);
        (v[0] as usize, v[1] as usize, v[2] as usize, v[3] as usize)
    };
    patch = &patch[prefix_len..];

    let mut literals = patch
        .get(..literals_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[literals_len..];

    let tags_len = controls
        .div_ceil(4)
        .checked_mul(3)
        .ok_or(io::Error::from(InvalidData))?
        .checked_add(deltas_len.div_ceil(4))
        .ok_or(io::Error::from(InvalidData))?;
    let u32_seq_len = tags_len
        .checked_mul(4)
        .ok_or(io::Error::from(InvalidData))?;
    // SAFETY: This follows from the checked arithmetic above
    unsafe { assert_unchecked(u32_seq_len >= controls.div_ceil(4) * 12) }

    let tags = patch
        .get(..tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[tags_len..];
    let (control_tags, delta_tags) = tags.split_at(controls.div_ceil(4) * 3);

    let mut delta_diffs = patch
        .get(..deltas_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[deltas_len..];

    let control_data_len = coder.data_len(control_tags);
    if patch.len() < data_len || data_len < control_data_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let data = &patch[..data_len];
    let (control_data, delta_data) = data.split_at(control_data_len);

    let mut u32_seq = vec![0; u32_seq_len];
    let (control_seq, delta_pos) = u32_seq.split_at_mut(controls.div_ceil(4) * 12);
    let controls_padded = controls.div_ceil(4) * 4;
    // SAFETY: These follow from the checked arithmetic above
    unsafe {
        assert_unchecked(delta_pos.len() >= deltas_len);
        assert_unchecked(control_seq.len() >= controls_padded * 2 + controls);
    }

    let _ = coder.decode(control_tags, control_data, control_seq);
    let _ = coder.decode_deltas(0, delta_tags, delta_data, delta_pos);
    for (idx, pos) in delta_pos.iter_mut().enumerate() {
        *pos = (*pos).wrapping_add(idx as u32);
    }
    for seek in &mut control_seq[..controls_padded] {
        let x = *seek;
        *seek = (x >> 1) ^ (x & 1).wrapping_neg()
    }
    let mut delta_pos = &delta_pos[..deltas_len];
    let seeks = &control_seq[..controls];
    let adds = &control_seq[controls_padded..][..controls];
    let copies = &control_seq[controls_padded * 2..][..controls];

    let mut old_cursor: usize = 0;
    let mut copy_cursor: usize = 0;

    for (&add, (&copy, &seek)) in adds.iter().zip(copies.iter().zip(seeks)) {
        let (add, copy, seek) = (add as usize, copy as usize, seek as i32 as i64);
        let old_slice = old
            .get(old_cursor..)
            .ok_or(io::Error::from(UnexpectedEof))?
            .get(..add)
            .ok_or(io::Error::from(UnexpectedEof))?;
        if new.capacity().wrapping_sub(new.len()) < old_slice.len() {
            Err(io::Error::from(UnexpectedEof))?;
        }
        new.extend_from_slice(old_slice);
        let mut nonzero = delta_pos.len().min(delta_diffs.len());
        for i in 0..nonzero {
            let delta_cursor = copy_cursor.wrapping_add(delta_pos[i] as usize);
            if delta_cursor >= new.len() {
                nonzero = i;
                break;
            }
            new[delta_cursor] = new[delta_cursor].wrapping_add(delta_diffs[i]);
        }
        delta_pos = &delta_pos[nonzero..];
        delta_diffs = &delta_diffs[nonzero..];
        let lit_slice = literals.get(..copy).ok_or(io::Error::from(UnexpectedEof))?;
        if new.capacity().wrapping_sub(new.len()) < lit_slice.len() {
            Err(io::Error::from(UnexpectedEof))?;
        }
        new.extend_from_slice(lit_slice);
        literals = &literals[copy..];
        copy_cursor = copy_cursor.wrapping_add(copy);
        old_cursor = usize::try_from(
            i64::try_from(
                old_cursor
                    .checked_add(add)
                    .ok_or(io::Error::from(InvalidData))?,
            )
            .map_err(|_| io::Error::from(InvalidData))?
            .checked_add(seek)
            .ok_or(io::Error::from(InvalidData))?,
        )
        .map_err(|_| io::Error::from(InvalidData))?;
    }
    Ok(())
}
