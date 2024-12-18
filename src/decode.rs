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
use std::io::Read;
use streamvbyte64::{Coder, Coder0124};

/// Decode a verbatim representation of bsdiff output.
#[allow(clippy::ptr_arg)]
pub fn decode<T: Read>(reader: &mut T, patch: &mut Vec<u8>) -> io::Result<()> {
    use io::ErrorKind::{InvalidData, UnexpectedEof};
    let mut buffer = [0; 24];
    loop {
        // Stop when EOF reached at start of frame.
        match reader.read(&mut buffer)? {
            0 => return Ok(()),
            n => reader.read_exact(&mut buffer[n..])?,
        }
        patch.extend(&buffer);

        let mix = u64::from_le_bytes(buffer[..8].try_into().unwrap());
        let copy = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
        let len = copy.checked_add(mix).ok_or(io::Error::from(InvalidData))?;

        if len != reader.take(len).read_to_end(patch)? as u64 {
            return Err(UnexpectedEof.into());
        }
    }
}

#[allow(unused)]
fn patch<T: Read>(old: &[u8], patch: &mut T, new: &mut Vec<u8>) -> io::Result<()> {
    let mut buf = [0; 16];
    patch.read_exact(&mut buf)?;

    let mut num_copies = u32::from_le_bytes(buf[0..][..4].try_into().unwrap()) as usize;
    let num_edits = u32::from_le_bytes(buf[4..][..4].try_into().unwrap()) as usize;
    let len_copies = u32::from_le_bytes(buf[8..][..4].try_into().unwrap()) as usize;
    let len_edits = u32::from_le_bytes(buf[12..][..4].try_into().unwrap()) as usize;

    let mut copy_tags = vec![0; (num_copies + 3) >> 2];
    patch.read_exact(&mut copy_tags)?;
    let mut edit_tags = vec![0; (num_edits + 3) >> 2];
    patch.read_exact(&mut edit_tags)?;

    let mut copy_data = vec![0; len_copies];
    patch.read_exact(&mut copy_tags)?;
    let mut copy_data = copy_data.as_slice();

    let mut edit_data = vec![0; len_edits];
    patch.read_exact(&mut edit_tags)?;
    let mut edit_data = edit_data.as_slice();

    let mut deltas = vec![0; num_edits];
    patch.read_exact(&mut deltas)?;
    let mut deltas = deltas.as_slice();

    // Literals follow

    let coder = Coder0124::new();

    let mut cursor = 0;
    let mut copies = [0u32; 12];
    for tags in copy_tags.chunks(3) {
        let read = coder.decode(tags, copy_data, &mut copies);
        copy_data = &copy_data[read..];
        for p in copies[..num_copies.min(12)].chunks_exact(3) {
            let mix = p[0] as usize;
            new.extend(&old[cursor..][..mix]);
            let lit = p[1] as usize;
            if lit != patch.take(lit as u64).read_to_end(new)? {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            cursor += mix;
            let seek = p[2];
            if seek & 1 != 0 {
                cursor -= (seek >> 1) as usize + 1;
            } else {
                cursor += (seek >> 1) as usize;
            }
            num_copies -= 3;
        }
    }

    cursor = 0;
    let mut edits = [0u32; 4];
    for tag in edit_tags.as_slice() {
        let read = coder.decode(std::slice::from_ref(tag), edit_data, &mut edits);
        edit_data = &edit_data[read..];
        for (&edit, &delta) in edits.iter().zip(deltas) {
            cursor += edit as usize;
            new[cursor] = new[cursor].wrapping_add(delta);
            cursor += 1;
        }
        deltas = &deltas[deltas.len().min(4)..];
    }
    Ok(())
}
