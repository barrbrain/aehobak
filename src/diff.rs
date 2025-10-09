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

fn find_best_match(mut sa: &[i32], old: &[u8], new: &[u8]) -> (usize, usize) {
    while sa.len() > 2 {
        let pos = (sa.len() - 1) / 2;
        let old = &old[sa[pos] as usize..];
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

fn diff_internal(old: &[u8], new: &[u8], writer: &mut dyn Write) -> io::Result<()> {
    let mut sa = vec![0; old.len() + 1];
    sais(&mut sa, &mut vec![0; old.len() + 1], old);

    let mut encoder = EncoderState::new();

    let mut scan = 0;
    let mut len = 0usize;
    let mut pos = 0usize;
    let mut last_scan = 0;
    let mut last_pos = 0;
    let mut last_offset = 0i32;
    while scan < new.len() {
        let mut old_score = 0;
        scan += len;
        let mut scsc = scan;
        while scan < new.len() {
            (pos, len) = find_best_match(&sa, old, &new[scan..]);
            while scsc < scan + len {
                if scsc as i32 + last_offset < old.len() as _
                    && (old[(scsc as i32 + last_offset) as usize] == new[scsc])
                {
                    old_score += 1;
                }
                scsc += 1;
            }
            if len == old_score && (len != 0) || len > old_score + 8 {
                break;
            }
            if scan as i32 + last_offset < old.len() as _
                && (old[(scan as i32 + last_offset) as usize] == new[scan])
            {
                old_score -= 1;
            }
            scan += 1;
        }
        if !(len != old_score || scan == new.len()) {
            continue;
        }
        let mut add = 0usize;
        {
            let mut score = 0;
            let mut best = 0;
            let mut i = 0usize;
            while last_scan + i < scan && (last_pos + i < old.len() as _) {
                if old[last_pos + i] == new[last_scan + i] {
                    score += 1;
                }
                i += 1;
                if score * 2 - i as i32 <= best * 2 - add as i32 {
                    continue;
                }
                best = score;
                add = i;
            }
        }
        let mut lenb = 0;
        if scan < new.len() {
            let mut score = 0i32;
            let mut best = 0;
            let mut i = 1;
            while scan >= last_scan + i && (pos >= i) {
                if old[pos - i] == new[scan - i] {
                    score += 1;
                }
                if score * 2 - i as i32 > best * 2 - lenb as i32 {
                    best = score;
                    lenb = i;
                }
                i += 1;
            }
        }
        if last_scan + add > scan - lenb {
            let overlap = last_scan + add - (scan - lenb);
            let mut score = 0;
            let mut best = 0;
            let mut lens = 0;
            for i in 0..overlap {
                if new[last_scan + add - overlap + i] == old[last_pos + add - overlap + i] {
                    score += 1;
                }
                if new[scan - lenb + i] == old[pos - lenb + i] {
                    score -= 1;
                }
                if score > best {
                    best = score;
                    lens = i + 1;
                }
            }
            add = add + lens - overlap;
            lenb -= lens;
        }
        let copy = scan - lenb - (last_scan + add);
        let seek = (pos - last_pos) as isize - (lenb + add) as isize;
        encoder.control(Aehobak {
            add: add as u32,
            copy: copy as u32,
            seek: seek as i32,
        });

        encoder.add(&old[last_pos..][..add], &new[last_scan..][..add]);

        let copy_from = last_scan + add;
        encoder.copy(&new[copy_from..][..copy]);

        last_scan = scan - lenb;
        last_pos = pos - lenb;
        last_offset = pos as i32 - scan as i32;
    }

    encoder.finalize(writer)
}
