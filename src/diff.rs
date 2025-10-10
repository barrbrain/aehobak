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

struct ScanState<'a> {
    sa: &'a [i32],
    old: &'a [u8],
    new: &'a [u8],
    scan: usize,
    len: usize,
    pos: usize,
    last_scan: usize,
    last_pos: usize,
    last_offset: i32,
}

fn diff_internal(old: &[u8], new: &[u8], writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    let mut sa = vec![0; old.len() + 1];
    sais(&mut sa, &mut vec![0; old.len() + 1], old);
    let sa = &sa;

    let mut scanner = ScanState {
        sa,
        old,
        new,
        scan: 0,
        len: 0,
        pos: 0,
        last_scan: 0,
        last_pos: 0,
        last_offset: 0,
    };

    let mut encoder = EncoderState::new();

    while scanner.scan < scanner.new.len() {
        let mut old_score = 0;
        scanner.scan += scanner.len;
        let mut scsc = scanner.scan;
        while scanner.scan < scanner.new.len() {
            (scanner.pos, scanner.len) =
                find_best_match(&scanner.sa, scanner.old, &scanner.new[scanner.scan..]);
            while scsc < scanner.scan + scanner.len {
                if scsc as i32 + scanner.last_offset < scanner.old.len() as _
                    && (scanner.old[(scsc as i32 + scanner.last_offset) as usize]
                        == scanner.new[scsc])
                {
                    old_score += 1;
                }
                scsc += 1;
            }
            if scanner.len == old_score && (scanner.len != 0) || scanner.len > old_score + 8 {
                break;
            }
            if scanner.scan as i32 + scanner.last_offset < scanner.old.len() as _
                && (scanner.old[(scanner.scan as i32 + scanner.last_offset) as usize]
                    == scanner.new[scanner.scan])
            {
                old_score -= 1;
            }
            scanner.scan += 1;
        }
        if !(scanner.len != old_score || scanner.scan == scanner.new.len()) {
            continue;
        }
        let mut add = 0usize;
        {
            let mut score = 0;
            let mut best = 0;
            let mut i = 0usize;
            while scanner.last_scan + i < scanner.scan
                && (scanner.last_pos + i < scanner.old.len() as _)
            {
                if scanner.old[scanner.last_pos + i] == scanner.new[scanner.last_scan + i] {
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
        if scanner.scan < scanner.new.len() {
            let mut score = 0i32;
            let mut best = 0;
            let mut i = 1;
            while scanner.scan >= scanner.last_scan + i && (scanner.pos >= i) {
                if scanner.old[scanner.pos - i] == scanner.new[scanner.scan - i] {
                    score += 1;
                }
                if score * 2 - i as i32 > best * 2 - lenb as i32 {
                    best = score;
                    lenb = i;
                }
                i += 1;
            }
        }
        if scanner.last_scan + add > scanner.scan - lenb {
            let overlap = scanner.last_scan + add - (scanner.scan - lenb);
            let mut score = 0;
            let mut best = 0;
            let mut lens = 0;
            for i in 0..overlap {
                if scanner.new[scanner.last_scan + add - overlap + i]
                    == scanner.old[scanner.last_pos + add - overlap + i]
                {
                    score += 1;
                }
                if scanner.new[scanner.scan - lenb + i] == scanner.old[scanner.pos - lenb + i] {
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
        let copy = scanner.scan - lenb - (scanner.last_scan + add);
        let seek = (scanner.pos - scanner.last_pos) as isize - (lenb + add) as isize;
        encoder.control(Aehobak {
            add: add as u32,
            copy: copy as u32,
            seek: seek as i32,
        });

        encoder.add(
            &scanner.old[scanner.last_pos..][..add],
            &scanner.new[scanner.last_scan..][..add],
        );

        let copy_from = scanner.last_scan + add;
        encoder.copy(&scanner.new[copy_from..][..copy]);

        scanner.last_scan = scanner.scan - lenb;
        scanner.last_pos = scanner.pos - lenb;
        scanner.last_offset = scanner.pos as i32 - scanner.scan as i32;
    }

    encoder.finalize(writer)
}
