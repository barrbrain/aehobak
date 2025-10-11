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
use std::io::Write;
use std::{error, io};

/// Directly generate a compact representation of bsdiff output.
/// If numeric limits are reached, the error will be wrapped with `io::Error`.
/// The exact control sequence may not match the bsdiff crate.
pub fn diff<T: Write>(old: &[u8], new: &[u8], writer: &mut T) -> io::Result<()> {
    diff_internal(old, new, writer)
}

#[inline]
fn invalid_data<E>(e: E) -> io::Error
where
    E: Into<Box<dyn error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, e)
}

fn diff_internal(old: &[u8], new: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let sa = sais(old)?;
    let mut scanner = ScanState::new(old, new, &sa);
    let mut encoder = EncoderState::new(new.len());

    while !scanner.done() {
        if !scanner.advance()? {
            continue;
        }
        let mut add = scanner.calc_add()?;
        let mut back = scanner.calc_back()?;
        (add, back) = scanner.optimize_overlap(add, back)?;
        let (copy, seek) = scanner.calc_copy_seek(add, back)?;

        let add_u32: u32 = add.try_into().map_err(invalid_data)?;
        let copy_u32: u32 = copy.try_into().map_err(invalid_data)?;
        let seek_i32: i32 = seek.try_into().map_err(invalid_data)?;

        encoder.control(Aehobak {
            add: add_u32,
            copy: copy_u32,
            seek: seek_i32,
        });
        encoder.add(scanner.old_add_slice(add)?, scanner.new_add_slice(add)?);
        encoder.copy(scanner.new_copy_slice(add, copy)?);
        scanner.commit(back)?;
    }
    encoder.finalize(writer)
}

fn sais(old: &[u8]) -> io::Result<Box<[i32]>> {
    use libsais_sys::libsais::libsais;
    if old.len() > i32::MAX as usize {
        return Err(invalid_data("libsais input too large"));
    }
    let mut sa = Vec::with_capacity(old.len());
    let len = old.len() as i32;
    let mut freq = Vec::with_capacity(256);
    let ret = unsafe { libsais(old.as_ptr(), sa.as_mut_ptr(), len, 0, freq.as_mut_ptr()) };
    if ret == 0 {
        unsafe {
            sa.set_len(old.len());
            freq.set_len(256);
        }
        Ok(sa.into_boxed_slice())
    } else {
        Err(io::Error::other("libsais failed"))
    }
}

#[inline(always)]
fn mismatch(old: &[u8], new: &[u8]) -> usize {
    old.iter().zip(new).take_while(|(a, b)| a == b).count()
}

struct ScanState<'a> {
    sa: &'a [i32],
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
    fn new(old: &'a [u8], new: &'a [u8], sa: &'a [i32]) -> Self {
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

    fn find_best_match(&self) -> io::Result<(usize, usize)> {
        let mut sa = self.sa;
        let old = self.old;
        let new = self.new.get(self.scan..).ok_or_else(|| invalid_data(""))?;

        while sa.len() > 2 {
            let pos = (sa.len() - 1) / 2;

            let old_start = sa
                .get(pos)
                .and_then(|&p| usize::try_from(p).ok())
                .ok_or_else(|| invalid_data(""))?;
            let old_slice = old.get(old_start..).ok_or_else(|| invalid_data(""))?;

            let len = old_slice.len().min(new.len());

            sa = if old_slice.get(..len) < new.get(..len) {
                &sa[pos..]
            } else {
                &sa[..=pos]
            };
        }

        if sa.is_empty() {
            return Err(invalid_data(""));
        }

        let a_start = sa
            .first()
            .and_then(|&p| usize::try_from(p).ok())
            .ok_or_else(|| invalid_data(""))?;
        let b_start = sa
            .last()
            .and_then(|&p| usize::try_from(p).ok())
            .ok_or_else(|| invalid_data(""))?;
        let a = mismatch(old.get(a_start..).ok_or_else(|| invalid_data(""))?, new);
        let b = mismatch(old.get(b_start..).ok_or_else(|| invalid_data(""))?, new);

        Ok(if a > b { (a_start, a) } else { (b_start, b) })
    }

    fn advance(&mut self) -> io::Result<bool> {
        let mut score = 0;
        self.scan = self
            .scan
            .checked_add(self.len)
            .ok_or_else(|| invalid_data(""))?;
        let mut subscan = self.scan;

        while self.scan < self.new.len() {
            (self.pos, self.len) = self.find_best_match()?;
            let scan_limit = self
                .scan
                .checked_add(self.len)
                .ok_or_else(|| invalid_data(""))?;

            while subscan < scan_limit {
                let idx = subscan
                    .checked_add_signed(self.last_offset)
                    .ok_or_else(|| invalid_data(""))?;
                if let Some(old_byte) = self.old.get(idx) {
                    if old_byte == &self.new[subscan] {
                        score += 1;
                    }
                }
                subscan = subscan.checked_add(1).ok_or_else(|| invalid_data(""))?;
            }

            if (self.len == score && self.len != 0) || self.len > score + 8 {
                break;
            }

            let idx = self
                .scan
                .checked_add_signed(self.last_offset)
                .ok_or_else(|| invalid_data(""))?;
            if idx < self.old.len() && self.old[idx] == self.new[self.scan] {
                score -= 1;
            }
            self.scan = self.scan.checked_add(1).ok_or_else(|| invalid_data(""))?;
        }

        Ok(self.len != score || self.scan == self.new.len())
    }

    fn calc_add(&self) -> io::Result<usize> {
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
            i = i.checked_add(1).ok_or_else(|| invalid_data(""))?;
            if score * 2 - i as i32 > best * 2 - add as i32 {
                best = score;
                add = i;
            }
        }

        Ok(add)
    }

    fn calc_back(&self) -> io::Result<usize> {
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
                .get(self.pos.checked_sub(i).ok_or_else(|| invalid_data(""))?)
                .zip(
                    self.new
                        .get(self.scan.checked_sub(i).ok_or_else(|| invalid_data(""))?),
                )
                .is_some_and(|(o, n)| o == n)
            {
                score += 1;
            }

            if score * 2 - i as isize > best * 2 - back as isize {
                best = score;
                back = i;
            }

            i = i.checked_add(1).ok_or_else(|| invalid_data(""))?;
        }

        Ok(back)
    }

    fn optimize_overlap(&self, mut add: usize, mut back: usize) -> io::Result<(usize, usize)> {
        if self
            .last_scan
            .checked_add(add)
            .ok_or_else(|| invalid_data(""))?
            > self
                .scan
                .checked_sub(back)
                .ok_or_else(|| invalid_data(""))?
        {
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

    fn calc_copy_seek(&self, add: usize, back: usize) -> io::Result<(usize, isize)> {
        let copy = self
            .scan
            .checked_sub(back)
            .and_then(|v| v.checked_sub(self.last_scan + add))
            .ok_or_else(|| invalid_data(""))?;
        let seek = (self.pos as isize)
            .checked_sub(self.last_pos as isize)
            .and_then(|v| v.checked_sub((back + add) as isize))
            .ok_or_else(|| invalid_data(""))?;

        Ok((copy, seek))
    }

    #[inline(always)]
    fn old_add_slice(&self, add: usize) -> Result<&[u8], io::Error> {
        self.old
            .get(self.last_pos..)
            .and_then(|s| s.get(..add))
            .ok_or_else(|| invalid_data(""))
    }

    #[inline(always)]
    fn new_add_slice(&self, add: usize) -> Result<&[u8], io::Error> {
        self.new
            .get(self.last_scan..)
            .and_then(|s| s.get(..add))
            .ok_or_else(|| invalid_data(""))
    }

    #[inline(always)]
    fn new_copy_slice(&self, add: usize, copy: usize) -> Result<&[u8], io::Error> {
        self.new
            .get(self.last_scan..)
            .and_then(|s| s.get(add..))
            .and_then(|s| s.get(..copy))
            .ok_or_else(|| invalid_data(""))
    }

    fn commit(&mut self, back: usize) -> io::Result<()> {
        self.last_scan = self
            .scan
            .checked_sub(back)
            .ok_or_else(|| invalid_data(""))?;
        self.last_pos = self.pos.checked_sub(back).ok_or_else(|| invalid_data(""))?;
        self.last_offset = (self.pos as isize)
            .checked_sub(self.scan as isize)
            .ok_or_else(|| invalid_data(""))?;
        Ok(())
    }
}
