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

use crate::control::Bsdiff as BsdiffControl;
use std::io;
use std::io::Read;

/// Decode a segmented representation of bsdiff output.
#[allow(clippy::ptr_arg)]
pub fn decode<T: Read>(reader: &mut T, patch: &mut Vec<u8>) -> io::Result<()> {
    let mut prefix = [0u8; 24];
    reader.read_exact(&mut prefix)?;

    let headers_len = u64::from_le_bytes(prefix[..8].try_into().unwrap()) as usize;
    let literals_len = u64::from_le_bytes(prefix[8..16].try_into().unwrap()) as usize;
    let deltas_len = u64::from_le_bytes(prefix[16..].try_into().unwrap()) as usize;

    let mut headers = vec![0; headers_len];
    let mut literals = vec![0; literals_len];
    let mut deltas = vec![0; deltas_len];

    reader.read_exact(&mut headers)?;
    reader.read_exact(&mut literals)?;
    reader.read_exact(&mut deltas)?;

    let mut literals = literals.as_slice();
    let mut deltas = deltas.as_slice();

    for buffer in headers.chunks_exact(24) {
        let control: BsdiffControl = buffer.try_into().unwrap();
        let (add, copy) = (control.add as usize, control.copy as usize);
        patch.extend(buffer);
        patch.extend(&deltas[..add]);
        patch.extend(&literals[..copy]);
        deltas = &deltas[add..];
        literals = &literals[copy..];
    }
    Ok(())
}
