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

use crate::control::Aehobak;
use crate::encode::EncoderState;
use anyhow::{ensure, Context, Error, Result};
use std::io;
use std::io::Write;

/// Directly generate a compact representation of bsdiff output.
/// If numeric limits are reached, the error will be wrapped with `io::Error`.
pub fn diff<T: Write>(old: &[u8], new: &[u8], writer: &mut T) -> io::Result<()> {
    match diff_internal(old, new, writer) {
        Ok(_) => Ok(()),
        Err(e) => match e.downcast::<io::Error>() {
            Ok(e) => Err(e),
            Err(e) => Err(io::Error::other(e)),
        },
    }
}

fn diff_internal(old: &[u8], new: &[u8], writer: &mut dyn Write) -> Result<()> {
    #[cfg(miri)]
    let sa = suf_sort_naive(old)?;
    #[cfg(not(miri))]
    let sa = sais(old)?;
    let mut scanner = ScanState::new(old, new, &sa);
    let mut encoder = EncoderState::new(new.len());

    while !scanner.done() {
        if !scanner.advance()? {
            continue;
        }
        let (add, back) = scanner.optimize_overlap(scanner.calc_add()?, scanner.calc_back()?)?;
        let (copy, seek) = scanner.calc_copy_seek(add, back)?;

        let add_u32: u32 = add.try_into()?;
        let copy_u32: u32 = copy.try_into()?;
        let seek_i32: i32 = seek.try_into()?;

        encoder.control(Aehobak {
            add: add_u32,
            copy: copy_u32,
            seek: seek_i32,
        });
        encoder.add(scanner.old_add_slice(add)?, scanner.new_add_slice(add)?);
        encoder.copy(scanner.new_copy_slice(add, copy)?);
        scanner.commit(back)?;
    }
    encoder.finalize(writer)?;
    Ok(())
}

#[cfg(miri)]
fn suf_sort_naive(old: &[u8]) -> Result<Box<[u32]>> {
    ensure!(old.len() <= i32::MAX as usize, "input too large");
    let mut sa: Vec<u32> = (0..old.len() as u32).collect();
    sa.sort_unstable_by_key(|&v| {
        // SAFETY: Values of `sa` are offsets into `old`
        unsafe { old.get_unchecked(v as usize..) }
    });
    Ok(sa.into_boxed_slice())
}

#[cfg(not(miri))]
fn sais(old: &[u8]) -> Result<Box<[u32]>> {
    ensure!(old.len() <= i32::MAX as usize, "input too large");
    let (_, sa) = cdivsufsort::sort(old).into_parts();
    // SAFETY: i32 to u32 transmute is safe; non-negative values
    let sa: Vec<u32> = unsafe { core::mem::transmute(sa) };
    Ok(sa.into_boxed_slice())
}

#[inline(never)]
fn mismatch(old: &[u8], new: &[u8]) -> usize {
    let min_len = old.len().min(new.len()).min(i32::MAX as usize);
    let mut i = 0;
    while i + 32 <= min_len {
        if old[i..i + 32] != new[i..i + 32] {
            break;
        }
        i += 32;
    }
    while i < min_len {
        if old[i] != new[i] {
            break;
        }
        i += 1;
    }
    i
}

struct ScanState<'a> {
    sa: &'a [u32],
    old: &'a [u8],
    new: &'a [u8],
    scan: usize,
    len: usize,
    pos: usize,
    last_scan: usize,
    last_pos: usize,
    last_offset: isize,
}

impl<'a> ScanState<'a> {
    #[inline(always)]
    fn new(old: &'a [u8], new: &'a [u8], sa: &'a [u32]) -> Self {
        Self {
            sa,
            old,
            new,
            scan: 0,
            len: 0,
            pos: 0,
            last_scan: 0,
            last_pos: 0,
            last_offset: 0,
        }
    }

    #[inline(always)]
    fn done(&self) -> bool {
        self.scan >= self.new.len()
    }

    fn find_best_match(&self) -> Result<(usize, usize)> {
        let mut sa = self.sa;
        let new = self.new.get(self.scan..).context("")?;

        while sa.len() > 2 {
            let pos = (sa.len() - 1) / 2;
            let old_start = sa.get(pos).map(|&p| p as usize).context("")?;
            let old_slice = self.old.get(old_start..).context("")?;

            let len = old_slice.len().min(new.len());
            sa = if old_slice.get(..len) < new.get(..len) {
                &sa[pos..]
            } else {
                &sa[..=pos]
            };
        }

        if sa.is_empty() {
            return Ok((self.sa.len(), 0));
        }

        let a_start = sa.first().map(|&p| p as usize).context("")?;
        let b_start = sa.last().map(|&p| p as usize).context("")?;
        let a = mismatch(self.old.get(a_start..).context("")?, new);
        let b = mismatch(self.old.get(b_start..).context("")?, new);

        Ok(if a > b { (a_start, a) } else { (b_start, b) })
    }

    fn advance(&mut self) -> Result<bool> {
        self.scan = self.scan.checked_add(self.len).context("")?;
        let mut score = 0;
        let mut subscan = self.scan;

        while self.scan < self.new.len() {
            (self.pos, self.len) = self.find_best_match()?;
            let scan_limit = self.scan.checked_add(self.len).context("")?;

            while subscan < scan_limit {
                let idx = subscan.checked_add_signed(self.last_offset).context("")?;
                if let Some(old_byte) = self.old.get(idx) {
                    if old_byte == &self.new[subscan] {
                        score += 1;
                    }
                }
                subscan = subscan.checked_add(1).context("")?;
            }

            if (self.len == score && self.len != 0) || self.len > score + 8 {
                break;
            }

            let idx = self.scan.checked_add_signed(self.last_offset).context("")?;
            if idx < self.old.len() && self.old[idx] == self.new[self.scan] {
                score -= 1;
            }
            self.scan = self.scan.checked_add(1).context("")?;
        }
        Ok(self.len != score || self.scan == self.new.len())
    }

    fn calc_add(&self) -> Result<usize> {
        let mut add = 0;
        let mut score = 0;
        let mut best = 0;
        let mut i = 0;

        while self.last_scan + i < self.scan && self.last_pos + i < self.old.len() {
            if self
                .old
                .get(self.last_pos + i)
                .zip(self.new.get(self.last_scan + i))
                .is_some_and(|(o, n)| o == n)
            {
                score += 1;
            }
            i = i.checked_add(1).context("")?;
            if score * 2 - i as i32 > best * 2 - add as i32 {
                best = score;
                add = i;
            }
        }
        Ok(add)
    }

    fn calc_back(&self) -> Result<usize> {
        if self.scan >= self.new.len() {
            return Ok(0);
        }

        let mut back = 0;
        let mut score = 0;
        let mut best = 0;
        let mut i = 1;

        while self.scan >= self.last_scan + i && self.pos >= i {
            if self
                .old
                .get(self.pos.checked_sub(i).context("")?)
                .zip(self.new.get(self.scan.checked_sub(i).context("")?))
                .is_some_and(|(o, n)| o == n)
            {
                score += 1;
            }

            if score * 2 - i as isize > best * 2 - back as isize {
                best = score;
                back = i;
            }
            i = i.checked_add(1).context("")?;
        }
        Ok(back)
    }

    fn optimize_overlap(&self, mut add: usize, mut back: usize) -> Result<(usize, usize)> {
        if self.last_scan.checked_add(add).context("")? > self.scan.checked_sub(back).context("")? {
            let overlap = self.last_scan + add - (self.scan - back);

            let mut score = 0;
            let mut best = 0;
            let mut forward = 0;

            for i in 0..overlap {
                // Safely compare indices within bounds
                if self.new.get(self.last_scan + add - overlap + i)
                    == self.old.get(self.last_pos + add - overlap + i)
                {
                    score += 1;
                }

                if self.new.get(self.scan - back + i) == self.old.get(self.pos - back + i) {
                    score -= 1;
                }

                if score > best {
                    best = score;
                    forward = i + 1;
                }
            }

            add = add + forward - overlap;
            back -= forward;
        }

        Ok((add, back))
    }

    fn calc_copy_seek(&self, add: usize, back: usize) -> Result<(usize, isize)> {
        let copy = self
            .scan
            .checked_sub(back)
            .and_then(|v| v.checked_sub(self.last_scan + add))
            .context("")?;
        let seek = (self.pos as isize)
            .checked_sub(self.last_pos as isize)
            .and_then(|v| v.checked_sub((back + add) as isize))
            .context("")?;

        Ok((copy, seek))
    }

    #[inline(always)]
    fn old_add_slice(&self, add: usize) -> Result<&[u8], Error> {
        self.old
            .get(self.last_pos..)
            .and_then(|s| s.get(..add))
            .context("")
    }

    #[inline(always)]
    fn new_add_slice(&self, add: usize) -> Result<&[u8], Error> {
        self.new
            .get(self.last_scan..)
            .and_then(|s| s.get(..add))
            .context("")
    }

    #[inline(always)]
    fn new_copy_slice(&self, add: usize, copy: usize) -> Result<&[u8], Error> {
        self.new
            .get(self.last_scan..)
            .and_then(|s| s.get(add..))
            .and_then(|s| s.get(..copy))
            .context("")
    }

    fn commit(&mut self, back: usize) -> Result<()> {
        self.last_scan = self.scan.checked_sub(back).context("")?;
        self.last_pos = self.pos.checked_sub(back).context("")?;
        self.last_offset = (self.pos as isize)
            .checked_sub(self.scan as isize)
            .context("")?;
        Ok(())
    }
}
