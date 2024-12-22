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
use std::io::Read;
use streamvbyte64::{Coder, Coder0124};

/// Decode a compact representation of bsdiff output.
#[allow(clippy::ptr_arg)]
pub fn decode<T: Read>(reader: &mut T, patch: &mut Vec<u8>) -> io::Result<()> {
    let mut prefix = [0u8; 20];
    reader.read_exact(&mut prefix)?;

    let controls_len = u32::from_le_bytes(prefix[..4].try_into().unwrap()) as usize;
    let control_data_len = u32::from_le_bytes(prefix[4..8].try_into().unwrap()) as usize;
    let literals_len = u32::from_le_bytes(prefix[8..12].try_into().unwrap()) as usize;
    let deltas_len = u32::from_le_bytes(prefix[12..16].try_into().unwrap()) as usize;
    let delta_data_len = u32::from_le_bytes(prefix[16..].try_into().unwrap()) as usize;

    let control_tags_len = (controls_len + 3) / 4;
    let delta_tags_len = (deltas_len + 3) / 4;

    let mut control_tags = vec![0; control_tags_len];
    let mut control_data = vec![0; control_data_len];
    let mut literals = vec![0; literals_len];
    let mut delta_diffs = vec![0; deltas_len];
    let mut delta_tags = vec![0; delta_tags_len];
    let mut delta_data = vec![0; delta_data_len];

    reader.read_exact(&mut control_tags)?;
    reader.read_exact(&mut delta_tags)?;
    reader.read_exact(&mut control_data)?;
    reader.read_exact(&mut literals)?;
    reader.read_exact(&mut delta_data)?;
    reader.read_exact(&mut delta_diffs)?;

    let coder = Coder0124::new();

    let mut controls = vec![0; 4 * control_tags_len];
    let _ = coder.decode(&control_tags, &control_data, &mut controls);
    controls.truncate(controls_len);

    let mut delta_skips = vec![0; 4 * delta_tags_len];
    let _ = coder.decode(&delta_tags, &delta_data, &mut delta_skips);
    delta_skips.truncate(deltas_len);

    let mut literals = literals.as_slice();
    let mut delta_skips = delta_skips.as_slice();
    let mut delta_diffs = delta_diffs.as_slice();

    let mut delta_buf = Vec::new();
    let mut delta_cursor = 0;
    let mut stream_cursor = 0;

    for buffer in controls.chunks_exact(3) {
        let control: BsdiffControl = (&AehobakControl::try_from(buffer).unwrap()).into();
        let (add, copy) = (control.add as usize, control.copy as usize);
        control.encode(patch);
        delta_buf.clear();
        delta_buf.resize(add, 0);
        while !delta_skips.is_empty() && !delta_diffs.is_empty() {
            let new_delta_cursor = delta_cursor + delta_skips[0] as usize;
            if new_delta_cursor >= stream_cursor + add {
                break;
            }
            delta_buf[new_delta_cursor - stream_cursor] = delta_diffs[0];
            delta_cursor = new_delta_cursor + 1;
            delta_skips = &delta_skips[1..];
            delta_diffs = &delta_diffs[1..];
        }
        patch.extend(&delta_buf);
        patch.extend(&literals[..copy]);
        literals = &literals[copy..];
        stream_cursor += add;
    }
    Ok(())
}
