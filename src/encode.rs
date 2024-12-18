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

/// Encode bsdiff output, returning a segmented representation.
pub fn encode<T: Write>(patch: &[u8], writer: &mut T) -> io::Result<()> {
    encode_internal(patch, writer)
}

fn encode_internal(mut patch: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let mut headers = Vec::<u8>::new();
    let mut deltas = Vec::<u8>::new();
    let mut literals = Vec::<u8>::new();

    while 24 <= patch.len() {
        let mix = u64::from_le_bytes(patch[..8].try_into().unwrap());
        let copy = u64::from_le_bytes(patch[8..16].try_into().unwrap());
        headers.extend(&patch[..24]);
        patch = &patch[24..];
        let (mix, copy) = (mix as usize, copy as usize);
        deltas.extend(&patch[..mix]);
        patch = &patch[mix..];
        literals.extend(&patch[..copy]);
        patch = &patch[copy..];
    }

    let mut prefix = [0u8; 24];
    prefix[..8].copy_from_slice(&(headers.len() as u64).to_le_bytes());
    prefix[8..16].copy_from_slice(&(literals.len() as u64).to_le_bytes());
    prefix[16..].copy_from_slice(&(deltas.len() as u64).to_le_bytes());

    writer.write_all(&prefix)?;
    writer.write_all(&headers)?;
    writer.write_all(&literals)?;
    writer.write_all(&deltas)?;

    Ok(())
}
