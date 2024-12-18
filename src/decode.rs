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
