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

use crate::control::Aehobak as AehobakControl;
use crate::control::Bsdiff as BsdiffControl;
use std::io;
use std::io::ErrorKind::{InvalidData, UnexpectedEof};
use streamvbyte64::{Coder, Coder0124};

/// Directly apply a compact representation of bsdiff output.
#[allow(clippy::ptr_arg)]
pub fn patch(old: &[u8], mut patch: &[u8], new: &mut Vec<u8>) -> io::Result<()> {
    let prefix_tag = patch.get(..1).ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[1..];

    let coder = Coder0124::new();
    let prefix_len = coder.data_len(prefix_tag);
    if patch.len() < prefix_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let (controls_len, deltas_len, control_data_len, literals_len) = {
        let mut v = [0u32; 4];
        coder.decode(prefix_tag, patch, &mut v);
        (v[0] as usize, v[1] as usize, v[2] as usize, v[3] as usize)
    };
    patch = &patch[prefix_len..];

    let control_tags_len = controls_len
        .checked_mul(3)
        .ok_or(io::Error::from(InvalidData))?
        .div_ceil(4);
    let delta_tags_len = deltas_len.div_ceil(4);

    let mut control_tags = patch
        .get(..control_tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[control_tags_len..];

    let mut delta_tags = patch
        .get(..delta_tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[delta_tags_len..];

    if patch.len() < control_data_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let mut control_data = patch;
    patch = &patch[control_data_len..];

    let mut literals = patch
        .get(..literals_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[literals_len..];

    let delta_data_len = coder.data_len(delta_tags);
    if patch.len() < delta_data_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let mut delta_data = patch;
    patch = &patch[delta_data_len..];

    let mut delta_diffs = patch
        .get(..deltas_len)
        .ok_or(io::Error::from(UnexpectedEof))?;

    let mut old_cursor: usize = 0;
    let mut delta_cursor: usize = 0;
    let mut stream_cursor: usize = 0;

    let mut controls_len = controls_len * 3;
    let mut u32_buf = [0u32; 128];
    let (mut control_begin, mut control_end) = (0, 0);
    let (mut delta_begin, mut delta_end) = (96, 96);
    while !control_tags.is_empty() || control_begin != control_end {
        if control_begin == control_end {
            let tags = control_tags.get(..24).unwrap_or(control_tags);
            unsafe { std::hint::assert_unchecked(tags.len() <= 24) };
            unsafe { std::hint::assert_unchecked(tags.len() <= control_tags.len()) };
            (control_begin, control_end) = (0, tags.len() * 4);
            let read = coder.decode(tags, control_data, &mut u32_buf[..control_end]);
            unsafe { std::hint::assert_unchecked(read <= control_data.len()) };
            control_end = controls_len.min(control_end);
            controls_len -= control_end;
            control_data = &control_data[read..];
            control_tags = &control_tags[tags.len()..];
        }
        unsafe { std::hint::assert_unchecked(control_begin <= 96 - 3) };
        let buffer = &u32_buf[control_begin..][..3];
        control_begin += 3;
        let control: BsdiffControl = (&AehobakControl::try_from(buffer).unwrap()).into();
        let (add, copy) = (control.add as usize, control.copy as usize);
        let new_stream_cursor = stream_cursor
            .checked_add(add)
            .ok_or(io::Error::from(InvalidData))?;
        let old_slice = old
            .get(old_cursor..)
            .ok_or(io::Error::from(UnexpectedEof))?
            .get(..add)
            .ok_or(io::Error::from(UnexpectedEof))?;
        let new_cursor = new.len();
        new.extend_from_slice(old_slice);
        while !delta_diffs.is_empty() && (!delta_tags.is_empty() || delta_begin != delta_end) {
            if delta_begin == delta_end {
                let tags = delta_tags.get(..8).unwrap_or(delta_tags);
                unsafe { std::hint::assert_unchecked(tags.len() <= 8) };
                unsafe { std::hint::assert_unchecked(tags.len() <= delta_tags.len()) };
                delta_begin = 96;
                delta_end = delta_begin + tags.len() * 4;
                let read = coder.decode(tags, delta_data, &mut u32_buf[delta_begin..delta_end]);
                unsafe { std::hint::assert_unchecked(read <= delta_data.len()) };
                delta_data = &delta_data[read..];
                delta_tags = &delta_tags[tags.len()..];
            }
            unsafe { std::hint::assert_unchecked(delta_begin < 128) };
            let Some(new_delta_cursor) = delta_cursor.checked_add(u32_buf[delta_begin] as usize)
            else {
                break;
            };
            if new_delta_cursor >= new_stream_cursor {
                break;
            }
            unsafe {
                std::hint::assert_unchecked(
                    new_delta_cursor - stream_cursor + new_cursor < new.len(),
                );
            }
            let new_byte = &mut new[new_delta_cursor - stream_cursor + new_cursor];
            *new_byte = new_byte.wrapping_add(delta_diffs[0]);
            delta_cursor = new_delta_cursor
                .checked_add(1)
                .ok_or(io::Error::from(InvalidData))?;
            delta_begin += 1;
            delta_diffs = &delta_diffs[1..];
        }
        new.extend_from_slice(literals.get(..copy).ok_or(io::Error::from(UnexpectedEof))?);
        literals = &literals[copy..];
        stream_cursor = new_stream_cursor;
        old_cursor = usize::try_from(
            i64::try_from(
                old_cursor
                    .checked_add(add)
                    .ok_or(io::Error::from(InvalidData))?,
            )
            .map_err(|_| io::Error::from(InvalidData))?
            .checked_add(control.seek)
            .ok_or(io::Error::from(InvalidData))?,
        )
        .map_err(|_| io::Error::from(InvalidData))?;
    }
    Ok(())
}
