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

    let mut tags = patch
        .get(..tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[tags_len..];
    let (add_tags, copy_tags, mut delta_tags, seek_tags);
    (add_tags, tags) = tags.split_at(controls.div_ceil(4));
    (copy_tags, tags) = tags.split_at(controls.div_ceil(4));
    (delta_tags, seek_tags) = tags.split_at(deltas_len.div_ceil(4));

    let mut delta_diffs = patch
        .get(..deltas_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[deltas_len..];

    let add_data_len = coder.data_len(add_tags);
    let copy_data_len = coder.data_len(copy_tags);
    let delta_data_len = coder.data_len(delta_tags);
    let seek_data_len = coder.data_len(seek_tags);
    if patch.len() < data_len
        || add_data_len
            .checked_add(copy_data_len)
            .ok_or(io::Error::from(InvalidData))?
            .checked_add(delta_data_len)
            .ok_or(io::Error::from(InvalidData))?
            .checked_add(seek_data_len)
            .ok_or(io::Error::from(InvalidData))?
            > data_len
    {
        return Err(io::Error::from(UnexpectedEof));
    }
    let mut data = &patch[..data_len];
    let (mut add_data, mut copy_data, mut delta_data, mut seek_data);
    (add_data, data) = data.split_at(add_data_len);
    (copy_data, data) = data.split_at(copy_data_len);
    (delta_data, seek_data) = data.split_at(delta_data_len);

    let mut old_cursor: usize = 0;
    let mut copy_cursor: usize = 0;

    let mut window;
    let mut delta_pos_buf = [0; 32];
    let mut delta_pos = &mut delta_pos_buf[..0];
    let mut delta_base = 0;

    for (add_tags, (copy_tags, seek_tags)) in add_tags
        .chunks(8)
        .zip(copy_tags.chunks(8).zip(seek_tags.chunks(8)))
    {
        let mut adds = [0; 32];
        let mut copies = [0; 32];
        let mut seeks = [0; 32];
        let mut read;
        let mut tags;
        tags = if add_tags.len() >= 8 {
            add_tags
        } else {
            window = [0; 8];
            window[..add_tags.len()].copy_from_slice(add_tags);
            &window[..]
        };
        read = coder.decode(tags, add_data, &mut adds);
        // SAFETY: This follows from the checked arithmetic above
        unsafe { assert_unchecked(read <= add_data.len()) }
        add_data = &add_data[read..];
        tags = if copy_tags.len() >= 8 {
            copy_tags
        } else {
            window = [0; 8];
            window[..copy_tags.len()].copy_from_slice(copy_tags);
            &window[..]
        };
        read = coder.decode(tags, copy_data, &mut copies);
        // SAFETY: This follows from the checked arithmetic above
        unsafe { assert_unchecked(read <= copy_data.len()) }
        copy_data = &copy_data[read..];
        tags = if seek_tags.len() >= 8 {
            seek_tags
        } else {
            window = [0; 8];
            window[..seek_tags.len()].copy_from_slice(seek_tags);
            &window[..]
        };
        read = coder.decode(tags, seek_data, &mut seeks);
        // SAFETY: This follows from the checked arithmetic above
        unsafe { assert_unchecked(read <= seek_data.len()) }
        seek_data = &seek_data[read..];
        for seek in &mut seeks {
            let x = *seek;
            *seek = (x >> 1) ^ (x & 1).wrapping_neg()
        }
        for (&add, (&copy, &seek)) in adds.iter().zip(copies.iter().zip(&seeks)) {
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
            'outer: while !delta_diffs.is_empty() {
                if delta_pos.is_empty() {
                    tags = if delta_tags.len() >= 8 {
                        let window = &delta_tags[..8];
                        delta_tags = &delta_tags[8..];
                        window
                    } else {
                        window = [0; 8];
                        window[..delta_tags.len()].copy_from_slice(delta_tags);
                        delta_tags = &delta_tags[delta_tags.len()..];
                        &window[..]
                    };
                    read = coder.decode_deltas(delta_base, tags, delta_data, &mut delta_pos_buf);
                    // SAFETY: This follows from the checked arithmetic above
                    unsafe { assert_unchecked(read <= delta_data.len()) }
                    delta_pos = &mut delta_pos_buf[..];
                    for (idx, pos) in delta_pos.iter_mut().enumerate() {
                        *pos = (*pos).wrapping_add(idx as u32);
                    }
                    delta_base = delta_pos.last().unwrap() + 1;
                    delta_data = &delta_data[read..];
                }
                let nonzero = delta_diffs.len().min(delta_pos.len());
                for i in 0..nonzero {
                    let delta_cursor = copy_cursor.wrapping_add(delta_pos[i] as usize);
                    if delta_cursor >= new.len() {
                        delta_pos = &mut delta_pos[i..];
                        delta_diffs = &delta_diffs[i..];
                        break 'outer;
                    }
                    new[delta_cursor] = new[delta_cursor].wrapping_add(delta_diffs[i]);
                }
                delta_pos = &mut delta_pos[nonzero..];
                delta_diffs = &delta_diffs[nonzero..];
            }
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
    }
    Ok(())
}
