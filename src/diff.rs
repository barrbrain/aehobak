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
use std::io;
use std::io::Write;

/// Directly generate a compact representation of bsdiff output.
/// Experimental: may assert if preconditions unmet.
pub fn diff<T: Write>(old: &[u8], new: &[u8], writer: &mut T) -> io::Result<()> {
    diff_internal(old, new, writer)
}

fn diff_internal(old: &[u8], new: &[u8], writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    let mut sa = vec![0; old.len() + 1];
    sais(&mut sa, &mut vec![0; old.len() + 1], old);
    let mut scanner = ScanState::new(old, new, &sa);
    let mut encoder = EncoderState::new();

    while !scanner.done() {
        if !scanner.advance() {
            continue;
        }
        let mut add = scanner.calc_add();
        let mut back = scanner.calc_back();
        (add, back) = scanner.optimize_overlap(add, back);
        let (copy, seek) = scanner.calc_copy_seek(add, back);
        encoder.control(Aehobak {
            add: add as u32,
            copy: copy as u32,
            seek: seek as i32,
        });
        encoder.add(
            &scanner.old[scanner.last_pos..][..add],
            &scanner.new[scanner.last_scan..][..add],
        );
        encoder.copy(&scanner.new[scanner.last_scan + add..][..copy]);
        scanner.commit(back);
    }
    encoder.finalize(writer)
}

fn sais(sa: &mut [i32], tmp: &mut [u16], old: &[u8]) {
    use core::ptr::null_mut;
    use libsais_sys::libsais16::libsais16;

    assert_eq!(tmp.len(), old.len() + 1);
    assert_eq!(sa.len(), old.len() + 1);
    assert!(tmp.len() <= i32::MAX as usize);

    // Use tmp for appending the sentinel symbol
    for (&o, t) in old.iter().zip(tmp.iter_mut()) {
        *t = o as u16 + 1;
    }
    tmp[old.len()] = 0;
    let len = tmp.len() as i32;

    let ret = unsafe { libsais16(tmp.as_ptr(), sa.as_mut_ptr(), len, 0, null_mut()) };
    assert_eq!(ret, 0);
}

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

    fn done(&self) -> bool {
        self.scan >= self.new.len()
    }

    fn find_best_match(&self) -> (usize, usize) {
        let mut sa = self.sa;
        let old = self.old;
        let new = &self.new[self.scan..];
        while sa.len() > 2 {
            let pos = (sa.len() - 1) / 2;
            let old = &self.old[sa[pos] as usize..];
            let len = old.len().min(new.len());
            sa = if old[..len] < new[..len] {
                &sa[pos..]
            } else {
                &sa[..=pos]
            };
        }
        assert!(!sa.is_empty());
        let a = mismatch(&old[sa[0] as usize..], new);
        let b = mismatch(&old[sa[sa.len() - 1] as usize..], new);
        if a > b {
            (sa[0] as usize, a)
        } else {
            (sa[sa.len() - 1] as usize, b)
        }
    }

    fn advance(&mut self) -> bool {
        let mut score = 0;
        self.scan += self.len;
        let mut subscan = self.scan;
        while self.scan < self.new.len() {
            (self.pos, self.len) = self.find_best_match();
            while subscan < self.scan + self.len {
                if subscan as isize + self.last_offset < self.old.len() as isize
                    && (self.old[(subscan as isize + self.last_offset) as usize]
                        == self.new[subscan])
                {
                    score += 1;
                }
                subscan += 1;
            }
            if (self.len == score && self.len != 0) || self.len > score + 8 {
                break;
            }
            if self.scan as isize + self.last_offset < self.old.len() as isize
                && (self.old[(self.scan as isize + self.last_offset) as usize]
                    == self.new[self.scan])
            {
                score -= 1;
            }
            self.scan += 1;
        }
        self.len != score || self.scan == self.new.len()
    }

    fn calc_add(&self) -> usize {
        let mut add = 0;
        let mut score = 0;
        let mut best = 0;
        let mut i = 0;
        while self.last_scan + i < self.scan && self.last_pos + i < self.old.len() {
            if self.old[self.last_pos + i] == self.new[self.last_scan + i] {
                score += 1;
            }
            i += 1;
            if score * 2 - i as i32 > best * 2 - add as i32 {
                best = score;
                add = i;
            }
        }
        add
    }

    fn calc_back(&self) -> usize {
        if self.scan >= self.new.len() {
            return 0;
        }
        let mut back = 0;
        let mut score = 0;
        let mut best = 0;
        let mut i = 1;
        while self.scan >= self.last_scan + i && self.pos >= i {
            if self.old[self.pos - i] == self.new[self.scan - i] {
                score += 1;
            }
            if score * 2 - i as isize > best * 2 - back as isize {
                best = score;
                back = i;
            }
            i += 1;
        }
        back
    }

    fn optimize_overlap(&self, mut add: usize, mut back: usize) -> (usize, usize) {
        if self.last_scan + add > self.scan - back {
            let overlap = self.last_scan + add - (self.scan - back);
            let mut score = 0;
            let mut best = 0;
            let mut forward = 0;
            for i in 0..overlap {
                if self.new[self.last_scan + add - overlap + i]
                    == self.old[self.last_pos + add - overlap + i]
                {
                    score += 1;
                }
                if self.new[self.scan - back + i] == self.old[self.pos - back + i] {
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
        (add, back)
    }

    fn calc_copy_seek(&self, add: usize, back: usize) -> (usize, isize) {
        let copy = self.scan - back - (self.last_scan + add);
        let seek = self.pos as isize - self.last_pos as isize - (back + add) as isize;
        (copy, seek)
    }

    fn commit(&mut self, back: usize) {
        self.last_scan = self.scan - back;
        self.last_pos = self.pos - back;
        self.last_offset = self.pos as isize - self.scan as isize;
    }
}
