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
use std::hint::assert_unchecked;
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

    let control_tags = patch
        .get(..control_tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[control_tags_len..];

    let delta_tags = patch
        .get(..delta_tags_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[delta_tags_len..];

    if patch.len() < control_data_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let control_data = patch;
    patch = &patch[control_data_len..];

    let mut literals = patch
        .get(..literals_len)
        .ok_or(io::Error::from(UnexpectedEof))?;
    patch = &patch[literals_len..];

    let delta_data_len = coder.data_len(delta_tags);
    if patch.len() < delta_data_len {
        return Err(io::Error::from(UnexpectedEof));
    }
    let delta_data = patch;
    patch = &patch[delta_data_len..];

    let mut delta_diffs = patch
        .get(..deltas_len)
        .ok_or(io::Error::from(UnexpectedEof))?;

    let buf_len = control_tags_len
        .checked_add(delta_tags_len)
        .ok_or(io::Error::from(InvalidData))?
        .checked_mul(4)
        .ok_or(io::Error::from(InvalidData))?;
    let mut u32_buf = vec![0; buf_len];
    // SAFETY: This follows from the checked arithmetic above
    unsafe {
        assert_unchecked(u32_buf.len() >= 4 * control_tags_len);
    }
    let (controls, delta_skips) = u32_buf.split_at_mut(4 * control_tags_len);

    // SAFETY: This follows from the checked arithmetic above
    unsafe {
        assert_unchecked(controls.len() >= controls_len * 3);
        assert_unchecked(delta_skips.len() >= deltas_len);
    }
    let _ = coder.decode(control_tags, control_data, controls);
    let controls = &controls[..controls_len * 3];
    let _ = coder.decode(delta_tags, delta_data, delta_skips);
    let mut delta_skips = &delta_skips[..deltas_len];

    let mut old_cursor: usize = 0;
    let mut delta_cursor: usize = 0;
    let mut stream_cursor: usize = 0;

    for buffer in controls.chunks_exact(3) {
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
        while !delta_skips.is_empty() && !delta_diffs.is_empty() {
            let Some(new_delta_cursor) = delta_cursor.checked_add(delta_skips[0] as usize) else {
                break;
            };
            if new_delta_cursor >= new_stream_cursor {
                break;
            }
            // SAFETY: This follows from the checked arithmetic above
            unsafe {
                assert_unchecked(new.len() > new_delta_cursor - stream_cursor + new_cursor);
            }
            let new_byte = &mut new[new_delta_cursor - stream_cursor + new_cursor];
            *new_byte = new_byte.wrapping_add(delta_diffs[0]);
            delta_cursor = new_delta_cursor
                .checked_add(1)
                .ok_or(io::Error::from(InvalidData))?;
            delta_skips = &delta_skips[1..];
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
