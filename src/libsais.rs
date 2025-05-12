/*--

This file is a part of libsais, a library for linear time suffix array,
longest common prefix array and burrows wheeler transform construction.

   Copyright (c) 2021-2025 Ilya Grebnov <ilya.grebnov@gmail.com>

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.

Please see the file LICENSE for full copyright information.

--*/

#![allow(
    clippy::needless_return,
    clippy::too_many_arguments,
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]

use std::ffi::{c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort, c_void};

extern "C" {
    fn malloc(_: c_ulong) -> *mut c_void;
    fn free(_: *mut c_void);
    fn memcpy(_: *mut c_void, _: *const c_void, _: c_ulong) -> *mut c_void;
    fn memmove(_: *mut c_void, _: *const c_void, _: c_ulong) -> *mut c_void;
    fn memset(_: *mut c_void, _: c_int, _: c_ulong) -> *mut c_void;
}
pub type __uint8_t = c_uchar;
pub type __uint16_t = c_ushort;
pub type __int32_t = c_int;
pub type __uint32_t = c_uint;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
pub struct LIBSAIS_CONTEXT {
    pub buckets: *mut sa_sint_t,
    pub thread_state: *mut LIBSAIS_THREAD_STATE,
    pub threads: fast_sint_t,
}
pub type fast_sint_t = ptrdiff_t;
pub type ptrdiff_t = c_long;
#[derive(Copy, Clone)]
pub union LIBSAIS_THREAD_STATE {
    pub state: C2RustUnnamed,
    pub padding: [uint8_t; 64],
}
#[derive(Copy, Clone)]
pub struct C2RustUnnamed {
    pub position: fast_sint_t,
    pub count: fast_sint_t,
    pub m: fast_sint_t,
    pub last_lms_suffix: fast_sint_t,
    pub buckets: *mut sa_sint_t,
    pub cache: *mut LIBSAIS_THREAD_CACHE,
}
#[derive(Copy, Clone)]
pub struct LIBSAIS_THREAD_CACHE {
    pub symbol: sa_sint_t,
    pub index: sa_sint_t,
}
pub type sa_sint_t = int32_t;
pub type size_t = c_ulong;
pub type sa_uint_t = uint32_t;
pub type fast_uint_t = size_t;
#[derive(Copy, Clone)]
pub struct LIBSAIS_UNBWT_CONTEXT {
    pub bucket2: *mut sa_uint_t,
    pub fastbits: *mut uint16_t,
    pub buckets: *mut sa_uint_t,
    pub threads: fast_sint_t,
}
unsafe fn libsais_prefetchr<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    core::arch::x86_64::_mm_prefetch(
        core::mem::transmute::<*const T, *mut i8>(p),
        core::arch::x86_64::_MM_HINT_T0,
    );
}
unsafe fn libsais_prefetchw<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    core::arch::x86_64::_mm_prefetch(
        core::mem::transmute::<*const T, *mut i8>(p),
        core::arch::x86_64::_MM_HINT_ET0,
    );
}
unsafe fn libsais_align_up(
    mut address: *const c_void,
    mut alignment: size_t,
) -> *mut c_void {
    ((address as ptrdiff_t + alignment as ptrdiff_t - 1) & -(alignment as ptrdiff_t)) as *mut c_void
}
unsafe fn libsais_alloc_aligned(mut size: size_t, mut alignment: size_t) -> *mut c_void {
    let mut address: *mut c_void = malloc(
        size.wrapping_add(size_of::<c_short>() as c_ulong)
            .wrapping_add(alignment)
            .wrapping_sub(1),
    );
    if !address.is_null() {
        let mut aligned_address: *mut c_void = libsais_align_up(
            (address as ptrdiff_t + size_of::<c_short>() as c_ulong as ptrdiff_t) as *mut c_void,
            alignment,
        );
        *(aligned_address as *mut c_short).offset(-1_isize) =
            (aligned_address as ptrdiff_t - address as ptrdiff_t) as c_short;
        return aligned_address;
    }
    std::ptr::null_mut::<c_void>()
}
unsafe fn libsais_free_aligned(mut aligned_address: *mut c_void) {
    if !aligned_address.is_null() {
        free(
            (aligned_address as ptrdiff_t
                - *(aligned_address as *mut c_short).offset(-1_isize) as c_long)
                as *mut c_void,
        );
    }
}
unsafe fn libsais_alloc_thread_state(
    mut threads: sa_sint_t,
) -> *mut LIBSAIS_THREAD_STATE {
    let mut thread_state: *mut LIBSAIS_THREAD_STATE = libsais_alloc_aligned(
        (threads as size_t).wrapping_mul(size_of::<LIBSAIS_THREAD_STATE>() as c_ulong),
        4096 as size_t,
    ) as *mut LIBSAIS_THREAD_STATE;
    let mut thread_buckets: *mut sa_sint_t = libsais_alloc_aligned(
        (threads as size_t)
            .wrapping_mul(4)
            .wrapping_mul(((1) << 8) as c_ulong)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        4096 as size_t,
    ) as *mut sa_sint_t;
    let mut thread_cache: *mut LIBSAIS_THREAD_CACHE = libsais_alloc_aligned(
        (threads as size_t)
            .wrapping_mul(24576)
            .wrapping_mul(size_of::<LIBSAIS_THREAD_CACHE>() as c_ulong),
        4096 as size_t,
    ) as *mut LIBSAIS_THREAD_CACHE;
    if !thread_state.is_null() && !thread_buckets.is_null() && !thread_cache.is_null() {
        let mut t: fast_sint_t = 0;
        t = 0 as fast_sint_t;
        while t < threads as c_long {
            let fresh0 = &mut (*thread_state.offset(t as isize)).state.buckets;
            *fresh0 = thread_buckets;
            thread_buckets = thread_buckets.offset((4 * ((1) << 8)) as isize);
            let fresh1 = &mut (*thread_state.offset(t as isize)).state.cache;
            *fresh1 = thread_cache;
            thread_cache = thread_cache.offset(24576);
            t += 1;
        }
        return thread_state;
    }
    libsais_free_aligned(thread_cache as *mut c_void);
    libsais_free_aligned(thread_buckets as *mut c_void);
    libsais_free_aligned(thread_state as *mut c_void);
    std::ptr::null_mut::<LIBSAIS_THREAD_STATE>()
}
unsafe fn libsais_free_thread_state(mut thread_state: *mut LIBSAIS_THREAD_STATE) {
    if !thread_state.is_null() {
        libsais_free_aligned((*thread_state).state.cache as *mut c_void);
        libsais_free_aligned((*thread_state).state.buckets as *mut c_void);
        libsais_free_aligned(thread_state as *mut c_void);
    }
}
unsafe fn libsais_flip_suffix_markers_omp(
    mut SA: *mut sa_sint_t,
    mut l: sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (l as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        l as c_long - omp_block_start
    };
    let mut i: fast_sint_t = 0;
    i = omp_block_start;
    while i < omp_block_start + omp_block_size {
        let fresh2 = &mut (*SA.offset(i as isize));
        *fresh2 ^= -(2147483647) - 1;
        i += 1;
    }
}
unsafe fn libsais_gather_lms_suffixes_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: fast_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    if omp_block_size > 0 {
        let prefetch_distance: fast_sint_t = 128 as fast_sint_t;
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = omp_block_start + omp_block_size;
        let mut c0: fast_sint_t =
            *T.offset((omp_block_start + omp_block_size - 1) as isize) as fast_sint_t;
        let mut c1: fast_sint_t = -(1) as fast_sint_t;
        while j < n as c_long && {
            c1 = *T.offset(j as isize) as fast_sint_t;
            c1 == c0
        } {
            j += 1;
        }
        let mut f0: fast_uint_t = (c0 >= c1) as c_int as fast_uint_t;
        let mut f1: fast_uint_t = 0 as fast_uint_t;
        i = omp_block_start + omp_block_size - 2;
        j = omp_block_start + 3;
        while i >= j {
            libsais_prefetchr(T.offset((i - prefetch_distance) as isize));
            c1 = *T.offset(i as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            c0 = *T.offset((i - 1) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = i as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            c1 = *T.offset((i - 2) as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            c0 = *T.offset((i - 3) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 2) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            i -= 4;
        }
        j -= 3;
        while i >= j {
            c1 = c0;
            c0 = *T.offset(i as isize) as fast_sint_t;
            f1 = f0;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            i -= 1;
        }
        *SA.offset(m as isize) = (i + 1) as sa_sint_t;
    }
}
unsafe fn libsais_gather_lms_suffixes_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (n as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        n as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_gather_lms_suffixes_8u(
            T,
            SA,
            n,
            n as fast_sint_t - 1,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_gather_lms_suffixes_32s(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: sa_sint_t = n - 2;
    let mut m: sa_sint_t = n - 1;
    let mut f0: fast_uint_t = 1 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset((n - 1) as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    while i >= 3 {
        libsais_prefetchr(T.offset((i as c_long - prefetch_distance) as isize));
        c1 = *T.offset(i as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i + 1;
        m -= (f1 & !f0) as sa_sint_t;
        c0 = *T.offset((i - 1) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i;
        m -= (f0 & !f1) as sa_sint_t;
        c1 = *T.offset((i - 2) as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i - 1;
        m -= (f1 & !f0) as sa_sint_t;
        c0 = *T.offset((i - 3) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i - 2;
        m -= (f0 & !f1) as sa_sint_t;
        i -= 4;
    }
    while i >= 0 {
        c1 = c0;
        c0 = *T.offset(i as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i + 1;
        m -= (f0 & !f1) as sa_sint_t;
        i -= 1;
    }
    n - 1 - m
}
unsafe fn libsais_gather_compacted_lms_suffixes_32s(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: sa_sint_t = n - 2;
    let mut m: sa_sint_t = n - 1;
    let mut f0: fast_uint_t = 1 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset((n - 1) as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    while i >= 3 {
        libsais_prefetchr(T.offset((i as c_long - prefetch_distance) as isize));
        c1 = *T.offset(i as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i + 1;
        m -= (f1 & !f0 & (c0 >= 0) as c_int as c_ulong) as sa_sint_t;
        c0 = *T.offset((i - 1) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i;
        m -= (f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as sa_sint_t;
        c1 = *T.offset((i - 2) as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i - 1;
        m -= (f1 & !f0 & (c0 >= 0) as c_int as c_ulong) as sa_sint_t;
        c0 = *T.offset((i - 3) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i - 2;
        m -= (f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as sa_sint_t;
        i -= 4;
    }
    while i >= 0 {
        c1 = c0;
        c0 = *T.offset(i as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = i + 1;
        m -= (f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as sa_sint_t;
        i -= 1;
    }
    n - 1 - m
}
unsafe fn libsais_count_lms_suffixes_32s_2k(
    mut T: *const sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    memset(
        buckets as *mut c_void,
        0,
        (2 as c_ulong)
            .wrapping_mul(k as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut i: sa_sint_t = n - 2;
    let mut f0: fast_uint_t = 1 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset((n - 1) as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    while i as c_long >= prefetch_distance + 3 {
        libsais_prefetchr(T.offset((i as c_long - 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            buckets.offset((*T.offset((i as c_long - prefetch_distance) as isize) << 1) as isize),
        );
        libsais_prefetchw(
            buckets
                .offset((*T.offset((i as c_long - prefetch_distance - 1) as isize) << 1) as isize),
        );
        libsais_prefetchw(
            buckets
                .offset((*T.offset((i as c_long - prefetch_distance - 2) as isize) << 1) as isize),
        );
        libsais_prefetchw(
            buckets
                .offset((*T.offset((i as c_long - prefetch_distance - 3) as isize) << 1) as isize),
        );
        c1 = *T.offset(i as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        let fresh3 =
            &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
        *fresh3 += 1;
        c0 = *T.offset((i - 1) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        let fresh4 =
            &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
        *fresh4 += 1;
        c1 = *T.offset((i - 2) as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        let fresh5 =
            &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
        *fresh5 += 1;
        c0 = *T.offset((i - 3) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        let fresh6 =
            &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
        *fresh6 += 1;
        i -= 4;
    }
    while i >= 0 {
        c1 = c0;
        c0 = *T.offset(i as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        let fresh7 =
            &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
        *fresh7 += 1;
        i -= 1;
    }
    let fresh8 = &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(0) as isize));
    *fresh8 += 1;
}
unsafe fn libsais_count_and_gather_lms_suffixes_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    memset(
        buckets as *mut c_void,
        0,
        (4 as size_t)
            .wrapping_mul(((1) << 8) as c_ulong)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut m: fast_sint_t = omp_block_start + omp_block_size - 1;
    if omp_block_size > 0 {
        let prefetch_distance: fast_sint_t = 128 as fast_sint_t;
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = m + 1;
        let mut c0: fast_sint_t = *T.offset(m as isize) as fast_sint_t;
        let mut c1: fast_sint_t = -(1) as fast_sint_t;
        while j < n as c_long && {
            c1 = *T.offset(j as isize) as fast_sint_t;
            c1 == c0
        } {
            j += 1;
        }
        let mut f0: fast_uint_t = (c0 >= c1) as c_int as fast_uint_t;
        let mut f1: fast_uint_t = 0 as fast_uint_t;
        i = m - 1;
        j = omp_block_start + 3;
        while i >= j {
            libsais_prefetchr(T.offset((i - prefetch_distance) as isize));
            c1 = *T.offset(i as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh9 = &mut (*buckets.offset(
                ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1))
                    as isize,
            ));
            *fresh9 += 1;
            c0 = *T.offset((i - 1) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = i as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh10 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh10 += 1;
            c1 = *T.offset((i - 2) as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh11 = &mut (*buckets.offset(
                ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1))
                    as isize,
            ));
            *fresh11 += 1;
            c0 = *T.offset((i - 3) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 2) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh12 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh12 += 1;
            i -= 4;
        }
        j -= 3;
        while i >= j {
            c1 = c0;
            c0 = *T.offset(i as isize) as fast_sint_t;
            f1 = f0;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh13 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh13 += 1;
            i -= 1;
        }
        c1 = (if i >= 0 {
            *T.offset(i as isize) as c_int
        } else {
            -(1)
        }) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = (i + 1) as sa_sint_t;
        m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
        let fresh14 = &mut (*buckets.offset(
            ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1)) as isize,
        ));
        *fresh14 += 1;
    }
    (omp_block_start + omp_block_size - 1 - m) as sa_sint_t
}
unsafe fn libsais_count_and_gather_lms_suffixes_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (n as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        n as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        m = libsais_count_and_gather_lms_suffixes_8u(
            T,
            SA,
            n,
            buckets,
            omp_block_start,
            omp_block_size,
        );
    }
    m
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_4k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    memset(
        buckets as *mut c_void,
        0,
        (4 as c_ulong)
            .wrapping_mul(k as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut m: fast_sint_t = omp_block_start + omp_block_size - 1;
    if omp_block_size > 0 {
        let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = m + 1;
        let mut c0: fast_sint_t = *T.offset(m as isize) as fast_sint_t;
        let mut c1: fast_sint_t = -(1) as fast_sint_t;
        while j < n as c_long && {
            c1 = *T.offset(j as isize) as fast_sint_t;
            c1 == c0
        } {
            j += 1;
        }
        let mut f0: fast_uint_t = (c0 >= c1) as c_int as fast_uint_t;
        let mut f1: fast_uint_t = 0 as fast_uint_t;
        i = m - 1;
        j = omp_block_start + prefetch_distance + 3;
        while i >= j {
            libsais_prefetchr(T.offset((i - 2 * prefetch_distance) as isize));
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance) as isize) << 2) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 1) as isize) << 2) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 2) as isize) << 2) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 3) as isize) << 2) as isize),
            );
            c1 = *T.offset(i as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh15 = &mut (*buckets.offset(
                ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1))
                    as isize,
            ));
            *fresh15 += 1;
            c0 = *T.offset((i - 1) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = i as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh16 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh16 += 1;
            c1 = *T.offset((i - 2) as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh17 = &mut (*buckets.offset(
                ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1))
                    as isize,
            ));
            *fresh17 += 1;
            c0 = *T.offset((i - 3) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 2) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh18 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh18 += 1;
            i -= 4;
        }
        j -= prefetch_distance + 3;
        while i >= j {
            c1 = c0;
            c0 = *T.offset(i as isize) as fast_sint_t;
            f1 = f0;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh19 = &mut (*buckets.offset(
                ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0))
                    as isize,
            ));
            *fresh19 += 1;
            i -= 1;
        }
        c1 = (if i >= 0 { *T.offset(i as isize) } else { -(1) }) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = (i + 1) as sa_sint_t;
        m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
        let fresh20 = &mut (*buckets.offset(
            ((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0).wrapping_add(f1)) as isize,
        ));
        *fresh20 += 1;
    }
    (omp_block_start + omp_block_size - 1 - m) as sa_sint_t
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_2k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    memset(
        buckets as *mut c_void,
        0,
        (2 as c_ulong)
            .wrapping_mul(k as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut m: fast_sint_t = omp_block_start + omp_block_size - 1;
    if omp_block_size > 0 {
        let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = m + 1;
        let mut c0: fast_sint_t = *T.offset(m as isize) as fast_sint_t;
        let mut c1: fast_sint_t = -(1) as fast_sint_t;
        while j < n as c_long && {
            c1 = *T.offset(j as isize) as fast_sint_t;
            c1 == c0
        } {
            j += 1;
        }
        let mut f0: fast_uint_t = (c0 >= c1) as c_int as fast_uint_t;
        let mut f1: fast_uint_t = 0 as fast_uint_t;
        i = m - 1;
        j = omp_block_start + prefetch_distance + 3;
        while i >= j {
            libsais_prefetchr(T.offset((i - 2 * prefetch_distance) as isize));
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance) as isize) << 1) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 1) as isize) << 1) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 2) as isize) << 1) as isize),
            );
            libsais_prefetchw(
                buckets.offset((*T.offset((i - prefetch_distance - 3) as isize) << 1) as isize),
            );
            c1 = *T.offset(i as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh21 =
                &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
            *fresh21 += 1;
            c0 = *T.offset((i - 1) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = i as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh22 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh22 += 1;
            c1 = *T.offset((i - 2) as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
            let fresh23 =
                &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
            *fresh23 += 1;
            c0 = *T.offset((i - 3) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 2) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh24 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh24 += 1;
            i -= 4;
        }
        j -= prefetch_distance + 3;
        while i >= j {
            c1 = c0;
            c0 = *T.offset(i as isize) as fast_sint_t;
            f1 = f0;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1) as fast_sint_t as fast_sint_t;
            let fresh25 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh25 += 1;
            i -= 1;
        }
        c1 = (if i >= 0 { *T.offset(i as isize) } else { -(1) }) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = (i + 1) as sa_sint_t;
        m = (m as c_ulong).wrapping_sub(f1 & !f0) as fast_sint_t as fast_sint_t;
        let fresh26 =
            &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
        *fresh26 += 1;
    }
    (omp_block_start + omp_block_size - 1 - m) as sa_sint_t
}
unsafe fn libsais_count_and_gather_compacted_lms_suffixes_32s_2k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    memset(
        buckets as *mut c_void,
        0,
        (2 as c_ulong)
            .wrapping_mul(k as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut m: fast_sint_t = omp_block_start + omp_block_size - 1;
    if omp_block_size > 0 {
        let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = m + 1;
        let mut c0: fast_sint_t = *T.offset(m as isize) as fast_sint_t;
        let mut c1: fast_sint_t = -(1) as fast_sint_t;
        while j < n as c_long && {
            c1 = *T.offset(j as isize) as fast_sint_t;
            c1 == c0
        } {
            j += 1;
        }
        let mut f0: fast_uint_t = (c0 >= c1) as c_int as fast_uint_t;
        let mut f1: fast_uint_t = 0 as fast_uint_t;
        i = m - 1;
        j = omp_block_start + prefetch_distance + 3;
        while i >= j {
            libsais_prefetchr(T.offset((i - 2 * prefetch_distance) as isize));
            libsais_prefetchw(buckets.offset(
                ((*T.offset((i - prefetch_distance) as isize) & 2147483647) << 1) as isize,
            ));
            libsais_prefetchw(buckets.offset(
                ((*T.offset((i - prefetch_distance - 1) as isize) & 2147483647) << 1) as isize,
            ));
            libsais_prefetchw(buckets.offset(
                ((*T.offset((i - prefetch_distance - 2) as isize) & 2147483647) << 1) as isize,
            ));
            libsais_prefetchw(buckets.offset(
                ((*T.offset((i - prefetch_distance - 3) as isize) & 2147483647) << 1) as isize,
            ));
            c1 = *T.offset(i as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0 & (c0 >= 0) as c_int as c_ulong) as fast_sint_t
                as fast_sint_t;
            c0 &= 2147483647;
            let fresh27 =
                &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
            *fresh27 += 1;
            c0 = *T.offset((i - 1) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = i as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as fast_sint_t
                as fast_sint_t;
            c1 &= 2147483647;
            let fresh28 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh28 += 1;
            c1 = *T.offset((i - 2) as isize) as fast_sint_t;
            f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f1 & !f0 & (c0 >= 0) as c_int as c_ulong) as fast_sint_t
                as fast_sint_t;
            c0 &= 2147483647;
            let fresh29 =
                &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
            *fresh29 += 1;
            c0 = *T.offset((i - 3) as isize) as fast_sint_t;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i - 2) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as fast_sint_t
                as fast_sint_t;
            c1 &= 2147483647;
            let fresh30 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh30 += 1;
            i -= 4;
        }
        j -= prefetch_distance + 3;
        while i >= j {
            c1 = c0;
            c0 = *T.offset(i as isize) as fast_sint_t;
            f1 = f0;
            f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
            *SA.offset(m as isize) = (i + 1) as sa_sint_t;
            m = (m as c_ulong).wrapping_sub(f0 & !f1 & (c1 >= 0) as c_int as c_ulong) as fast_sint_t
                as fast_sint_t;
            c1 &= 2147483647;
            let fresh31 =
                &mut (*buckets.offset(((c1 as fast_uint_t) << 1).wrapping_add(f0 & !f1) as isize));
            *fresh31 += 1;
            i -= 1;
        }
        c1 = (if i >= 0 { *T.offset(i as isize) } else { -(1) }) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        *SA.offset(m as isize) = (i + 1) as sa_sint_t;
        m = (m as c_ulong).wrapping_sub(f1 & !f0 & (c0 >= 0) as c_int as c_ulong) as fast_sint_t
            as fast_sint_t;
        c0 &= 2147483647;
        let fresh32 =
            &mut (*buckets.offset(((c0 as fast_uint_t) << 1).wrapping_add(f1 & !f0) as isize));
        *fresh32 += 1;
    }
    (omp_block_start + omp_block_size - 1 - m) as sa_sint_t
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_4k_nofs_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    if omp_num_threads == 1 {
        m = libsais_count_and_gather_lms_suffixes_32s_4k(
            T,
            SA,
            n,
            k,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    m
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_2k_nofs_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    if omp_num_threads == 1 {
        m = libsais_count_and_gather_lms_suffixes_32s_2k(
            T,
            SA,
            n,
            k,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    m
}
unsafe fn libsais_count_and_gather_compacted_lms_suffixes_32s_2k_nofs_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    if omp_num_threads == 1 {
        m = libsais_count_and_gather_compacted_lms_suffixes_32s_2k(
            T,
            SA,
            n,
            k,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    m
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_4k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    m = libsais_count_and_gather_lms_suffixes_32s_4k_nofs_omp(T, SA, n, k, buckets, threads);
    m
}
unsafe fn libsais_count_and_gather_lms_suffixes_32s_2k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut m: sa_sint_t = 0;
    m = libsais_count_and_gather_lms_suffixes_32s_2k_nofs_omp(T, SA, n, k, buckets, threads);
    m
}
unsafe fn libsais_count_and_gather_compacted_lms_suffixes_32s_2k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_count_and_gather_compacted_lms_suffixes_32s_2k_nofs_omp(T, SA, n, k, buckets, threads);
}
unsafe fn libsais_count_suffixes_32s(
    mut T: *const sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    memset(
        buckets as *mut c_void,
        0,
        (k as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = 0 as fast_sint_t;
    j = n as fast_sint_t - 7;
    while i < j {
        libsais_prefetchr(T.offset((i + prefetch_distance) as isize));
        let fresh33 = &mut (*buckets.offset(*T.offset(i as isize) as isize));
        *fresh33 += 1;
        let fresh34 = &mut (*buckets.offset(*T.offset((i + 1) as isize) as isize));
        *fresh34 += 1;
        let fresh35 = &mut (*buckets.offset(*T.offset((i + 2) as isize) as isize));
        *fresh35 += 1;
        let fresh36 = &mut (*buckets.offset(*T.offset((i + 3) as isize) as isize));
        *fresh36 += 1;
        let fresh37 = &mut (*buckets.offset(*T.offset((i + 4) as isize) as isize));
        *fresh37 += 1;
        let fresh38 = &mut (*buckets.offset(*T.offset((i + 5) as isize) as isize));
        *fresh38 += 1;
        let fresh39 = &mut (*buckets.offset(*T.offset((i + 6) as isize) as isize));
        *fresh39 += 1;
        let fresh40 = &mut (*buckets.offset(*T.offset((i + 7) as isize) as isize));
        *fresh40 += 1;
        i += 8;
    }
    j += 7;
    while i < j {
        let fresh41 = &mut (*buckets.offset(*T.offset(i as isize) as isize));
        *fresh41 += 1;
        i += 1;
    }
}
unsafe fn libsais_initialize_buckets_start_and_end_8u(
    mut buckets: *mut sa_sint_t,
    mut freq: *mut sa_sint_t,
) -> sa_sint_t {
    let mut bucket_start: *mut sa_sint_t =
        &mut *buckets.offset((6 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut bucket_end: *mut sa_sint_t =
        &mut *buckets.offset((7 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut k: fast_sint_t = -(1) as fast_sint_t;
    if !freq.is_null() {
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = 0;
        let mut sum: sa_sint_t = 0;
        i = 0 as fast_sint_t;
        j = 0 as fast_sint_t;
        while i <= ((((1) << 8) - 1) << 2) as c_long {
            let mut total: sa_sint_t = *buckets.offset((i + 0 as c_long) as isize)
                + *buckets.offset((i + 1 as c_long) as isize)
                + *buckets.offset((i + 2 as c_long) as isize)
                + *buckets.offset((i + 3 as c_long) as isize);
            *bucket_start.offset(j as isize) = sum;
            sum += total;
            *bucket_end.offset(j as isize) = sum;
            k = if total > 0 { j } else { k };
            *freq.offset(j as isize) = total;
            i += ((1) << 2) as c_long;
            j += 1;
        }
    } else {
        let mut i_0: fast_sint_t = 0;
        let mut j_0: fast_sint_t = 0;
        let mut sum_0: sa_sint_t = 0;
        i_0 = 0 as fast_sint_t;
        j_0 = 0 as fast_sint_t;
        while i_0 <= ((((1) << 8) - 1) << 2) as c_long {
            let mut total_0: sa_sint_t = *buckets.offset((i_0 + 0 as c_long) as isize)
                + *buckets.offset((i_0 + 1 as c_long) as isize)
                + *buckets.offset((i_0 + 2 as c_long) as isize)
                + *buckets.offset((i_0 + 3 as c_long) as isize);
            *bucket_start.offset(j_0 as isize) = sum_0;
            sum_0 += total_0;
            *bucket_end.offset(j_0 as isize) = sum_0;
            k = if total_0 > 0 { j_0 } else { k };
            i_0 += ((1) << 2) as c_long;
            j_0 += 1;
        }
    }
    (k + 1) as sa_sint_t
}
unsafe fn libsais_initialize_buckets_start_and_end_32s_6k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut bucket_start: *mut sa_sint_t =
        &mut *buckets.offset((4 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut bucket_end: *mut sa_sint_t =
        &mut *buckets.offset((5 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 2) {
        *bucket_start.offset(j as isize) = sum;
        sum += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 1 as c_long) as isize)
            + *buckets.offset((i + 2 as c_long) as isize)
            + *buckets.offset((i + 3 as c_long) as isize);
        *bucket_end.offset(j as isize) = sum;
        i += ((1) << 2) as c_long;
        j += 1;
    }
}
unsafe fn libsais_initialize_buckets_start_and_end_32s_4k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut bucket_start: *mut sa_sint_t =
        &mut *buckets.offset((2 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut bucket_end: *mut sa_sint_t =
        &mut *buckets.offset((3 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        *bucket_start.offset(j as isize) = sum;
        sum += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 1 as c_long) as isize);
        *bucket_end.offset(j as isize) = sum;
        i += ((1) << 1) as c_long;
        j += 1;
    }
}
unsafe fn libsais_initialize_buckets_end_32s_2k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut i: fast_sint_t = 0;
    let mut sum0: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        sum0 += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 1 as c_long) as isize);
        *buckets.offset((i + 0 as c_long) as isize) = sum0;
        i += ((1) << 1) as c_long;
    }
}
unsafe fn libsais_initialize_buckets_start_and_end_32s_2k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        *buckets.offset(j as isize) = *buckets.offset(i as isize);
        i += ((1) << 1) as c_long;
        j += 1;
    }
    *buckets.offset(k as isize) = 0;
    memcpy(
        &mut *buckets.offset((k + 1) as isize) as *mut sa_sint_t as *mut c_void,
        buckets as *const c_void,
        (k as size_t)
            .wrapping_sub(1)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_initialize_buckets_start_32s_1k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut i: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    while i < k as fast_sint_t {
        let mut tmp: sa_sint_t = *buckets.offset(i as isize);
        *buckets.offset(i as isize) = sum;
        sum += tmp;
        i += 1;
    }
}
unsafe fn libsais_initialize_buckets_end_32s_1k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut i: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    while i < k as fast_sint_t {
        sum += *buckets.offset(i as isize);
        *buckets.offset(i as isize) = sum;
        i += 1;
    }
}
unsafe fn libsais_initialize_buckets_for_lms_suffixes_radix_sort_8u(
    mut T: *const uint8_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
) -> sa_sint_t {
    let mut f0: fast_uint_t = 0 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset(first_lms_suffix as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    loop {
        first_lms_suffix -= 1;
        if first_lms_suffix < 0 {
            break;
        }
        c1 = c0;
        c0 = *T.offset(first_lms_suffix as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        let fresh42 = &mut (*buckets.offset(
            ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0)) as isize,
        ));
        *fresh42 -= 1;
    }
    let fresh43 = &mut (*buckets
        .offset(((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0)) as isize));
    *fresh43 -= 1;
    let mut temp_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((((1) << 8) - 1) << 2) as c_long {
        *temp_bucket.offset((j + 1 as c_long) as isize) = sum;
        sum += *buckets.offset((i + 1 as c_long) as isize)
            + *buckets.offset((i + 3 as c_long) as isize);
        *temp_bucket.offset(j as isize) = sum;
        i += ((1) << 2) as c_long;
        j += ((1) << 1) as c_long;
    }
    sum
}
unsafe fn libsais_initialize_buckets_for_lms_suffixes_radix_sort_32s_2k(
    mut T: *const sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
) {
    let fresh44 = &mut (*buckets.offset((*T.offset(first_lms_suffix as isize) << 1) as isize));
    *fresh44 += 1;
    let fresh45 =
        &mut (*buckets.offset(((*T.offset(first_lms_suffix as isize) << 1) + 1) as isize));
    *fresh45 -= 1;
    let mut i: fast_sint_t = 0;
    let mut sum0: sa_sint_t = 0;
    let mut sum1: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        sum0 += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 1 as c_long) as isize);
        sum1 += *buckets.offset((i + 1 as c_long) as isize);
        *buckets.offset((i + 0 as c_long) as isize) = sum0;
        *buckets.offset((i + 1 as c_long) as isize) = sum1;
        i += ((1) << 1) as c_long;
    }
}
unsafe fn libsais_initialize_buckets_for_lms_suffixes_radix_sort_32s_6k(
    mut T: *const sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
) -> sa_sint_t {
    let mut f0: fast_uint_t = 0 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset(first_lms_suffix as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    loop {
        first_lms_suffix -= 1;
        if first_lms_suffix < 0 {
            break;
        }
        c1 = c0;
        c0 = *T.offset(first_lms_suffix as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        let fresh46 = &mut (*buckets.offset(
            ((c1 as fast_uint_t) << 2).wrapping_add(f1.wrapping_add(f1).wrapping_add(f0)) as isize,
        ));
        *fresh46 -= 1;
    }
    let fresh47 = &mut (*buckets
        .offset(((c0 as fast_uint_t) << 2).wrapping_add(f0.wrapping_add(f0)) as isize));
    *fresh47 -= 1;
    let mut temp_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 2) {
        sum += *buckets.offset((i + 1 as c_long) as isize)
            + *buckets.offset((i + 3 as c_long) as isize);
        *temp_bucket.offset(j as isize) = sum;
        i += ((1) << 2) as c_long;
        j += 1;
    }
    sum
}
unsafe fn libsais_initialize_buckets_for_radix_and_partial_sorting_32s_4k(
    mut T: *const sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
) {
    let mut bucket_start: *mut sa_sint_t =
        &mut *buckets.offset((2 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut bucket_end: *mut sa_sint_t =
        &mut *buckets.offset((3 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let fresh48 = &mut (*buckets.offset((*T.offset(first_lms_suffix as isize) << 1) as isize));
    *fresh48 += 1;
    let fresh49 =
        &mut (*buckets.offset(((*T.offset(first_lms_suffix as isize) << 1) + 1) as isize));
    *fresh49 -= 1;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum0: sa_sint_t = 0;
    let mut sum1: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        *bucket_start.offset(j as isize) = sum1;
        sum0 += *buckets.offset((i + 1 as c_long) as isize);
        sum1 += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 1 as c_long) as isize);
        *buckets.offset((i + 1 as c_long) as isize) = sum0;
        *bucket_end.offset(j as isize) = sum1;
        i += ((1) << 1) as c_long;
        j += 1;
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 3;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 2 * prefetch_distance) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 1) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 2) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 3) as isize) as isize));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        let fresh50 =
            &mut (*induction_bucket.offset(((*T.offset(p0 as isize) as c_int) << 1) as isize));
        *fresh50 -= 1;
        *SA.offset(*fresh50 as isize) = p0;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        let fresh51 =
            &mut (*induction_bucket.offset(((*T.offset(p1 as isize) as c_int) << 1) as isize));
        *fresh51 -= 1;
        *SA.offset(*fresh51 as isize) = p1;
        let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
        let fresh52 =
            &mut (*induction_bucket.offset(((*T.offset(p2 as isize) as c_int) << 1) as isize));
        *fresh52 -= 1;
        *SA.offset(*fresh52 as isize) = p2;
        let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
        let fresh53 =
            &mut (*induction_bucket.offset(((*T.offset(p3 as isize) as c_int) << 1) as isize));
        *fresh53 -= 1;
        *SA.offset(*fresh53 as isize) = p3;
        i -= 4;
    }
    j -= prefetch_distance + 3;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        let fresh54 =
            &mut (*induction_bucket.offset(((*T.offset(p as isize) as c_int) << 1) as isize));
        *fresh54 -= 1;
        *SA.offset(*fresh54 as isize) = p;
        i -= 1;
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut flags: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if flags & 2 != 0 {
        let fresh55 = &mut (*buckets.offset((4 * ((1) << 8)) as isize));
        *fresh55 -= 1;
    }
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    if omp_num_threads == 1 {
        libsais_radix_sort_lms_suffixes_8u(
            T,
            SA,
            &mut *buckets.offset((4 * ((1) << 8)) as isize),
            n as fast_sint_t - m as fast_sint_t + 1,
            m as fast_sint_t - 1,
        );
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_32s_6k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 3;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 3 * prefetch_distance) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 1) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 2) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 3) as isize) as isize));
        libsais_prefetchw(
            induction_bucket
                .offset(*T.offset(*SA.offset((i - prefetch_distance) as isize) as isize) as isize),
        );
        libsais_prefetchw(
            induction_bucket.offset(
                *T.offset(*SA.offset((i - prefetch_distance - 1) as isize) as isize) as isize,
            ),
        );
        libsais_prefetchw(
            induction_bucket.offset(
                *T.offset(*SA.offset((i - prefetch_distance - 2) as isize) as isize) as isize,
            ),
        );
        libsais_prefetchw(
            induction_bucket.offset(
                *T.offset(*SA.offset((i - prefetch_distance - 3) as isize) as isize) as isize,
            ),
        );
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        let fresh56 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
        *fresh56 -= 1;
        *SA.offset(*fresh56 as isize) = p0;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        let fresh57 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
        *fresh57 -= 1;
        *SA.offset(*fresh57 as isize) = p1;
        let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
        let fresh58 = &mut (*induction_bucket.offset(*T.offset(p2 as isize) as isize));
        *fresh58 -= 1;
        *SA.offset(*fresh58 as isize) = p2;
        let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
        let fresh59 = &mut (*induction_bucket.offset(*T.offset(p3 as isize) as isize));
        *fresh59 -= 1;
        *SA.offset(*fresh59 as isize) = p3;
        i -= 4;
    }
    j -= 2 * prefetch_distance + 3;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        let fresh60 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
        *fresh60 -= 1;
        *SA.offset(*fresh60 as isize) = p;
        i -= 1;
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_32s_2k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 3;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 3 * prefetch_distance) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 1) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 2) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - 2 * prefetch_distance - 3) as isize) as isize));
        libsais_prefetchw(induction_bucket.offset(
            (*T.offset(*SA.offset((i - prefetch_distance) as isize) as isize) << 1) as isize,
        ));
        libsais_prefetchw(induction_bucket.offset(
            (*T.offset(*SA.offset((i - prefetch_distance - 1) as isize) as isize) << 1) as isize,
        ));
        libsais_prefetchw(induction_bucket.offset(
            (*T.offset(*SA.offset((i - prefetch_distance - 2) as isize) as isize) << 1) as isize,
        ));
        libsais_prefetchw(induction_bucket.offset(
            (*T.offset(*SA.offset((i - prefetch_distance - 3) as isize) as isize) << 1) as isize,
        ));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        let fresh61 = &mut (*induction_bucket.offset((*T.offset(p0 as isize) << 1) as isize));
        *fresh61 -= 1;
        *SA.offset(*fresh61 as isize) = p0;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        let fresh62 = &mut (*induction_bucket.offset((*T.offset(p1 as isize) << 1) as isize));
        *fresh62 -= 1;
        *SA.offset(*fresh62 as isize) = p1;
        let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
        let fresh63 = &mut (*induction_bucket.offset((*T.offset(p2 as isize) << 1) as isize));
        *fresh63 -= 1;
        *SA.offset(*fresh63 as isize) = p2;
        let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
        let fresh64 = &mut (*induction_bucket.offset((*T.offset(p3 as isize) << 1) as isize));
        *fresh64 -= 1;
        *SA.offset(*fresh64 as isize) = p3;
        i -= 4;
    }
    j -= 2 * prefetch_distance + 3;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        let fresh65 = &mut (*induction_bucket.offset((*T.offset(p as isize) << 1) as isize));
        *fresh65 -= 1;
        *SA.offset(*fresh65 as isize) = p;
        i -= 1;
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_32s_6k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || m < 65536 {
        libsais_radix_sort_lms_suffixes_32s_6k(
            T,
            SA,
            induction_bucket,
            n as fast_sint_t - m as fast_sint_t + 1,
            m as fast_sint_t - 1,
        );
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_32s_2k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || m < 65536 {
        libsais_radix_sort_lms_suffixes_32s_2k(
            T,
            SA,
            induction_bucket,
            n as fast_sint_t - m as fast_sint_t + 1,
            m as fast_sint_t - 1,
        );
    }
}
unsafe fn libsais_radix_sort_lms_suffixes_32s_1k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: sa_sint_t = n - 2;
    let mut m: sa_sint_t = 0;
    let mut f0: fast_uint_t = 1 as fast_uint_t;
    let mut f1: fast_uint_t = 0 as fast_uint_t;
    let mut c0: fast_sint_t = *T.offset((n - 1) as isize) as fast_sint_t;
    let mut c1: fast_sint_t = 0 as fast_sint_t;
    let mut c2: fast_sint_t = 0 as fast_sint_t;
    while i as c_long >= prefetch_distance + 3 {
        libsais_prefetchr(T.offset((i as c_long - 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            buckets.offset(*T.offset((i as c_long - prefetch_distance) as isize) as isize),
        );
        libsais_prefetchw(
            buckets.offset(*T.offset((i as c_long - prefetch_distance - 1) as isize) as isize),
        );
        libsais_prefetchw(
            buckets.offset(*T.offset((i as c_long - prefetch_distance - 2) as isize) as isize),
        );
        libsais_prefetchw(
            buckets.offset(*T.offset((i as c_long - prefetch_distance - 3) as isize) as isize),
        );
        c1 = *T.offset(i as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        if f1 & !f0 != 0 {
            c2 = c0;
            let fresh66 = &mut (*buckets.offset(c2 as isize));
            *fresh66 -= 1;
            *SA.offset(*fresh66 as isize) = i + 1;
            m += 1;
        }
        c0 = *T.offset((i - 1) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        if f0 & !f1 != 0 {
            c2 = c1;
            let fresh67 = &mut (*buckets.offset(c2 as isize));
            *fresh67 -= 1;
            *SA.offset(*fresh67 as isize) = i;
            m += 1;
        }
        c1 = *T.offset((i - 2) as isize) as fast_sint_t;
        f1 = (c1 > c0 - f0 as fast_sint_t) as c_int as fast_uint_t;
        if f1 & !f0 != 0 {
            c2 = c0;
            let fresh68 = &mut (*buckets.offset(c2 as isize));
            *fresh68 -= 1;
            *SA.offset(*fresh68 as isize) = i - 1;
            m += 1;
        }
        c0 = *T.offset((i - 3) as isize) as fast_sint_t;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        if f0 & !f1 != 0 {
            c2 = c1;
            let fresh69 = &mut (*buckets.offset(c2 as isize));
            *fresh69 -= 1;
            *SA.offset(*fresh69 as isize) = i - 2;
            m += 1;
        }
        i -= 4;
    }
    while i >= 0 {
        c1 = c0;
        c0 = *T.offset(i as isize) as fast_sint_t;
        f1 = f0;
        f0 = (c0 > c1 - f1 as fast_sint_t) as c_int as fast_uint_t;
        if f0 & !f1 != 0 {
            c2 = c1;
            let fresh70 = &mut (*buckets.offset(c2 as isize));
            *fresh70 -= 1;
            *SA.offset(*fresh70 as isize) = i + 1;
            m += 1;
        }
        i -= 1;
    }
    if m > 1 {
        *SA.offset(*buckets.offset(c2 as isize) as isize) = 0;
    }
    m
}
unsafe fn libsais_radix_sort_set_markers_32s_6k(
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 3;
    while i < j {
        libsais_prefetchr(induction_bucket.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            SA.offset(*induction_bucket.offset((i + prefetch_distance) as isize) as isize),
        );
        libsais_prefetchw(
            SA.offset(*induction_bucket.offset((i + prefetch_distance + 1) as isize) as isize),
        );
        libsais_prefetchw(
            SA.offset(*induction_bucket.offset((i + prefetch_distance + 2) as isize) as isize),
        );
        libsais_prefetchw(
            SA.offset(*induction_bucket.offset((i + prefetch_distance + 3) as isize) as isize),
        );
        let fresh71 = &mut (*SA.offset(*induction_bucket.offset(i as isize) as isize));
        *fresh71 |= -(2147483647) - 1;
        let fresh72 = &mut (*SA.offset(*induction_bucket.offset((i + 1) as isize) as isize));
        *fresh72 |= -(2147483647) - 1;
        let fresh73 = &mut (*SA.offset(*induction_bucket.offset((i + 2) as isize) as isize));
        *fresh73 |= -(2147483647) - 1;
        let fresh74 = &mut (*SA.offset(*induction_bucket.offset((i + 3) as isize) as isize));
        *fresh74 |= -(2147483647) - 1;
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        let fresh75 = &mut (*SA.offset(*induction_bucket.offset(i as isize) as isize));
        *fresh75 |= -(2147483647) - 1;
        i += 1;
    }
}
unsafe fn libsais_radix_sort_set_markers_32s_4k(
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 3;
    while i < j {
        libsais_prefetchr(induction_bucket.offset(((i + 2 * prefetch_distance) << 1) as isize));
        libsais_prefetchw(
            SA.offset(*induction_bucket.offset(((i + prefetch_distance) << 1) as isize) as isize),
        );
        libsais_prefetchw(SA.offset(
            *induction_bucket.offset(((i + prefetch_distance + 1) << 1) as isize) as isize,
        ));
        libsais_prefetchw(SA.offset(
            *induction_bucket.offset(((i + prefetch_distance + 2) << 1) as isize) as isize,
        ));
        libsais_prefetchw(SA.offset(
            *induction_bucket.offset(((i + prefetch_distance + 3) << 1) as isize) as isize,
        ));
        let fresh76 = &mut (*SA.offset(*induction_bucket.offset((i << 1) as isize) as isize));
        *fresh76 |= (1) << (32 - 1 - 1);
        let fresh77 = &mut (*SA.offset(*induction_bucket.offset(((i + 1) << 1) as isize) as isize));
        *fresh77 |= (1) << (32 - 1 - 1);
        let fresh78 = &mut (*SA.offset(*induction_bucket.offset(((i + 2) << 1) as isize) as isize));
        *fresh78 |= (1) << (32 - 1 - 1);
        let fresh79 = &mut (*SA.offset(*induction_bucket.offset(((i + 3) << 1) as isize) as isize));
        *fresh79 |= (1) << (32 - 1 - 1);
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        let fresh80 = &mut (*SA.offset(*induction_bucket.offset((i << 1) as isize) as isize));
        *fresh80 |= (1) << (32 - 1 - 1);
        i += 1;
    }
}
unsafe fn libsais_radix_sort_set_markers_32s_6k_omp(
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_block_start: fast_sint_t = 0 as fast_sint_t;
    let mut omp_block_size: fast_sint_t = k as fast_sint_t - 1;
    libsais_radix_sort_set_markers_32s_6k(SA, induction_bucket, omp_block_start, omp_block_size);
}
unsafe fn libsais_radix_sort_set_markers_32s_4k_omp(
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_block_start: fast_sint_t = 0 as fast_sint_t;
    let mut omp_block_size: fast_sint_t = k as fast_sint_t - 1;
    libsais_radix_sort_set_markers_32s_4k(SA, induction_bucket, omp_block_start, omp_block_size);
}
unsafe fn libsais_initialize_buckets_for_partial_sorting_8u(
    mut T: *const uint8_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
) {
    let mut temp_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let fresh81 = &mut (*buckets.offset(
        ((*T.offset(first_lms_suffix as isize) as fast_uint_t) << 2).wrapping_add(1) as isize,
    ));
    *fresh81 += 1;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum0: sa_sint_t = left_suffixes_count + 1;
    let mut sum1: sa_sint_t = 0;
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i <= ((((1) << 8) - 1) << 2) as c_long {
        *temp_bucket.offset((j + 0 as c_long) as isize) = sum0;
        sum0 += *buckets.offset((i + 0 as c_long) as isize)
            + *buckets.offset((i + 2 as c_long) as isize);
        sum1 += *buckets.offset((i + 1 as c_long) as isize);
        *buckets.offset((j + 0 as c_long) as isize) = sum0;
        *buckets.offset((j + 1 as c_long) as isize) = sum1;
        i += ((1) << 2) as c_long;
        j += ((1) << 1) as c_long;
    }
}
unsafe fn libsais_initialize_buckets_for_partial_sorting_32s_6k(
    mut T: *const sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
) {
    let mut temp_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut sum0: sa_sint_t = left_suffixes_count + 1;
    let mut sum1: sa_sint_t = 0;
    let mut sum2: sa_sint_t = 0;
    first_lms_suffix = *T.offset(first_lms_suffix as isize);
    i = 0 as fast_sint_t;
    j = 0 as fast_sint_t;
    while i != ((first_lms_suffix as fast_sint_t) << 2) {
        let mut SS: sa_sint_t = *buckets.offset((i + 0 as c_long) as isize);
        let mut LS: sa_sint_t = *buckets.offset((i + 1 as c_long) as isize);
        let mut SL: sa_sint_t = *buckets.offset((i + 2 as c_long) as isize);
        let mut LL: sa_sint_t = *buckets.offset((i + 3 as c_long) as isize);
        *buckets.offset((i + 0 as c_long) as isize) = sum0;
        *buckets.offset((i + 1 as c_long) as isize) = sum2;
        *buckets.offset((i + 2 as c_long) as isize) = 0;
        *buckets.offset((i + 3 as c_long) as isize) = 0;
        sum0 += SS + SL;
        sum1 += LS;
        sum2 += LS + LL;
        *temp_bucket.offset((j + 0 as c_long) as isize) = sum0;
        *temp_bucket.offset((j + 1 as c_long) as isize) = sum1;
        i += ((1) << 2) as c_long;
        j += ((1) << 1) as c_long;
    }
    sum1 += 1;
    while i <= ((k as fast_sint_t - 1) << 2) {
        let mut SS_0: sa_sint_t = *buckets.offset((i + 0 as c_long) as isize);
        let mut LS_0: sa_sint_t = *buckets.offset((i + 1 as c_long) as isize);
        let mut SL_0: sa_sint_t = *buckets.offset((i + 2 as c_long) as isize);
        let mut LL_0: sa_sint_t = *buckets.offset((i + 3 as c_long) as isize);
        *buckets.offset((i + 0 as c_long) as isize) = sum0;
        *buckets.offset((i + 1 as c_long) as isize) = sum2;
        *buckets.offset((i + 2 as c_long) as isize) = 0;
        *buckets.offset((i + 3 as c_long) as isize) = 0;
        sum0 += SS_0 + SL_0;
        sum1 += LS_0;
        sum2 += LS_0 + LL_0;
        *temp_bucket.offset((j + 0 as c_long) as isize) = sum0;
        *temp_bucket.offset((j + 1 as c_long) as isize) = sum1;
        i += ((1) << 2) as c_long;
        j += ((1) << 1) as c_long;
    }
}
unsafe fn libsais_partial_sorting_scan_left_to_right_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut induction_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t =
        &mut *buckets.offset((2 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 1;
    while i < j {
        libsais_prefetchr(SA.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + prefetch_distance + 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + prefetch_distance + 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        d += (p0 < 0) as c_int;
        p0 &= 2147483647;
        let mut v0: sa_sint_t = ((*T.offset((p0 - 1) as isize) as c_int) << 1)
            + (*T.offset((p0 - 2) as isize) as c_int >= *T.offset((p0 - 1) as isize) as c_int)
                as c_int;
        let fresh82 = &mut (*induction_bucket.offset(v0 as isize));
        let fresh83 = *fresh82;
        *fresh82 += 1;
        *SA.offset(fresh83 as isize) = (p0 - 1)
            | (((*distinct_names.offset(v0 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v0 as isize) = d;
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        d += (p1 < 0) as c_int;
        p1 &= 2147483647;
        let mut v1: sa_sint_t = ((*T.offset((p1 - 1) as isize) as c_int) << 1)
            + (*T.offset((p1 - 2) as isize) as c_int >= *T.offset((p1 - 1) as isize) as c_int)
                as c_int;
        let fresh84 = &mut (*induction_bucket.offset(v1 as isize));
        let fresh85 = *fresh84;
        *fresh84 += 1;
        *SA.offset(fresh85 as isize) = (p1 - 1)
            | (((*distinct_names.offset(v1 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v1 as isize) = d;
        i += 2;
    }
    j += prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        d += (p < 0) as c_int;
        p &= 2147483647;
        let mut v: sa_sint_t = ((*T.offset((p - 1) as isize) as c_int) << 1)
            + (*T.offset((p - 2) as isize) as c_int >= *T.offset((p - 1) as isize) as c_int)
                as c_int;
        let fresh86 = &mut (*induction_bucket.offset(v as isize));
        let fresh87 = *fresh86;
        *fresh86 += 1;
        *SA.offset(fresh87 as isize) = (p - 1)
            | (((*distinct_names.offset(v as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v as isize) = d;
        i += 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut induction_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t =
        &mut *buckets.offset((2 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let fresh88 = &mut (*induction_bucket.offset(
        (((*T.offset((n - 1) as isize) as c_int) << 1)
            + (*T.offset((n - 2) as isize) as c_int >= *T.offset((n - 1) as isize) as c_int)
                as c_int) as isize,
    ));
    let fresh89 = *fresh88;
    *fresh88 += 1;
    *SA.offset(fresh89 as isize) = (n - 1) | (-(2147483647) - 1);
    d += 1;
    *distinct_names.offset(
        (((*T.offset((n - 1) as isize) as c_int) << 1)
            + (*T.offset((n - 2) as isize) as c_int >= *T.offset((n - 1) as isize) as c_int)
                as c_int) as isize,
    ) = d;
    if threads == 1 || left_suffixes_count < 65536 {
        d = libsais_partial_sorting_scan_left_to_right_8u(
            T,
            SA,
            buckets,
            d,
            0 as fast_sint_t,
            left_suffixes_count as fast_sint_t,
        );
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_6k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 2 * prefetch_distance - 1;
    while i < j {
        libsais_prefetchr(SA.offset((i + 3 * prefetch_distance) as isize));
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + 2 * prefetch_distance) as isize) & 2147483647) as isize)
                as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i + 2 * prefetch_distance) as isize) & 2147483647) as isize)
                as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset(
                (*SA.offset((i + 2 * prefetch_distance + 1) as isize) & 2147483647) as isize,
            ) as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset(
                (*SA.offset((i + 2 * prefetch_distance + 1) as isize) & 2147483647) as isize,
            ) as *const sa_sint_t)
                .offset(-1),
        );
        let mut p0: sa_sint_t = *SA.offset((i + prefetch_distance) as isize) & 2147483647;
        let mut v0: sa_sint_t = *T.offset((p0 - (p0 > 0) as c_int) as isize) << 2;
        libsais_prefetchw(buckets.offset(v0 as isize));
        let mut p1: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize) & 2147483647;
        let mut v1: sa_sint_t = *T.offset((p1 - (p1 > 0) as c_int) as isize) << 2;
        libsais_prefetchw(buckets.offset(v1 as isize));
        let mut p2: sa_sint_t = *SA.offset(i as isize);
        d += (p2 < 0) as c_int;
        p2 &= 2147483647;
        let mut v2: sa_sint_t = (*T.offset((p2 - 1) as isize) << 2)
            + (*T.offset((p2 - 2) as isize) >= *T.offset((p2 - 1) as isize)) as c_int;
        let fresh90 = &mut (*buckets.offset(v2 as isize));
        let fresh91 = *fresh90;
        *fresh90 += 1;
        *SA.offset(fresh91 as isize) = (p2 - 1)
            | (((*buckets.offset((2 + v2) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v2) as isize) = d;
        let mut p3: sa_sint_t = *SA.offset((i + 1) as isize);
        d += (p3 < 0) as c_int;
        p3 &= 2147483647;
        let mut v3: sa_sint_t = (*T.offset((p3 - 1) as isize) << 2)
            + (*T.offset((p3 - 2) as isize) >= *T.offset((p3 - 1) as isize)) as c_int;
        let fresh92 = &mut (*buckets.offset(v3 as isize));
        let fresh93 = *fresh92;
        *fresh92 += 1;
        *SA.offset(fresh93 as isize) = (p3 - 1)
            | (((*buckets.offset((2 + v3) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v3) as isize) = d;
        i += 2;
    }
    j += 2 * prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        d += (p < 0) as c_int;
        p &= 2147483647;
        let mut v: sa_sint_t = (*T.offset((p - 1) as isize) << 2)
            + (*T.offset((p - 2) as isize) >= *T.offset((p - 1) as isize)) as c_int;
        let fresh94 = &mut (*buckets.offset(v as isize));
        let fresh95 = *fresh94;
        *fresh94 += 1;
        *SA.offset(fresh95 as isize) = (p - 1)
            | (((*buckets.offset((2 + v) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v) as isize) = d;
        i += 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_4k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut induction_bucket: *mut sa_sint_t =
        &mut *buckets.offset((2 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t = &mut *buckets as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 2 * prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t = &*T.offset(
            (if s0 > 0 {
                s0 & !((1) << (32 - 1 - 1))
            } else {
                2
            }) as isize,
        ) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + 2 * prefetch_distance + 1) as isize);
        let mut Ts1: *const sa_sint_t = &*T.offset(
            (if s1 > 0 {
                s1 & !((1) << (32 - 1 - 1))
            } else {
                2
            }) as isize,
        ) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        if s2 > 0 {
            let Ts2: fast_sint_t =
                *T.offset(((s2 & !((1) << (32 - 1 - 1))) - 1) as isize) as fast_sint_t;
            libsais_prefetchw(induction_bucket.offset(Ts2 as isize));
            libsais_prefetchw(distinct_names.offset((Ts2 << 1) as isize));
        }
        let mut s3: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        if s3 > 0 {
            let Ts3: fast_sint_t =
                *T.offset(((s3 & !((1) << (32 - 1 - 1))) - 1) as isize) as fast_sint_t;
            libsais_prefetchw(induction_bucket.offset(Ts3 as isize));
            libsais_prefetchw(distinct_names.offset((Ts3 << 1) as isize));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            *SA.offset(i as isize) = 0;
            d += p0 >> (32 - 1 - 1);
            p0 &= !((1) << (32 - 1 - 1));
            let mut v0: sa_sint_t = (*T.offset((p0 - 1) as isize) << 1)
                + (*T.offset((p0 - 2) as isize) < *T.offset((p0 - 1) as isize)) as c_int;
            let fresh96 = &mut (*induction_bucket.offset(*T.offset((p0 - 1) as isize) as isize));
            let fresh97 = *fresh96;
            *fresh96 += 1;
            *SA.offset(fresh97 as isize) = (p0 - 1)
                | (((*T.offset((p0 - 2) as isize) < *T.offset((p0 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v0 as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v0 as isize) = d;
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            *SA.offset((i + 1) as isize) = 0;
            d += p1 >> (32 - 1 - 1);
            p1 &= !((1) << (32 - 1 - 1));
            let mut v1: sa_sint_t = (*T.offset((p1 - 1) as isize) << 1)
                + (*T.offset((p1 - 2) as isize) < *T.offset((p1 - 1) as isize)) as c_int;
            let fresh98 = &mut (*induction_bucket.offset(*T.offset((p1 - 1) as isize) as isize));
            let fresh99 = *fresh98;
            *fresh98 += 1;
            *SA.offset(fresh99 as isize) = (p1 - 1)
                | (((*T.offset((p1 - 2) as isize) < *T.offset((p1 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v1 as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v1 as isize) = d;
        }
        i += 2;
    }
    j += 2 * prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            *SA.offset(i as isize) = 0;
            d += p >> (32 - 1 - 1);
            p &= !((1) << (32 - 1 - 1));
            let mut v: sa_sint_t = (*T.offset((p - 1) as isize) << 1)
                + (*T.offset((p - 2) as isize) < *T.offset((p - 1) as isize)) as c_int;
            let fresh100 = &mut (*induction_bucket.offset(*T.offset((p - 1) as isize) as isize));
            let fresh101 = *fresh100;
            *fresh100 += 1;
            *SA.offset(fresh101 as isize) = (p - 1)
                | (((*T.offset((p - 2) as isize) < *T.offset((p - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v as isize) = d;
        }
        i += 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_1k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 2 * prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t =
            &*T.offset((if s0 > 0 { s0 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + 2 * prefetch_distance + 1) as isize);
        let mut Ts1: *const sa_sint_t =
            &*T.offset((if s1 > 0 { s1 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        if s2 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s2 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s2 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut s3: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        if s3 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s3 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s3 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            *SA.offset(i as isize) = 0;
            let fresh102 = &mut (*induction_bucket.offset(*T.offset((p0 - 1) as isize) as isize));
            let fresh103 = *fresh102;
            *fresh102 += 1;
            *SA.offset(fresh103 as isize) = (p0 - 1)
                | (((*T.offset((p0 - 2) as isize) < *T.offset((p0 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            *SA.offset((i + 1) as isize) = 0;
            let fresh104 = &mut (*induction_bucket.offset(*T.offset((p1 - 1) as isize) as isize));
            let fresh105 = *fresh104;
            *fresh104 += 1;
            *SA.offset(fresh105 as isize) = (p1 - 1)
                | (((*T.offset((p1 - 2) as isize) < *T.offset((p1 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 2;
    }
    j += 2 * prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            *SA.offset(i as isize) = 0;
            let fresh106 = &mut (*induction_bucket.offset(*T.offset((p - 1) as isize) as isize));
            let fresh107 = *fresh106;
            *fresh106 += 1;
            *SA.offset(fresh107 as isize) = (p - 1)
                | (((*T.offset((p - 2) as isize) < *T.offset((p - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 1;
    }
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_6k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let fresh108 = &mut (*buckets.offset(
        ((*T.offset((n - 1) as isize) << 2)
            + (*T.offset((n - 2) as isize) >= *T.offset((n - 1) as isize)) as c_int)
            as isize,
    ));
    let fresh109 = *fresh108;
    *fresh108 += 1;
    *SA.offset(fresh109 as isize) = (n - 1) | (-(2147483647) - 1);
    d += 1;
    *buckets.offset(
        (2 + ((*T.offset((n - 1) as isize) << 2)
            + (*T.offset((n - 2) as isize) >= *T.offset((n - 1) as isize)) as c_int))
            as isize,
    ) = d;
    if threads == 1 || left_suffixes_count < 65536 {
        d = libsais_partial_sorting_scan_left_to_right_32s_6k(
            T,
            SA,
            buckets,
            d,
            0 as fast_sint_t,
            left_suffixes_count as fast_sint_t,
        );
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_4k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut induction_bucket: *mut sa_sint_t =
        &mut *buckets.offset((2 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t = &mut *buckets as *mut sa_sint_t;
    let fresh110 = &mut (*induction_bucket.offset(*T.offset((n - 1) as isize) as isize));
    let fresh111 = *fresh110;
    *fresh110 += 1;
    *SA.offset(fresh111 as isize) = (n - 1)
        | (((*T.offset((n - 2) as isize) < *T.offset((n - 1) as isize)) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t
        | (1) << (32 - 1 - 1);
    d += 1;
    *distinct_names.offset(
        ((*T.offset((n - 1) as isize) << 1)
            + (*T.offset((n - 2) as isize) < *T.offset((n - 1) as isize)) as c_int)
            as isize,
    ) = d;
    if threads == 1 || n < 65536 {
        d = libsais_partial_sorting_scan_left_to_right_32s_4k(
            T,
            SA,
            k,
            buckets,
            d,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    d
}
unsafe fn libsais_partial_sorting_scan_left_to_right_32s_1k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let fresh112 = &mut (*buckets.offset(*T.offset((n - 1) as isize) as isize));
    let fresh113 = *fresh112;
    *fresh112 += 1;
    *SA.offset(fresh113 as isize) = (n - 1)
        | (((*T.offset((n - 2) as isize) < *T.offset((n - 1) as isize)) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t;
    if threads == 1 || n < 65536 {
        libsais_partial_sorting_scan_left_to_right_32s_1k(
            T,
            SA,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
}
unsafe fn libsais_partial_sorting_shift_markers_8u_omp(
    mut SA: *mut sa_sint_t,
    mut _n: sa_sint_t,
    mut buckets: *const sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut temp_bucket: *const sa_sint_t =
        &*buckets.offset((4 * ((1) << 8)) as isize) as *const sa_sint_t;
    let mut c: fast_sint_t = 0;
    c = ((((1) << 8) - 1) << 1) as fast_sint_t;
    while c >= ((1) << 1) as c_long {
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = 0;
        let mut s: sa_sint_t = -(2147483647) - 1;
        i = *temp_bucket.offset(c as isize) as fast_sint_t - 1;
        j = *buckets.offset((c - ((1) << 1) as c_long) as isize) as fast_sint_t + 3;
        while i >= j {
            libsais_prefetchw(SA.offset((i - prefetch_distance) as isize));
            let mut p0: sa_sint_t = *SA.offset(i as isize);
            let mut q0: sa_sint_t = p0 & (-(2147483647) - 1) ^ s;
            s ^= q0;
            *SA.offset(i as isize) = p0 ^ q0;
            let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
            let mut q1: sa_sint_t = p1 & (-(2147483647) - 1) ^ s;
            s ^= q1;
            *SA.offset((i - 1) as isize) = p1 ^ q1;
            let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
            let mut q2: sa_sint_t = p2 & (-(2147483647) - 1) ^ s;
            s ^= q2;
            *SA.offset((i - 2) as isize) = p2 ^ q2;
            let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
            let mut q3: sa_sint_t = p3 & (-(2147483647) - 1) ^ s;
            s ^= q3;
            *SA.offset((i - 3) as isize) = p3 ^ q3;
            i -= 4;
        }
        j -= 3;
        while i >= j {
            let mut p: sa_sint_t = *SA.offset(i as isize);
            let mut q: sa_sint_t = p & (-(2147483647) - 1) ^ s;
            s ^= q;
            *SA.offset(i as isize) = p ^ q;
            i -= 1;
        }
        c -= ((1) << 1) as c_long;
    }
}
unsafe fn libsais_partial_sorting_shift_markers_32s_6k_omp(
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *const sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut temp_bucket: *const sa_sint_t =
        &*buckets.offset((4 * k as fast_sint_t) as isize) as *const sa_sint_t;
    let mut c: fast_sint_t = 0;
    c = k as fast_sint_t - 1;
    while c >= 1 {
        let mut i: fast_sint_t = 0;
        let mut j: fast_sint_t = 0;
        let mut s: sa_sint_t = -(2147483647) - 1;
        i = *buckets.offset((c << 2) as isize) as fast_sint_t - 1;
        j = *temp_bucket.offset(((c - 1) << 1) as isize) as fast_sint_t + 3;
        while i >= j {
            libsais_prefetchw(SA.offset((i - prefetch_distance) as isize));
            let mut p0: sa_sint_t = *SA.offset(i as isize);
            let mut q0: sa_sint_t = p0 & (-(2147483647) - 1) ^ s;
            s ^= q0;
            *SA.offset(i as isize) = p0 ^ q0;
            let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
            let mut q1: sa_sint_t = p1 & (-(2147483647) - 1) ^ s;
            s ^= q1;
            *SA.offset((i - 1) as isize) = p1 ^ q1;
            let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
            let mut q2: sa_sint_t = p2 & (-(2147483647) - 1) ^ s;
            s ^= q2;
            *SA.offset((i - 2) as isize) = p2 ^ q2;
            let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
            let mut q3: sa_sint_t = p3 & (-(2147483647) - 1) ^ s;
            s ^= q3;
            *SA.offset((i - 3) as isize) = p3 ^ q3;
            i -= 4;
        }
        j -= 3;
        while i >= j {
            let mut p: sa_sint_t = *SA.offset(i as isize);
            let mut q: sa_sint_t = p & (-(2147483647) - 1) ^ s;
            s ^= q;
            *SA.offset(i as isize) = p ^ q;
            i -= 1;
        }
        c -= 1;
    }
}
unsafe fn libsais_partial_sorting_shift_markers_32s_4k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut s: sa_sint_t = (1) << (32 - 1 - 1);
    i = n as fast_sint_t - 1;
    while i >= 3 {
        libsais_prefetchw(SA.offset((i - prefetch_distance) as isize));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        let mut q0: sa_sint_t =
            (p0 & (1) << (32 - 1 - 1) ^ s) & ((p0 > 0) as c_int) << (32 - 1 - 1);
        s ^= q0;
        *SA.offset(i as isize) = p0 ^ q0;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        let mut q1: sa_sint_t =
            (p1 & (1) << (32 - 1 - 1) ^ s) & ((p1 > 0) as c_int) << (32 - 1 - 1);
        s ^= q1;
        *SA.offset((i - 1) as isize) = p1 ^ q1;
        let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
        let mut q2: sa_sint_t =
            (p2 & (1) << (32 - 1 - 1) ^ s) & ((p2 > 0) as c_int) << (32 - 1 - 1);
        s ^= q2;
        *SA.offset((i - 2) as isize) = p2 ^ q2;
        let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
        let mut q3: sa_sint_t =
            (p3 & (1) << (32 - 1 - 1) ^ s) & ((p3 > 0) as c_int) << (32 - 1 - 1);
        s ^= q3;
        *SA.offset((i - 3) as isize) = p3 ^ q3;
        i -= 4;
    }
    while i >= 0 {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        let mut q: sa_sint_t = (p & (1) << (32 - 1 - 1) ^ s) & ((p > 0) as c_int) << (32 - 1 - 1);
        s ^= q;
        *SA.offset(i as isize) = p ^ q;
        i -= 1;
    }
}
unsafe fn libsais_partial_sorting_shift_buckets_32s_6k(
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let mut temp_bucket: *mut sa_sint_t =
        &mut *buckets.offset((4 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    i = 0 as fast_sint_t;
    while i <= ((k as fast_sint_t - 1) << 1) {
        *buckets.offset((2 * i + 0 as c_long) as isize) =
            *temp_bucket.offset((i + 0 as c_long) as isize);
        *buckets.offset((2 * i + 1 as c_long) as isize) =
            *temp_bucket.offset((i + 1 as c_long) as isize);
        i += ((1) << 1) as c_long;
    }
}
unsafe fn libsais_partial_sorting_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut induction_bucket: *mut sa_sint_t = &mut *buckets as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t =
        &mut *buckets.offset((2 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 2 * prefetch_distance) as isize));
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance - 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance - 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        d += (p0 < 0) as c_int;
        p0 &= 2147483647;
        let mut v0: sa_sint_t = ((*T.offset((p0 - 1) as isize) as c_int) << 1)
            + (*T.offset((p0 - 2) as isize) as c_int > *T.offset((p0 - 1) as isize) as c_int)
                as c_int;
        let fresh114 = &mut (*induction_bucket.offset(v0 as isize));
        *fresh114 -= 1;
        *SA.offset(*fresh114 as isize) = (p0 - 1)
            | (((*distinct_names.offset(v0 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v0 as isize) = d;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        d += (p1 < 0) as c_int;
        p1 &= 2147483647;
        let mut v1: sa_sint_t = ((*T.offset((p1 - 1) as isize) as c_int) << 1)
            + (*T.offset((p1 - 2) as isize) as c_int > *T.offset((p1 - 1) as isize) as c_int)
                as c_int;
        let fresh115 = &mut (*induction_bucket.offset(v1 as isize));
        *fresh115 -= 1;
        *SA.offset(*fresh115 as isize) = (p1 - 1)
            | (((*distinct_names.offset(v1 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v1 as isize) = d;
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        d += (p < 0) as c_int;
        p &= 2147483647;
        let mut v: sa_sint_t = ((*T.offset((p - 1) as isize) as c_int) << 1)
            + (*T.offset((p - 2) as isize) as c_int > *T.offset((p - 1) as isize) as c_int)
                as c_int;
        let fresh116 = &mut (*induction_bucket.offset(v as isize));
        *fresh116 -= 1;
        *SA.offset(*fresh116 as isize) = (p - 1)
            | (((*distinct_names.offset(v as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *distinct_names.offset(v as isize) = d;
        i -= 1;
    }
    d
}
unsafe fn libsais_partial_gsa_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut induction_bucket: *mut sa_sint_t = &mut *buckets as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t =
        &mut *buckets.offset((2 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 2 * prefetch_distance) as isize));
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance - 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - prefetch_distance - 1) as isize) & 2147483647) as isize)
                as *const uint8_t)
                .offset(-1),
        );
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        d += (p0 < 0) as c_int;
        p0 &= 2147483647;
        let mut v0: sa_sint_t = ((*T.offset((p0 - 1) as isize) as c_int) << 1)
            + (*T.offset((p0 - 2) as isize) as c_int > *T.offset((p0 - 1) as isize) as c_int)
                as c_int;
        if v0 != 1 {
            let fresh117 = &mut (*induction_bucket.offset(v0 as isize));
            *fresh117 -= 1;
            *SA.offset(*fresh117 as isize) = (p0 - 1)
                | (((*distinct_names.offset(v0 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                    as sa_sint_t;
            *distinct_names.offset(v0 as isize) = d;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        d += (p1 < 0) as c_int;
        p1 &= 2147483647;
        let mut v1: sa_sint_t = ((*T.offset((p1 - 1) as isize) as c_int) << 1)
            + (*T.offset((p1 - 2) as isize) as c_int > *T.offset((p1 - 1) as isize) as c_int)
                as c_int;
        if v1 != 1 {
            let fresh118 = &mut (*induction_bucket.offset(v1 as isize));
            *fresh118 -= 1;
            *SA.offset(*fresh118 as isize) = (p1 - 1)
                | (((*distinct_names.offset(v1 as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                    as sa_sint_t;
            *distinct_names.offset(v1 as isize) = d;
        }
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        d += (p < 0) as c_int;
        p &= 2147483647;
        let mut v: sa_sint_t = ((*T.offset((p - 1) as isize) as c_int) << 1)
            + (*T.offset((p - 2) as isize) as c_int > *T.offset((p - 1) as isize) as c_int)
                as c_int;
        if v != 1 {
            let fresh119 = &mut (*induction_bucket.offset(v as isize));
            *fresh119 -= 1;
            *SA.offset(*fresh119 as isize) = (p - 1)
                | (((*distinct_names.offset(v as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                    as sa_sint_t;
            *distinct_names.offset(v as isize) = d;
        }
        i -= 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut scan_start: fast_sint_t = left_suffixes_count as fast_sint_t + 1;
    let mut scan_end: fast_sint_t = n as fast_sint_t - first_lms_suffix as fast_sint_t;
    if threads == 1 || scan_end - scan_start < 65536 {
        libsais_partial_sorting_scan_right_to_left_8u(
            T,
            SA,
            buckets,
            d,
            scan_start,
            scan_end - scan_start,
        );
    }
}
unsafe fn libsais_partial_gsa_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut scan_start: fast_sint_t = left_suffixes_count as fast_sint_t + 1;
    let mut scan_end: fast_sint_t = n as fast_sint_t - first_lms_suffix as fast_sint_t;
    if threads == 1 || scan_end - scan_start < 65536 {
        libsais_partial_gsa_scan_right_to_left_8u(
            T,
            SA,
            buckets,
            d,
            scan_start,
            scan_end - scan_start,
        );
    }
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_6k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 1;
    while i >= j {
        libsais_prefetchr(SA.offset((i - 3 * prefetch_distance) as isize));
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - 2 * prefetch_distance) as isize) & 2147483647) as isize)
                as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset((*SA.offset((i - 2 * prefetch_distance) as isize) & 2147483647) as isize)
                as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset(
                (*SA.offset((i - 2 * prefetch_distance - 1) as isize) & 2147483647) as isize,
            ) as *const sa_sint_t)
                .offset(-1),
        );
        libsais_prefetchr(
            (&*T.offset(
                (*SA.offset((i - 2 * prefetch_distance - 1) as isize) & 2147483647) as isize,
            ) as *const sa_sint_t)
                .offset(-1),
        );
        let mut p0: sa_sint_t = *SA.offset((i - prefetch_distance) as isize) & 2147483647;
        let mut v0: sa_sint_t = *T.offset((p0 - (p0 > 0) as c_int) as isize) << 2;
        libsais_prefetchw(buckets.offset(v0 as isize));
        let mut p1: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize) & 2147483647;
        let mut v1: sa_sint_t = *T.offset((p1 - (p1 > 0) as c_int) as isize) << 2;
        libsais_prefetchw(buckets.offset(v1 as isize));
        let mut p2: sa_sint_t = *SA.offset(i as isize);
        d += (p2 < 0) as c_int;
        p2 &= 2147483647;
        let mut v2: sa_sint_t = (*T.offset((p2 - 1) as isize) << 2)
            + (*T.offset((p2 - 2) as isize) > *T.offset((p2 - 1) as isize)) as c_int;
        let fresh120 = &mut (*buckets.offset(v2 as isize));
        *fresh120 -= 1;
        *SA.offset(*fresh120 as isize) = (p2 - 1)
            | (((*buckets.offset((2 + v2) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v2) as isize) = d;
        let mut p3: sa_sint_t = *SA.offset((i - 1) as isize);
        d += (p3 < 0) as c_int;
        p3 &= 2147483647;
        let mut v3: sa_sint_t = (*T.offset((p3 - 1) as isize) << 2)
            + (*T.offset((p3 - 2) as isize) > *T.offset((p3 - 1) as isize)) as c_int;
        let fresh121 = &mut (*buckets.offset(v3 as isize));
        *fresh121 -= 1;
        *SA.offset(*fresh121 as isize) = (p3 - 1)
            | (((*buckets.offset((2 + v3) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v3) as isize) = d;
        i -= 2;
    }
    j -= 2 * prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        d += (p < 0) as c_int;
        p &= 2147483647;
        let mut v: sa_sint_t = (*T.offset((p - 1) as isize) << 2)
            + (*T.offset((p - 2) as isize) > *T.offset((p - 1) as isize)) as c_int;
        let fresh122 = &mut (*buckets.offset(v as isize));
        *fresh122 -= 1;
        *SA.offset(*fresh122 as isize) = (p - 1)
            | (((*buckets.offset((2 + v) as isize) != d) as c_int as sa_uint_t) << (32 - 1))
                as sa_sint_t;
        *buckets.offset((2 + v) as isize) = d;
        i -= 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_4k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut induction_bucket: *mut sa_sint_t =
        &mut *buckets.offset((3 * k as fast_sint_t) as isize) as *mut sa_sint_t;
    let mut distinct_names: *mut sa_sint_t = &mut *buckets as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t = &*T.offset(
            (if s0 > 0 {
                s0 & !((1) << (32 - 1 - 1))
            } else {
                2
            }) as isize,
        ) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - 2 * prefetch_distance - 1) as isize);
        let mut Ts1: *const sa_sint_t = &*T.offset(
            (if s1 > 0 {
                s1 & !((1) << (32 - 1 - 1))
            } else {
                2
            }) as isize,
        ) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        if s2 > 0 {
            let Ts2: fast_sint_t =
                *T.offset(((s2 & !((1) << (32 - 1 - 1))) - 1) as isize) as fast_sint_t;
            libsais_prefetchw(induction_bucket.offset(Ts2 as isize));
            libsais_prefetchw(distinct_names.offset((Ts2 << 1) as isize));
        }
        let mut s3: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        if s3 > 0 {
            let Ts3: fast_sint_t =
                *T.offset(((s3 & !((1) << (32 - 1 - 1))) - 1) as isize) as fast_sint_t;
            libsais_prefetchw(induction_bucket.offset(Ts3 as isize));
            libsais_prefetchw(distinct_names.offset((Ts3 << 1) as isize));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        if p0 > 0 {
            *SA.offset(i as isize) = 0;
            d += p0 >> (32 - 1 - 1);
            p0 &= !((1) << (32 - 1 - 1));
            let mut v0: sa_sint_t = (*T.offset((p0 - 1) as isize) << 1)
                + (*T.offset((p0 - 2) as isize) > *T.offset((p0 - 1) as isize)) as c_int;
            let fresh123 = &mut (*induction_bucket.offset(*T.offset((p0 - 1) as isize) as isize));
            *fresh123 -= 1;
            *SA.offset(*fresh123 as isize) = (p0 - 1)
                | (((*T.offset((p0 - 2) as isize) > *T.offset((p0 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v0 as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v0 as isize) = d;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        if p1 > 0 {
            *SA.offset((i - 1) as isize) = 0;
            d += p1 >> (32 - 1 - 1);
            p1 &= !((1) << (32 - 1 - 1));
            let mut v1: sa_sint_t = (*T.offset((p1 - 1) as isize) << 1)
                + (*T.offset((p1 - 2) as isize) > *T.offset((p1 - 1) as isize)) as c_int;
            let fresh124 = &mut (*induction_bucket.offset(*T.offset((p1 - 1) as isize) as isize));
            *fresh124 -= 1;
            *SA.offset(*fresh124 as isize) = (p1 - 1)
                | (((*T.offset((p1 - 2) as isize) > *T.offset((p1 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v1 as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v1 as isize) = d;
        }
        i -= 2;
    }
    j -= 2 * prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        if p > 0 {
            *SA.offset(i as isize) = 0;
            d += p >> (32 - 1 - 1);
            p &= !((1) << (32 - 1 - 1));
            let mut v: sa_sint_t = (*T.offset((p - 1) as isize) << 1)
                + (*T.offset((p - 2) as isize) > *T.offset((p - 1) as isize)) as c_int;
            let fresh125 = &mut (*induction_bucket.offset(*T.offset((p - 1) as isize) as isize));
            *fresh125 -= 1;
            *SA.offset(*fresh125 as isize) = (p - 1)
                | (((*T.offset((p - 2) as isize) > *T.offset((p - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t
                | ((*distinct_names.offset(v as isize) != d) as c_int) << (32 - 1 - 1);
            *distinct_names.offset(v as isize) = d;
        }
        i -= 1;
    }
    d
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_1k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t =
            &*T.offset((if s0 > 0 { s0 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - 2 * prefetch_distance - 1) as isize);
        let mut Ts1: *const sa_sint_t =
            &*T.offset((if s1 > 0 { s1 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        if s2 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s2 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s2 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut s3: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        if s3 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s3 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s3 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        if p0 > 0 {
            *SA.offset(i as isize) = 0;
            let fresh126 = &mut (*induction_bucket.offset(*T.offset((p0 - 1) as isize) as isize));
            *fresh126 -= 1;
            *SA.offset(*fresh126 as isize) = (p0 - 1)
                | (((*T.offset((p0 - 2) as isize) > *T.offset((p0 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        if p1 > 0 {
            *SA.offset((i - 1) as isize) = 0;
            let fresh127 = &mut (*induction_bucket.offset(*T.offset((p1 - 1) as isize) as isize));
            *fresh127 -= 1;
            *SA.offset(*fresh127 as isize) = (p1 - 1)
                | (((*T.offset((p1 - 2) as isize) > *T.offset((p1 - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 2;
    }
    j -= 2 * prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        if p > 0 {
            *SA.offset(i as isize) = 0;
            let fresh128 = &mut (*induction_bucket.offset(*T.offset((p - 1) as isize) as isize));
            *fresh128 -= 1;
            *SA.offset(*fresh128 as isize) = (p - 1)
                | (((*T.offset((p - 2) as isize) > *T.offset((p - 1) as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 1;
    }
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_6k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut scan_start: fast_sint_t = left_suffixes_count as fast_sint_t + 1;
    let mut scan_end: fast_sint_t = n as fast_sint_t - first_lms_suffix as fast_sint_t;
    if threads == 1 || scan_end - scan_start < 65536 {
        d = libsais_partial_sorting_scan_right_to_left_32s_6k(
            T,
            SA,
            buckets,
            d,
            scan_start,
            scan_end - scan_start,
        );
    }
    d
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_4k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut d: sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    if threads == 1 || n < 65536 {
        d = libsais_partial_sorting_scan_right_to_left_32s_4k(
            T,
            SA,
            k,
            buckets,
            d,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    d
}
unsafe fn libsais_partial_sorting_scan_right_to_left_32s_1k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || n < 65536 {
        libsais_partial_sorting_scan_right_to_left_32s_1k(
            T,
            SA,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
}
unsafe fn libsais_partial_sorting_gather_lms_suffixes_32s_4k(
    mut SA: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> fast_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut l: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 3;
    l = omp_block_start;
    while i < j {
        libsais_prefetchr(SA.offset((i + prefetch_distance) as isize));
        let mut s0: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        *SA.offset(l as isize) = (s0.wrapping_sub(((1) << (32 - 1 - 1)) as sa_uint_t)
            & !((1) << (32 - 1 - 1)) as sa_uint_t) as sa_sint_t;
        l += ((s0 as sa_sint_t) < 0) as c_int as c_long;
        let mut s1: sa_uint_t = *SA.offset((i + 1) as isize) as sa_uint_t;
        *SA.offset(l as isize) = (s1.wrapping_sub(((1) << (32 - 1 - 1)) as sa_uint_t)
            & !((1) << (32 - 1 - 1)) as sa_uint_t) as sa_sint_t;
        l += ((s1 as sa_sint_t) < 0) as c_int as c_long;
        let mut s2: sa_uint_t = *SA.offset((i + 2) as isize) as sa_uint_t;
        *SA.offset(l as isize) = (s2.wrapping_sub(((1) << (32 - 1 - 1)) as sa_uint_t)
            & !((1) << (32 - 1 - 1)) as sa_uint_t) as sa_sint_t;
        l += ((s2 as sa_sint_t) < 0) as c_int as c_long;
        let mut s3: sa_uint_t = *SA.offset((i + 3) as isize) as sa_uint_t;
        *SA.offset(l as isize) = (s3.wrapping_sub(((1) << (32 - 1 - 1)) as sa_uint_t)
            & !((1) << (32 - 1 - 1)) as sa_uint_t) as sa_sint_t;
        l += ((s3 as sa_sint_t) < 0) as c_int as c_long;
        i += 4;
    }
    j += 3;
    while i < j {
        let mut s: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        *SA.offset(l as isize) = (s.wrapping_sub(((1) << (32 - 1 - 1)) as sa_uint_t)
            & !((1) << (32 - 1 - 1)) as sa_uint_t) as sa_sint_t;
        l += ((s as sa_sint_t) < 0) as c_int as c_long;
        i += 1;
    }
    l
}
unsafe fn libsais_partial_sorting_gather_lms_suffixes_32s_1k(
    mut SA: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> fast_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut l: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 3;
    l = omp_block_start;
    while i < j {
        libsais_prefetchr(SA.offset((i + prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(l as isize) = s0 & 2147483647;
        l += (s0 < 0) as c_int as c_long;
        let mut s1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset(l as isize) = s1 & 2147483647;
        l += (s1 < 0) as c_int as c_long;
        let mut s2: sa_sint_t = *SA.offset((i + 2) as isize);
        *SA.offset(l as isize) = s2 & 2147483647;
        l += (s2 < 0) as c_int as c_long;
        let mut s3: sa_sint_t = *SA.offset((i + 3) as isize);
        *SA.offset(l as isize) = s3 & 2147483647;
        l += (s3 < 0) as c_int as c_long;
        i += 4;
    }
    j += 3;
    while i < j {
        let mut s: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(l as isize) = s & 2147483647;
        l += (s < 0) as c_int as c_long;
        i += 1;
    }
    l
}
unsafe fn libsais_partial_sorting_gather_lms_suffixes_32s_4k_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (n as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        n as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_partial_sorting_gather_lms_suffixes_32s_4k(SA, omp_block_start, omp_block_size);
    }
}
unsafe fn libsais_partial_sorting_gather_lms_suffixes_32s_1k_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (n as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        n as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_partial_sorting_gather_lms_suffixes_32s_1k(SA, omp_block_start, omp_block_size);
    }
}
unsafe fn libsais_induce_partial_order_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut flags: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    memset(
        &mut *buckets.offset((2 * ((1) << 8)) as isize) as *mut sa_sint_t as *mut c_void,
        0,
        (2 as size_t)
            .wrapping_mul(((1) << 8) as c_ulong)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    if flags & 2 != 0 {
        *buckets.offset((4 * ((1) << 8) + 1) as isize) =
            *buckets.offset((4 * ((1) << 8) + (((1) << 1) + 1)) as isize) - 1;
        libsais_flip_suffix_markers_omp(
            SA,
            *buckets.offset((4 * ((1) << 8) + 1) as isize),
            threads,
        );
    }
    let mut d: sa_sint_t = libsais_partial_sorting_scan_left_to_right_8u_omp(
        T,
        SA,
        n,
        k,
        buckets,
        left_suffixes_count,
        0,
        threads,
        thread_state,
    );
    libsais_partial_sorting_shift_markers_8u_omp(SA, n, buckets, threads);
    if flags & 2 != 0 {
        libsais_partial_gsa_scan_right_to_left_8u_omp(
            T,
            SA,
            n,
            k,
            buckets,
            first_lms_suffix,
            left_suffixes_count,
            d,
            threads,
            thread_state,
        );
        if *T.offset(first_lms_suffix as isize) as c_int == 0 {
            memmove(
                &mut *SA.offset(1) as *mut sa_sint_t as *mut c_void,
                &mut *SA as *mut sa_sint_t as *const c_void,
                ((*buckets.offset((((1) << 1) + 1) as isize) - 1) as size_t)
                    .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
            *SA = first_lms_suffix | (-(2147483647) - 1);
        }
        *buckets.offset(1_isize) = 0;
    } else {
        libsais_partial_sorting_scan_right_to_left_8u_omp(
            T,
            SA,
            n,
            k,
            buckets,
            first_lms_suffix,
            left_suffixes_count,
            d,
            threads,
            thread_state,
        );
    };
}
unsafe fn libsais_induce_partial_order_32s_6k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut first_lms_suffix: sa_sint_t,
    mut left_suffixes_count: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut d: sa_sint_t = libsais_partial_sorting_scan_left_to_right_32s_6k_omp(
        T,
        SA,
        n,
        buckets,
        left_suffixes_count,
        0,
        threads,
        thread_state,
    );
    libsais_partial_sorting_shift_markers_32s_6k_omp(SA, k, buckets, threads);
    libsais_partial_sorting_shift_buckets_32s_6k(k, buckets);
    libsais_partial_sorting_scan_right_to_left_32s_6k_omp(
        T,
        SA,
        n,
        buckets,
        first_lms_suffix,
        left_suffixes_count,
        d,
        threads,
        thread_state,
    );
}
unsafe fn libsais_induce_partial_order_32s_4k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    memset(
        buckets as *mut c_void,
        0,
        (2 as c_ulong)
            .wrapping_mul(k as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut d: sa_sint_t = libsais_partial_sorting_scan_left_to_right_32s_4k_omp(
        T,
        SA,
        n,
        k,
        buckets,
        0,
        threads,
        thread_state,
    );
    libsais_partial_sorting_shift_markers_32s_4k(SA, n);
    libsais_partial_sorting_scan_right_to_left_32s_4k_omp(
        T,
        SA,
        n,
        k,
        buckets,
        d,
        threads,
        thread_state,
    );
    libsais_partial_sorting_gather_lms_suffixes_32s_4k_omp(SA, n, threads, thread_state);
}
unsafe fn libsais_induce_partial_order_32s_2k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_partial_sorting_scan_left_to_right_32s_1k_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
    libsais_partial_sorting_scan_right_to_left_32s_1k_omp(
        T,
        SA,
        n,
        &mut *buckets,
        threads,
        thread_state,
    );
    libsais_partial_sorting_gather_lms_suffixes_32s_1k_omp(SA, n, threads, thread_state);
}
unsafe fn libsais_induce_partial_order_32s_1k_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_count_suffixes_32s(T, n, k, buckets);
    libsais_initialize_buckets_start_32s_1k(k, buckets);
    libsais_partial_sorting_scan_left_to_right_32s_1k_omp(T, SA, n, buckets, threads, thread_state);
    libsais_count_suffixes_32s(T, n, k, buckets);
    libsais_initialize_buckets_end_32s_1k(k, buckets);
    libsais_partial_sorting_scan_right_to_left_32s_1k_omp(T, SA, n, buckets, threads, thread_state);
    libsais_partial_sorting_gather_lms_suffixes_32s_1k_omp(SA, n, threads, thread_state);
}
unsafe fn libsais_renumber_lms_suffixes_8u(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut name: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAm: *mut sa_sint_t = &mut *SA.offset(m as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 3;
    while i < j {
        libsais_prefetchr(SA.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            SAm.offset(((*SA.offset((i + prefetch_distance) as isize) & 2147483647) >> 1) as isize),
        );
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 1) as isize) & 2147483647) >> 1) as isize,
        ));
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 2) as isize) & 2147483647) >> 1) as isize,
        ));
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 3) as isize) & 2147483647) >> 1) as isize,
        ));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SAm.offset(((p0 & 2147483647) >> 1) as isize) = name | (-(2147483647) - 1);
        name += (p0 < 0) as c_int;
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SAm.offset(((p1 & 2147483647) >> 1) as isize) = name | (-(2147483647) - 1);
        name += (p1 < 0) as c_int;
        let mut p2: sa_sint_t = *SA.offset((i + 2) as isize);
        *SAm.offset(((p2 & 2147483647) >> 1) as isize) = name | (-(2147483647) - 1);
        name += (p2 < 0) as c_int;
        let mut p3: sa_sint_t = *SA.offset((i + 3) as isize);
        *SAm.offset(((p3 & 2147483647) >> 1) as isize) = name | (-(2147483647) - 1);
        name += (p3 < 0) as c_int;
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SAm.offset(((p & 2147483647) >> 1) as isize) = name | (-(2147483647) - 1);
        name += (p < 0) as c_int;
        i += 1;
    }
    name
}
unsafe fn libsais_gather_marked_lms_suffixes(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut l: fast_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> fast_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    l -= 1;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = m as fast_sint_t + omp_block_start + omp_block_size - 1;
    j = m as fast_sint_t + omp_block_start + 3;
    while i >= j {
        libsais_prefetchr(SA.offset((i - prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(l as isize) = s0 & 2147483647;
        l -= (s0 < 0) as c_int as c_long;
        let mut s1: sa_sint_t = *SA.offset((i - 1) as isize);
        *SA.offset(l as isize) = s1 & 2147483647;
        l -= (s1 < 0) as c_int as c_long;
        let mut s2: sa_sint_t = *SA.offset((i - 2) as isize);
        *SA.offset(l as isize) = s2 & 2147483647;
        l -= (s2 < 0) as c_int as c_long;
        let mut s3: sa_sint_t = *SA.offset((i - 3) as isize);
        *SA.offset(l as isize) = s3 & 2147483647;
        l -= (s3 < 0) as c_int as c_long;
        i -= 4;
    }
    j -= 3;
    while i >= j {
        let mut s: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(l as isize) = s & 2147483647;
        l -= (s < 0) as c_int as c_long;
        i -= 1;
    }
    l += 1;
    l
}
unsafe fn libsais_renumber_lms_suffixes_8u_omp(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut name: sa_sint_t = 0;
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (m as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        m as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        name = libsais_renumber_lms_suffixes_8u(SA, m, 0, omp_block_start, omp_block_size);
    }
    name
}
unsafe fn libsais_gather_marked_lms_suffixes_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t =
        ((n as fast_sint_t >> 1) / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        (n as fast_sint_t >> 1) - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_gather_marked_lms_suffixes(
            SA,
            m,
            n as fast_sint_t + fs as fast_sint_t,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_renumber_and_gather_lms_suffixes_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    memset(
        &mut *SA.offset(m as isize) as *mut sa_sint_t as *mut c_void,
        0,
        (n as size_t >> 1).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut name: sa_sint_t = libsais_renumber_lms_suffixes_8u_omp(SA, m, threads, thread_state);
    if name < m {
        libsais_gather_marked_lms_suffixes_omp(SA, n, m, fs, threads, thread_state);
    } else {
        let mut i: fast_sint_t = 0;
        i = 0 as fast_sint_t;
        while i < m as c_long {
            let fresh129 = &mut (*SA.offset(i as isize));
            *fresh129 &= 2147483647;
            i += 1;
        }
    }
    name
}
unsafe fn libsais_renumber_distinct_lms_suffixes_32s_4k(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut name: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAm: *mut sa_sint_t = &mut *SA.offset(m as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut p0: sa_sint_t = 0;
    let mut p1: sa_sint_t = 0;
    let mut p2: sa_sint_t = 0;
    let mut p3: sa_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 3;
    while i < j {
        libsais_prefetchw(SA.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            SAm.offset(((*SA.offset((i + prefetch_distance) as isize) & 2147483647) >> 1) as isize),
        );
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 1) as isize) & 2147483647) >> 1) as isize,
        ));
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 2) as isize) & 2147483647) >> 1) as isize,
        ));
        libsais_prefetchw(SAm.offset(
            ((*SA.offset((i + prefetch_distance + 3) as isize) & 2147483647) >> 1) as isize,
        ));
        p0 = *SA.offset(i as isize);
        let fresh130 = &mut (*SA.offset(i as isize));
        *fresh130 = p0 & 2147483647;
        *SAm.offset((*fresh130 >> 1) as isize) = name | p0 & p3 & (-(2147483647) - 1);
        name += (p0 < 0) as c_int;
        p1 = *SA.offset((i + 1) as isize);
        let fresh131 = &mut (*SA.offset((i + 1) as isize));
        *fresh131 = p1 & 2147483647;
        *SAm.offset((*fresh131 >> 1) as isize) = name | p1 & p0 & (-(2147483647) - 1);
        name += (p1 < 0) as c_int;
        p2 = *SA.offset((i + 2) as isize);
        let fresh132 = &mut (*SA.offset((i + 2) as isize));
        *fresh132 = p2 & 2147483647;
        *SAm.offset((*fresh132 >> 1) as isize) = name | p2 & p1 & (-(2147483647) - 1);
        name += (p2 < 0) as c_int;
        p3 = *SA.offset((i + 3) as isize);
        let fresh133 = &mut (*SA.offset((i + 3) as isize));
        *fresh133 = p3 & 2147483647;
        *SAm.offset((*fresh133 >> 1) as isize) = name | p3 & p2 & (-(2147483647) - 1);
        name += (p3 < 0) as c_int;
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        p2 = p3;
        p3 = *SA.offset(i as isize);
        let fresh134 = &mut (*SA.offset(i as isize));
        *fresh134 = p3 & 2147483647;
        *SAm.offset((*fresh134 >> 1) as isize) = name | p3 & p2 & (-(2147483647) - 1);
        name += (p3 < 0) as c_int;
        i += 1;
    }
    name
}
unsafe fn libsais_mark_distinct_lms_suffixes_32s(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut p0: sa_sint_t = 0;
    let mut p1: sa_sint_t = 0;
    let mut p2: sa_sint_t = 0;
    let mut p3: sa_sint_t = 0;
    i = m as fast_sint_t + omp_block_start;
    j = m as fast_sint_t + omp_block_start + omp_block_size - 3;
    while i < j {
        libsais_prefetchw(SA.offset((i + prefetch_distance) as isize));
        p0 = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & (p3 | 2147483647);
        p0 = if p0 == 0 { p3 } else { p0 };
        p1 = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 & (p0 | 2147483647);
        p1 = if p1 == 0 { p0 } else { p1 };
        p2 = *SA.offset((i + 2) as isize);
        *SA.offset((i + 2) as isize) = p2 & (p1 | 2147483647);
        p2 = if p2 == 0 { p1 } else { p2 };
        p3 = *SA.offset((i + 3) as isize);
        *SA.offset((i + 3) as isize) = p3 & (p2 | 2147483647);
        p3 = if p3 == 0 { p2 } else { p3 };
        i += 4;
    }
    j += 3;
    while i < j {
        p2 = p3;
        p3 = *SA.offset(i as isize);
        *SA.offset(i as isize) = p3 & (p2 | 2147483647);
        p3 = if p3 == 0 { p2 } else { p3 };
        i += 1;
    }
}
unsafe fn libsais_clamp_lms_suffixes_length_32s(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAm: *mut sa_sint_t = &mut *SA.offset(m as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 3;
    while i < j {
        libsais_prefetchw(SAm.offset((i + prefetch_distance) as isize));
        *SAm.offset(i as isize) = (if *SAm.offset(i as isize) < 0 {
            *SAm.offset(i as isize)
        } else {
            0
        }) & 2147483647;
        *SAm.offset((i + 1) as isize) = (if *SAm.offset((i + 1) as isize) < 0 {
            *SAm.offset((i + 1) as isize)
        } else {
            0
        }) & 2147483647;
        *SAm.offset((i + 2) as isize) = (if *SAm.offset((i + 2) as isize) < 0 {
            *SAm.offset((i + 2) as isize)
        } else {
            0
        }) & 2147483647;
        *SAm.offset((i + 3) as isize) = (if *SAm.offset((i + 3) as isize) < 0 {
            *SAm.offset((i + 3) as isize)
        } else {
            0
        }) & 2147483647;
        i += 4;
    }
    j += 3;
    while i < j {
        *SAm.offset(i as isize) = (if *SAm.offset(i as isize) < 0 {
            *SAm.offset(i as isize)
        } else {
            0
        }) & 2147483647;
        i += 1;
    }
}
unsafe fn libsais_renumber_distinct_lms_suffixes_32s_4k_omp(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut name: sa_sint_t = 0;
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (m as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        m as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        name = libsais_renumber_distinct_lms_suffixes_32s_4k(
            SA,
            m,
            1,
            omp_block_start,
            omp_block_size,
        );
    }
    name - 1
}
unsafe fn libsais_mark_distinct_lms_suffixes_32s_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_block_start: fast_sint_t = 0 as fast_sint_t;
    let mut omp_block_size: fast_sint_t = n as fast_sint_t >> 1;
    libsais_mark_distinct_lms_suffixes_32s(SA, m, omp_block_start, omp_block_size);
}
unsafe fn libsais_clamp_lms_suffixes_length_32s_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_block_start: fast_sint_t = 0 as fast_sint_t;
    let mut omp_block_size: fast_sint_t = n as fast_sint_t >> 1;
    libsais_clamp_lms_suffixes_length_32s(SA, m, omp_block_start, omp_block_size);
}
unsafe fn libsais_renumber_and_mark_distinct_lms_suffixes_32s_4k_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    memset(
        &mut *SA.offset(m as isize) as *mut sa_sint_t as *mut c_void,
        0,
        (n as size_t >> 1).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut name: sa_sint_t =
        libsais_renumber_distinct_lms_suffixes_32s_4k_omp(SA, m, threads, thread_state);
    if name < m {
        libsais_mark_distinct_lms_suffixes_32s_omp(SA, n, m, threads);
    }
    name
}
unsafe fn libsais_renumber_and_mark_distinct_lms_suffixes_32s_1k_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut threads: sa_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAm: *mut sa_sint_t = &mut *SA.offset(m as isize) as *mut sa_sint_t;
    libsais_gather_lms_suffixes_32s(T, SA, n);
    memset(
        &mut *SA.offset(m as isize) as *mut sa_sint_t as *mut c_void,
        0,
        (n as size_t)
            .wrapping_sub(m as size_t)
            .wrapping_sub(m as size_t)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = n as fast_sint_t - m as fast_sint_t;
    j = n as fast_sint_t - 1 - prefetch_distance - 3;
    while i < j {
        libsais_prefetchr(SA.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            SAm.offset((*SA.offset((i + prefetch_distance) as isize) as sa_uint_t >> 1) as isize),
        );
        libsais_prefetchw(
            SAm.offset(
                (*SA.offset((i + prefetch_distance + 1) as isize) as sa_uint_t >> 1) as isize,
            ),
        );
        libsais_prefetchw(
            SAm.offset(
                (*SA.offset((i + prefetch_distance + 2) as isize) as sa_uint_t >> 1) as isize,
            ),
        );
        libsais_prefetchw(
            SAm.offset(
                (*SA.offset((i + prefetch_distance + 3) as isize) as sa_uint_t >> 1) as isize,
            ),
        );
        *SAm.offset((*SA.offset(i as isize) as sa_uint_t >> 1) as isize) =
            *SA.offset((i + 1) as isize) - *SA.offset(i as isize) + 1 + (-(2147483647) - 1);
        *SAm.offset((*SA.offset((i + 1) as isize) as sa_uint_t >> 1) as isize) =
            *SA.offset((i + 2) as isize) - *SA.offset((i + 1) as isize) + 1 + (-(2147483647) - 1);
        *SAm.offset((*SA.offset((i + 2) as isize) as sa_uint_t >> 1) as isize) =
            *SA.offset((i + 3) as isize) - *SA.offset((i + 2) as isize) + 1 + (-(2147483647) - 1);
        *SAm.offset((*SA.offset((i + 3) as isize) as sa_uint_t >> 1) as isize) =
            *SA.offset((i + 4) as isize) - *SA.offset((i + 3) as isize) + 1 + (-(2147483647) - 1);
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        *SAm.offset((*SA.offset(i as isize) as sa_uint_t >> 1) as isize) =
            *SA.offset((i + 1) as isize) - *SA.offset(i as isize) + 1 + (-(2147483647) - 1);
        i += 1;
    }
    *SAm.offset((*SA.offset((n - 1) as isize) as sa_uint_t >> 1) as isize) =
        1 + (-(2147483647) - 1);
    libsais_clamp_lms_suffixes_length_32s_omp(SA, n, m, threads);
    let mut name: sa_sint_t = 1;
    let mut i_0: fast_sint_t = 0;
    let mut j_0: fast_sint_t = 0;
    let mut p: fast_sint_t = *SA as fast_sint_t;
    let mut plen: fast_sint_t = *SAm.offset((p >> 1) as isize) as fast_sint_t;
    let mut pdiff: sa_sint_t = -(2147483647) - 1;
    i_0 = 1 as fast_sint_t;
    j_0 = m as c_long - prefetch_distance - 1;
    while i_0 < j_0 {
        libsais_prefetchr(SA.offset((i_0 + 2 * prefetch_distance) as isize));
        libsais_prefetchw(
            SAm.offset((*SA.offset((i_0 + prefetch_distance) as isize) as sa_uint_t >> 1) as isize),
        );
        libsais_prefetchr(
            T.offset(*SA.offset((i_0 + prefetch_distance) as isize) as sa_uint_t as isize),
        );
        libsais_prefetchw(SAm.offset(
            (*SA.offset((i_0 + prefetch_distance + 1) as isize) as sa_uint_t >> 1) as isize,
        ));
        libsais_prefetchr(
            T.offset(*SA.offset((i_0 + prefetch_distance + 1) as isize) as sa_uint_t as isize),
        );
        let mut q: fast_sint_t = *SA.offset(i_0 as isize) as fast_sint_t;
        let mut qlen: fast_sint_t = *SAm.offset((q >> 1) as isize) as fast_sint_t;
        let mut qdiff: sa_sint_t = -(2147483647) - 1;
        if plen == qlen {
            let mut l: fast_sint_t = 0 as fast_sint_t;
            while *T.offset((p + l) as isize) == *T.offset((q + l) as isize) {
                l += 1;
                if l >= qlen {
                    break;
                }
            }
            qdiff = (l - qlen) as sa_sint_t & (-(2147483647) - 1);
        }
        *SAm.offset((p >> 1) as isize) = name | pdiff & qdiff;
        name += (qdiff < 0) as c_int;
        p = *SA.offset((i_0 + 1) as isize) as fast_sint_t;
        plen = *SAm.offset((p >> 1) as isize) as fast_sint_t;
        pdiff = -(2147483647) - 1;
        if qlen == plen {
            let mut l_0: fast_sint_t = 0 as fast_sint_t;
            while *T.offset((q + l_0) as isize) == *T.offset((p + l_0) as isize) {
                l_0 += 1;
                if l_0 >= plen {
                    break;
                }
            }
            pdiff = (l_0 - plen) as sa_sint_t & (-(2147483647) - 1);
        }
        *SAm.offset((q >> 1) as isize) = name | qdiff & pdiff;
        name += (pdiff < 0) as c_int;
        i_0 += 2;
    }
    j_0 += prefetch_distance + 1;
    while i_0 < j_0 {
        let mut q_0: fast_sint_t = *SA.offset(i_0 as isize) as fast_sint_t;
        let mut qlen_0: fast_sint_t = *SAm.offset((q_0 >> 1) as isize) as fast_sint_t;
        let mut qdiff_0: sa_sint_t = -(2147483647) - 1;
        if plen == qlen_0 {
            let mut l_1: fast_sint_t = 0 as fast_sint_t;
            while *T.offset((p + l_1) as isize) == *T.offset((q_0 + l_1) as isize) {
                l_1 += 1;
                if l_1 >= plen {
                    break;
                }
            }
            qdiff_0 = (l_1 - plen) as sa_sint_t & (-(2147483647) - 1);
        }
        *SAm.offset((p >> 1) as isize) = name | pdiff & qdiff_0;
        name += (qdiff_0 < 0) as c_int;
        p = q_0;
        plen = qlen_0;
        pdiff = qdiff_0;
        i_0 += 1;
    }
    *SAm.offset((p >> 1) as isize) = name | pdiff;
    name += 1;
    if name <= m {
        libsais_mark_distinct_lms_suffixes_32s_omp(SA, n, m, threads);
    }
    name - 1
}
unsafe fn libsais_reconstruct_lms_suffixes(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAnm: *const sa_sint_t = &mut *SA.offset((n - m) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 3;
    while i < j {
        libsais_prefetchw(SA.offset((i + 2 * prefetch_distance) as isize));
        libsais_prefetchr(SAnm.offset(*SA.offset((i + prefetch_distance) as isize) as isize));
        libsais_prefetchr(SAnm.offset(*SA.offset((i + prefetch_distance + 1) as isize) as isize));
        libsais_prefetchr(SAnm.offset(*SA.offset((i + prefetch_distance + 2) as isize) as isize));
        libsais_prefetchr(SAnm.offset(*SA.offset((i + prefetch_distance + 3) as isize) as isize));
        *SA.offset(i as isize) = *SAnm.offset(*SA.offset(i as isize) as isize);
        *SA.offset((i + 1) as isize) = *SAnm.offset(*SA.offset((i + 1) as isize) as isize);
        *SA.offset((i + 2) as isize) = *SAnm.offset(*SA.offset((i + 2) as isize) as isize);
        *SA.offset((i + 3) as isize) = *SAnm.offset(*SA.offset((i + 3) as isize) as isize);
        i += 4;
    }
    j += prefetch_distance + 3;
    while i < j {
        *SA.offset(i as isize) = *SAnm.offset(*SA.offset(i as isize) as isize);
        i += 1;
    }
}
unsafe fn libsais_reconstruct_lms_suffixes_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut omp_block_start: fast_sint_t = 0 as fast_sint_t;
    let mut omp_block_size: fast_sint_t = m as fast_sint_t;
    libsais_reconstruct_lms_suffixes(SA, n, m, omp_block_start, omp_block_size);
}
unsafe fn libsais_place_lms_suffixes_interval_8u(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut flags: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    if flags & 2 != 0 {
        let fresh135 = &mut (*buckets.offset((7 * ((1) << 8)) as isize));
        *fresh135 -= 1;
    }
    let mut bucket_end: *const sa_sint_t =
        &mut *buckets.offset((7 * ((1) << 8)) as isize) as *mut sa_sint_t;
    let mut c: fast_sint_t = 0;
    let mut j: fast_sint_t = n as fast_sint_t;
    c = (((1) << 8) - 2) as fast_sint_t;
    while c >= 0 {
        let mut l: fast_sint_t = *buckets.offset(((c << 1) + 1 + ((1) << 1) as c_long) as isize)
            as fast_sint_t
            - *buckets.offset(((c << 1) + 1) as isize) as fast_sint_t;
        if l > 0 {
            let mut i: fast_sint_t = *bucket_end.offset(c as isize) as fast_sint_t;
            if j - i > 0 {
                memset(
                    &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                    0,
                    ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            j = i - l;
            m -= l as sa_sint_t;
            memmove(
                &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        c -= 1;
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
    if flags & 2 != 0 {
        let fresh136 = &mut (*buckets.offset((7 * ((1) << 8)) as isize));
        *fresh136 += 1;
    }
}
unsafe fn libsais_place_lms_suffixes_interval_32s_4k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *const sa_sint_t,
) {
    let mut bucket_end: *const sa_sint_t =
        &*buckets.offset((3 * k as fast_sint_t) as isize) as *const sa_sint_t;
    let mut c: fast_sint_t = 0;
    let mut j: fast_sint_t = n as fast_sint_t;
    c = k as fast_sint_t - 2;
    while c >= 0 {
        let mut l: fast_sint_t = *buckets.offset(((c << 1) + 1 + ((1) << 1) as c_long) as isize)
            as fast_sint_t
            - *buckets.offset(((c << 1) + 1) as isize) as fast_sint_t;
        if l > 0 {
            let mut i: fast_sint_t = *bucket_end.offset(c as isize) as fast_sint_t;
            if j - i > 0 {
                memset(
                    &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                    0,
                    ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            j = i - l;
            m -= l as sa_sint_t;
            memmove(
                &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        c -= 1;
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_place_lms_suffixes_interval_32s_2k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *const sa_sint_t,
) {
    let mut j: fast_sint_t = n as fast_sint_t;
    if k > 1 {
        let mut c: fast_sint_t = 0;
        c = (k as fast_sint_t - 2) << 1;
        while c >= 0 as c_long {
            let mut l: fast_sint_t = *buckets.offset((c + (((1) << 1) + 1) as c_long) as isize)
                as fast_sint_t
                - *buckets.offset((c + 1 as c_long) as isize) as fast_sint_t;
            if l > 0 {
                let mut i: fast_sint_t = *buckets.offset(c as isize) as fast_sint_t;
                if j - i > 0 {
                    memset(
                        &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                        0,
                        ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                    );
                }
                j = i - l;
                m -= l as sa_sint_t;
                memmove(
                    &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                    &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                    (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            c -= ((1) << 1) as c_long;
        }
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_place_lms_suffixes_interval_32s_1k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *mut sa_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut c: sa_sint_t = k - 1;
    let mut i: fast_sint_t = 0;
    let mut l: fast_sint_t = *buckets.offset(c as isize) as fast_sint_t;
    i = m as fast_sint_t - 1;
    while i >= prefetch_distance + 3 {
        libsais_prefetchr(SA.offset((i - 2 * prefetch_distance) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 1) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 2) as isize) as isize));
        libsais_prefetchr(T.offset(*SA.offset((i - prefetch_distance - 3) as isize) as isize));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        if *T.offset(p0 as isize) != c {
            c = *T.offset(p0 as isize);
            memset(
                &mut *SA.offset(*buckets.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                ((l - *buckets.offset(c as isize) as c_long) as size_t).wrapping_mul(size_of::<
                    sa_sint_t,
                >(
                )
                    as c_ulong),
            );
            l = *buckets.offset(c as isize) as fast_sint_t;
        }
        l -= 1;
        *SA.offset(l as isize) = p0;
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        if *T.offset(p1 as isize) != c {
            c = *T.offset(p1 as isize);
            memset(
                &mut *SA.offset(*buckets.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                ((l - *buckets.offset(c as isize) as c_long) as size_t).wrapping_mul(size_of::<
                    sa_sint_t,
                >(
                )
                    as c_ulong),
            );
            l = *buckets.offset(c as isize) as fast_sint_t;
        }
        l -= 1;
        *SA.offset(l as isize) = p1;
        let mut p2: sa_sint_t = *SA.offset((i - 2) as isize);
        if *T.offset(p2 as isize) != c {
            c = *T.offset(p2 as isize);
            memset(
                &mut *SA.offset(*buckets.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                ((l - *buckets.offset(c as isize) as c_long) as size_t).wrapping_mul(size_of::<
                    sa_sint_t,
                >(
                )
                    as c_ulong),
            );
            l = *buckets.offset(c as isize) as fast_sint_t;
        }
        l -= 1;
        *SA.offset(l as isize) = p2;
        let mut p3: sa_sint_t = *SA.offset((i - 3) as isize);
        if *T.offset(p3 as isize) != c {
            c = *T.offset(p3 as isize);
            memset(
                &mut *SA.offset(*buckets.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                ((l - *buckets.offset(c as isize) as c_long) as size_t).wrapping_mul(size_of::<
                    sa_sint_t,
                >(
                )
                    as c_ulong),
            );
            l = *buckets.offset(c as isize) as fast_sint_t;
        }
        l -= 1;
        *SA.offset(l as isize) = p3;
        i -= 4;
    }
    while i >= 0 {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        if *T.offset(p as isize) != c {
            c = *T.offset(p as isize);
            memset(
                &mut *SA.offset(*buckets.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                ((l - *buckets.offset(c as isize) as c_long) as size_t).wrapping_mul(size_of::<
                    sa_sint_t,
                >(
                )
                    as c_ulong),
            );
            l = *buckets.offset(c as isize) as fast_sint_t;
        }
        l -= 1;
        *SA.offset(l as isize) = p;
        i -= 1;
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_place_lms_suffixes_histogram_32s_6k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *const sa_sint_t,
) {
    let mut bucket_end: *const sa_sint_t =
        &*buckets.offset((5 * k as fast_sint_t) as isize) as *const sa_sint_t;
    let mut c: fast_sint_t = 0;
    let mut j: fast_sint_t = n as fast_sint_t;
    c = k as fast_sint_t - 2;
    while c >= 0 {
        let mut l: fast_sint_t = *buckets.offset(((c << 2) + 1) as isize) as fast_sint_t;
        if l > 0 {
            let mut i: fast_sint_t = *bucket_end.offset(c as isize) as fast_sint_t;
            if j - i > 0 {
                memset(
                    &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                    0,
                    ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            j = i - l;
            m -= l as sa_sint_t;
            memmove(
                &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        c -= 1;
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_place_lms_suffixes_histogram_32s_4k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *const sa_sint_t,
) {
    let mut bucket_end: *const sa_sint_t =
        &*buckets.offset((3 * k as fast_sint_t) as isize) as *const sa_sint_t;
    let mut c: fast_sint_t = 0;
    let mut j: fast_sint_t = n as fast_sint_t;
    c = k as fast_sint_t - 2;
    while c >= 0 {
        let mut l: fast_sint_t = *buckets.offset(((c << 1) + 1) as isize) as fast_sint_t;
        if l > 0 {
            let mut i: fast_sint_t = *bucket_end.offset(c as isize) as fast_sint_t;
            if j - i > 0 {
                memset(
                    &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                    0,
                    ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            j = i - l;
            m -= l as sa_sint_t;
            memmove(
                &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        c -= 1;
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_place_lms_suffixes_histogram_32s_2k(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut buckets: *const sa_sint_t,
) {
    let mut j: fast_sint_t = n as fast_sint_t;
    if k > 1 {
        let mut c: fast_sint_t = 0;
        c = (k as fast_sint_t - 2) << 1;
        while c >= 0 as c_long {
            let mut l: fast_sint_t = *buckets.offset((c + 1 as c_long) as isize) as fast_sint_t;
            if l > 0 {
                let mut i: fast_sint_t = *buckets.offset(c as isize) as fast_sint_t;
                if j - i > 0 {
                    memset(
                        &mut *SA.offset(i as isize) as *mut sa_sint_t as *mut c_void,
                        0,
                        ((j - i) as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                    );
                }
                j = i - l;
                m -= l as sa_sint_t;
                memmove(
                    &mut *SA.offset(j as isize) as *mut sa_sint_t as *mut c_void,
                    &mut *SA.offset(m as isize) as *mut sa_sint_t as *const c_void,
                    (l as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            c -= ((1) << 1) as c_long;
        }
    }
    memset(
        &mut *SA as *mut sa_sint_t as *mut c_void,
        0,
        (j as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_final_bwt_scan_left_to_right_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            *SA.offset(i as isize) = *T.offset(p0 as isize) as c_int | (-(2147483647) - 1);
            let fresh137 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            let fresh138 = *fresh137;
            *fresh137 += 1;
            *SA.offset(fresh138 as isize) = p0
                | ((((*T.offset((p0 - (p0 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p0 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            *SA.offset((i + 1) as isize) = *T.offset(p1 as isize) as c_int | (-(2147483647) - 1);
            let fresh139 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            let fresh140 = *fresh139;
            *fresh139 += 1;
            *SA.offset(fresh140 as isize) = p1
                | ((((*T.offset((p1 - (p1 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p1 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 2;
    }
    j += prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            *SA.offset(i as isize) = *T.offset(p as isize) as c_int | (-(2147483647) - 1);
            let fresh141 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            let fresh142 = *fresh141;
            *fresh141 += 1;
            *SA.offset(fresh142 as isize) = p
                | ((((*T.offset((p - (p > 0) as c_int) as isize) as c_int)
                    < *T.offset(p as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 1;
    }
}
unsafe fn libsais_final_bwt_aux_scan_left_to_right_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut rm: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            *SA.offset(i as isize) = *T.offset(p0 as isize) as c_int | (-(2147483647) - 1);
            let fresh143 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            let fresh144 = *fresh143;
            *fresh143 += 1;
            *SA.offset(fresh144 as isize) = p0
                | ((((*T.offset((p0 - (p0 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p0 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
            if p0 & rm == 0 {
                *I.offset((p0 / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p0 as isize) as isize);
            }
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            *SA.offset((i + 1) as isize) = *T.offset(p1 as isize) as c_int | (-(2147483647) - 1);
            let fresh145 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            let fresh146 = *fresh145;
            *fresh145 += 1;
            *SA.offset(fresh146 as isize) = p1
                | ((((*T.offset((p1 - (p1 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p1 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
            if p1 & rm == 0 {
                *I.offset((p1 / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p1 as isize) as isize);
            }
        }
        i += 2;
    }
    j += prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            *SA.offset(i as isize) = *T.offset(p as isize) as c_int | (-(2147483647) - 1);
            let fresh147 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            let fresh148 = *fresh147;
            *fresh147 += 1;
            *SA.offset(fresh148 as isize) = p
                | ((((*T.offset((p - (p > 0) as c_int) as isize) as c_int)
                    < *T.offset(p as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
            if p & rm == 0 {
                *I.offset((p / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p as isize) as isize);
            }
        }
        i += 1;
    }
}
unsafe fn libsais_final_sorting_scan_left_to_right_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 ^ (-(2147483647) - 1);
        if p0 > 0 {
            p0 -= 1;
            let fresh149 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            let fresh150 = *fresh149;
            *fresh149 += 1;
            *SA.offset(fresh150 as isize) = p0
                | ((((*T.offset((p0 - (p0 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p0 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 ^ (-(2147483647) - 1);
        if p1 > 0 {
            p1 -= 1;
            let fresh151 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            let fresh152 = *fresh151;
            *fresh151 += 1;
            *SA.offset(fresh152 as isize) = p1
                | ((((*T.offset((p1 - (p1 > 0) as c_int) as isize) as c_int)
                    < *T.offset(p1 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 2;
    }
    j += prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p ^ (-(2147483647) - 1);
        if p > 0 {
            p -= 1;
            let fresh153 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            let fresh154 = *fresh153;
            *fresh153 += 1;
            *SA.offset(fresh154 as isize) = p
                | ((((*T.offset((p - (p > 0) as c_int) as isize) as c_int)
                    < *T.offset(p as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 1;
    }
}
unsafe fn libsais_final_sorting_scan_left_to_right_32s(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 2 * prefetch_distance - 1;
    while i < j {
        libsais_prefetchw(SA.offset((i + 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i + 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t =
            &*T.offset((if s0 > 0 { s0 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i + 2 * prefetch_distance + 1) as isize);
        let mut Ts1: *const sa_sint_t =
            &*T.offset((if s1 > 0 { s1 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i + prefetch_distance) as isize);
        if s2 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s2 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s2 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut s3: sa_sint_t = *SA.offset((i + prefetch_distance + 1) as isize);
        if s3 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s3 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s3 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 ^ (-(2147483647) - 1);
        if p0 > 0 {
            p0 -= 1;
            let fresh155 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            let fresh156 = *fresh155;
            *fresh155 += 1;
            *SA.offset(fresh156 as isize) = p0
                | (((*T.offset((p0 - (p0 > 0) as c_int) as isize) < *T.offset(p0 as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i + 1) as isize);
        *SA.offset((i + 1) as isize) = p1 ^ (-(2147483647) - 1);
        if p1 > 0 {
            p1 -= 1;
            let fresh157 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            let fresh158 = *fresh157;
            *fresh157 += 1;
            *SA.offset(fresh158 as isize) = p1
                | (((*T.offset((p1 - (p1 > 0) as c_int) as isize) < *T.offset(p1 as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 2;
    }
    j += 2 * prefetch_distance + 1;
    while i < j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p ^ (-(2147483647) - 1);
        if p > 0 {
            p -= 1;
            let fresh159 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            let fresh160 = *fresh159;
            *fresh159 += 1;
            *SA.offset(fresh160 as isize) = p
                | (((*T.offset((p - (p > 0) as c_int) as isize) < *T.offset(p as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i += 1;
    }
}
unsafe fn libsais_final_bwt_scan_left_to_right_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: fast_sint_t,
    mut _k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let fresh161 =
        &mut (*induction_bucket.offset(*T.offset((n as sa_sint_t - 1) as isize) as isize));
    let fresh162 = *fresh161;
    *fresh161 += 1;
    *SA.offset(fresh162 as isize) = (n as sa_sint_t - 1)
        | ((((*T.offset((n as sa_sint_t - 2) as isize) as c_int)
            < *T.offset((n as sa_sint_t - 1) as isize) as c_int) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t;
    if threads == 1 || n < 65536 {
        libsais_final_bwt_scan_left_to_right_8u(T, SA, induction_bucket, 0 as fast_sint_t, n);
    }
}
unsafe fn libsais_final_bwt_aux_scan_left_to_right_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: fast_sint_t,
    mut _k: sa_sint_t,
    mut rm: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let fresh163 =
        &mut (*induction_bucket.offset(*T.offset((n as sa_sint_t - 1) as isize) as isize));
    let fresh164 = *fresh163;
    *fresh163 += 1;
    *SA.offset(fresh164 as isize) = (n as sa_sint_t - 1)
        | ((((*T.offset((n as sa_sint_t - 2) as isize) as c_int)
            < *T.offset((n as sa_sint_t - 1) as isize) as c_int) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t;
    if (n as sa_sint_t - 1) & rm == 0 {
        *I.offset(((n as sa_sint_t - 1) / (rm + 1)) as isize) =
            *induction_bucket.offset(*T.offset((n as sa_sint_t - 1) as isize) as isize);
    }
    if threads == 1 || n < 65536 {
        libsais_final_bwt_aux_scan_left_to_right_8u(
            T,
            SA,
            rm,
            I,
            induction_bucket,
            0 as fast_sint_t,
            n,
        );
    }
}
unsafe fn libsais_final_sorting_scan_left_to_right_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: fast_sint_t,
    mut _k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let fresh165 =
        &mut (*induction_bucket.offset(*T.offset((n as sa_sint_t - 1) as isize) as isize));
    let fresh166 = *fresh165;
    *fresh165 += 1;
    *SA.offset(fresh166 as isize) = (n as sa_sint_t - 1)
        | ((((*T.offset((n as sa_sint_t - 2) as isize) as c_int)
            < *T.offset((n as sa_sint_t - 1) as isize) as c_int) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t;
    if threads == 1 || n < 65536 {
        libsais_final_sorting_scan_left_to_right_8u(T, SA, induction_bucket, 0 as fast_sint_t, n);
    }
}
unsafe fn libsais_final_sorting_scan_left_to_right_32s_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let fresh167 = &mut (*induction_bucket.offset(*T.offset((n - 1) as isize) as isize));
    let fresh168 = *fresh167;
    *fresh167 += 1;
    *SA.offset(fresh168 as isize) = (n - 1)
        | (((*T.offset((n - 2) as isize) < *T.offset((n - 1) as isize)) as c_int as sa_uint_t)
            << (32 - 1)) as sa_sint_t;
    if threads == 1 || n < 65536 {
        libsais_final_sorting_scan_left_to_right_32s(
            T,
            SA,
            induction_bucket,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
}
unsafe fn libsais_final_bwt_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut index: sa_sint_t = -(1);
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        index = if p0 == 0 { i as sa_sint_t } else { index };
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            let mut c0: uint8_t = *T.offset((p0 - (p0 > 0) as c_int) as isize);
            let mut c1: uint8_t = *T.offset(p0 as isize);
            *SA.offset(i as isize) = c1 as sa_sint_t;
            let mut t: sa_sint_t = c0 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh169 = &mut (*induction_bucket.offset(c1 as isize));
            *fresh169 -= 1;
            *SA.offset(*fresh169 as isize) = if c0 <= c1 { p0 } else { t };
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        index = if p1 == 0 { (i - 1) as sa_sint_t } else { index };
        *SA.offset((i - 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            let mut c0_0: uint8_t = *T.offset((p1 - (p1 > 0) as c_int) as isize);
            let mut c1_0: uint8_t = *T.offset(p1 as isize);
            *SA.offset((i - 1) as isize) = c1_0 as sa_sint_t;
            let mut t_0: sa_sint_t = c0_0 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh170 = &mut (*induction_bucket.offset(c1_0 as isize));
            *fresh170 -= 1;
            *SA.offset(*fresh170 as isize) = if c0_0 <= c1_0 { p1 } else { t_0 };
        }
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        index = if p == 0 { i as sa_sint_t } else { index };
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            let mut c0_1: uint8_t = *T.offset((p - (p > 0) as c_int) as isize);
            let mut c1_1: uint8_t = *T.offset(p as isize);
            *SA.offset(i as isize) = c1_1 as sa_sint_t;
            let mut t_1: sa_sint_t = c0_1 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh171 = &mut (*induction_bucket.offset(c1_1 as isize));
            *fresh171 -= 1;
            *SA.offset(*fresh171 as isize) = if c0_1 <= c1_1 { p } else { t_1 };
        }
        i -= 1;
    }
    index
}
unsafe fn libsais_final_bwt_aux_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut rm: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            let mut c0: uint8_t = *T.offset((p0 - (p0 > 0) as c_int) as isize);
            let mut c1: uint8_t = *T.offset(p0 as isize);
            *SA.offset(i as isize) = c1 as sa_sint_t;
            let mut t: sa_sint_t = c0 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh172 = &mut (*induction_bucket.offset(c1 as isize));
            *fresh172 -= 1;
            *SA.offset(*fresh172 as isize) = if c0 <= c1 { p0 } else { t };
            if p0 & rm == 0 {
                *I.offset((p0 / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p0 as isize) as isize) + 1;
            }
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        *SA.offset((i - 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            let mut c0_0: uint8_t = *T.offset((p1 - (p1 > 0) as c_int) as isize);
            let mut c1_0: uint8_t = *T.offset(p1 as isize);
            *SA.offset((i - 1) as isize) = c1_0 as sa_sint_t;
            let mut t_0: sa_sint_t = c0_0 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh173 = &mut (*induction_bucket.offset(c1_0 as isize));
            *fresh173 -= 1;
            *SA.offset(*fresh173 as isize) = if c0_0 <= c1_0 { p1 } else { t_0 };
            if p1 & rm == 0 {
                *I.offset((p1 / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p1 as isize) as isize) + 1;
            }
        }
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            let mut c0_1: uint8_t = *T.offset((p - (p > 0) as c_int) as isize);
            let mut c1_1: uint8_t = *T.offset(p as isize);
            *SA.offset(i as isize) = c1_1 as sa_sint_t;
            let mut t_1: sa_sint_t = c0_1 as c_int | (-(2147483647 as c_int) - 1 as c_int);
            let fresh174 = &mut (*induction_bucket.offset(c1_1 as isize));
            *fresh174 -= 1;
            *SA.offset(*fresh174 as isize) = if c0_1 <= c1_1 { p } else { t_1 };
            if p & rm == 0 {
                *I.offset((p / (rm + 1)) as isize) =
                    *induction_bucket.offset(*T.offset(p as isize) as isize) + 1;
            }
        }
        i -= 1;
    }
}
unsafe fn libsais_final_sorting_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            let fresh175 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            *fresh175 -= 1;
            *SA.offset(*fresh175 as isize) = p0
                | (((*T.offset((p0 - (p0 > 0) as c_int) as isize) as c_int
                    > *T.offset(p0 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        *SA.offset((i - 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            let fresh176 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            *fresh176 -= 1;
            *SA.offset(*fresh176 as isize) = p1
                | (((*T.offset((p1 - (p1 > 0) as c_int) as isize) as c_int
                    > *T.offset(p1 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            let fresh177 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            *fresh177 -= 1;
            *SA.offset(*fresh177 as isize) = p
                | (((*T.offset((p - (p > 0) as c_int) as isize) as c_int
                    > *T.offset(p as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 1;
    }
}
unsafe fn libsais_final_gsa_scan_right_to_left_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 2 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        let mut Ts0: *const uint8_t =
            &*T.offset((if s0 > 0 { s0 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts0.offset(-1));
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        let mut Ts1: *const uint8_t =
            &*T.offset((if s1 > 0 { s1 } else { 2 }) as isize) as *const uint8_t;
        libsais_prefetchr(Ts1.offset(-1));
        libsais_prefetchr(Ts1.offset(-1));
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 && *T.offset((p0 - 1) as isize) as c_int > 0 {
            p0 -= 1;
            let fresh178 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            *fresh178 -= 1;
            *SA.offset(*fresh178 as isize) = p0
                | (((*T.offset((p0 - (p0 > 0) as c_int) as isize) as c_int
                    > *T.offset(p0 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        *SA.offset((i - 1) as isize) = p1 & 2147483647;
        if p1 > 0 && *T.offset((p1 - 1) as isize) as c_int > 0 {
            p1 -= 1;
            let fresh179 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            *fresh179 -= 1;
            *SA.offset(*fresh179 as isize) = p1
                | (((*T.offset((p1 - (p1 > 0) as c_int) as isize) as c_int
                    > *T.offset(p1 as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 2;
    }
    j -= prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 && *T.offset((p - 1) as isize) as c_int > 0 {
            p -= 1;
            let fresh180 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            *fresh180 -= 1;
            *SA.offset(*fresh180 as isize) = p
                | (((*T.offset((p - (p > 0) as c_int) as isize) as c_int
                    > *T.offset(p as isize) as c_int) as c_int as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 1;
    }
}
unsafe fn libsais_final_sorting_scan_right_to_left_32s(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    i = omp_block_start + omp_block_size - 1;
    j = omp_block_start + 2 * prefetch_distance + 1;
    while i >= j {
        libsais_prefetchw(SA.offset((i - 3 * prefetch_distance) as isize));
        let mut s0: sa_sint_t = *SA.offset((i - 2 * prefetch_distance) as isize);
        let mut Ts0: *const sa_sint_t =
            &*T.offset((if s0 > 0 { s0 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts0.offset(-1));
        let mut s1: sa_sint_t = *SA.offset((i - 2 * prefetch_distance - 1) as isize);
        let mut Ts1: *const sa_sint_t =
            &*T.offset((if s1 > 0 { s1 } else { 1 }) as isize) as *const sa_sint_t;
        libsais_prefetchr(Ts1.offset(-1));
        let mut s2: sa_sint_t = *SA.offset((i - prefetch_distance) as isize);
        if s2 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s2 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s2 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut s3: sa_sint_t = *SA.offset((i - prefetch_distance - 1) as isize);
        if s3 > 0 {
            libsais_prefetchw(induction_bucket.offset(*T.offset((s3 - 1) as isize) as isize));
            libsais_prefetchr((&*T.offset(s3 as isize) as *const sa_sint_t).offset(-1));
        }
        let mut p0: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p0 & 2147483647;
        if p0 > 0 {
            p0 -= 1;
            let fresh181 = &mut (*induction_bucket.offset(*T.offset(p0 as isize) as isize));
            *fresh181 -= 1;
            *SA.offset(*fresh181 as isize) = p0
                | (((*T.offset((p0 - (p0 > 0) as c_int) as isize) > *T.offset(p0 as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        let mut p1: sa_sint_t = *SA.offset((i - 1) as isize);
        *SA.offset((i - 1) as isize) = p1 & 2147483647;
        if p1 > 0 {
            p1 -= 1;
            let fresh182 = &mut (*induction_bucket.offset(*T.offset(p1 as isize) as isize));
            *fresh182 -= 1;
            *SA.offset(*fresh182 as isize) = p1
                | (((*T.offset((p1 - (p1 > 0) as c_int) as isize) > *T.offset(p1 as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 2;
    }
    j -= 2 * prefetch_distance + 1;
    while i >= j {
        let mut p: sa_sint_t = *SA.offset(i as isize);
        *SA.offset(i as isize) = p & 2147483647;
        if p > 0 {
            p -= 1;
            let fresh183 = &mut (*induction_bucket.offset(*T.offset(p as isize) as isize));
            *fresh183 -= 1;
            *SA.offset(*fresh183 as isize) = p
                | (((*T.offset((p - (p > 0) as c_int) as isize) > *T.offset(p as isize)) as c_int
                    as sa_uint_t)
                    << (32 - 1)) as sa_sint_t;
        }
        i -= 1;
    }
}
unsafe fn libsais_final_bwt_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut index: sa_sint_t = -(1);
    if threads == 1 || n < 65536 {
        index = libsais_final_bwt_scan_right_to_left_8u(
            T,
            SA,
            induction_bucket,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
    index
}
unsafe fn libsais_final_bwt_aux_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut _k: sa_sint_t,
    mut rm: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || n < 65536 {
        libsais_final_bwt_aux_scan_right_to_left_8u(
            T,
            SA,
            rm,
            I,
            induction_bucket,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
}
unsafe fn libsais_final_sorting_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
    mut _k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || omp_block_size < 65536 {
        libsais_final_sorting_scan_right_to_left_8u(
            T,
            SA,
            induction_bucket,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_final_gsa_scan_right_to_left_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
    mut _k: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || omp_block_size < 65536 {
        libsais_final_gsa_scan_right_to_left_8u(
            T,
            SA,
            induction_bucket,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_final_sorting_scan_right_to_left_32s_omp(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut induction_bucket: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if threads == 1 || n < 65536 {
        libsais_final_sorting_scan_right_to_left_32s(
            T,
            SA,
            induction_bucket,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
    }
}
unsafe fn libsais_clear_lms_suffixes_omp(
    mut SA: *mut sa_sint_t,
    mut _n: sa_sint_t,
    mut k: sa_sint_t,
    mut bucket_start: *mut sa_sint_t,
    mut bucket_end: *mut sa_sint_t,
    mut _threads: sa_sint_t,
) {
    let mut c: fast_sint_t = 0;
    c = 0 as fast_sint_t;
    while c < k as c_long {
        if *bucket_end.offset(c as isize) > *bucket_start.offset(c as isize) {
            memset(
                &mut *SA.offset(*bucket_start.offset(c as isize) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                (*bucket_end.offset(c as isize) as size_t)
                    .wrapping_sub(*bucket_start.offset(c as isize) as size_t)
                    .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        c += 1;
    }
}
unsafe fn libsais_induce_final_order_8u_omp(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut flags: sa_sint_t,
    mut r: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    if flags & 1 == 0 {
        if flags & 2 != 0 {
            *buckets.offset((6 * ((1) << 8)) as isize) =
                *buckets.offset((7 * ((1) << 8)) as isize) - 1;
        }
        libsais_final_sorting_scan_left_to_right_8u_omp(
            T,
            SA,
            n as fast_sint_t,
            k,
            &mut *buckets.offset((6 * ((1) << 8)) as isize),
            threads,
            thread_state,
        );
        if threads > 1 && n >= 65536 {
            libsais_clear_lms_suffixes_omp(
                SA,
                n,
                (1) << 8,
                &mut *buckets.offset((6 * ((1) << 8)) as isize),
                &mut *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
            );
        }
        if flags & 2 != 0 {
            libsais_flip_suffix_markers_omp(
                SA,
                *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
            );
            libsais_final_gsa_scan_right_to_left_8u_omp(
                T,
                SA,
                *buckets.offset((7 * ((1) << 8)) as isize) as fast_sint_t,
                n as fast_sint_t - *buckets.offset((7 * ((1) << 8)) as isize) as c_long,
                k,
                &mut *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
                thread_state,
            );
        } else {
            libsais_final_sorting_scan_right_to_left_8u_omp(
                T,
                SA,
                0 as fast_sint_t,
                n as fast_sint_t,
                k,
                &mut *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
                thread_state,
            );
        }
        0
    } else if !I.is_null() {
        libsais_final_bwt_aux_scan_left_to_right_8u_omp(
            T,
            SA,
            n as fast_sint_t,
            k,
            r - 1,
            I,
            &mut *buckets.offset((6 * ((1) << 8)) as isize),
            threads,
            thread_state,
        );
        if threads > 1 && n >= 65536 {
            libsais_clear_lms_suffixes_omp(
                SA,
                n,
                (1) << 8,
                &mut *buckets.offset((6 * ((1) << 8)) as isize),
                &mut *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
            );
        }
        libsais_final_bwt_aux_scan_right_to_left_8u_omp(
            T,
            SA,
            n,
            k,
            r - 1,
            I,
            &mut *buckets.offset((7 * ((1) << 8)) as isize),
            threads,
            thread_state,
        );
        return 0;
    } else {
        libsais_final_bwt_scan_left_to_right_8u_omp(
            T,
            SA,
            n as fast_sint_t,
            k,
            &mut *buckets.offset((6 * ((1) << 8)) as isize),
            threads,
            thread_state,
        );
        if threads > 1 && n >= 65536 {
            libsais_clear_lms_suffixes_omp(
                SA,
                n,
                (1) << 8,
                &mut *buckets.offset((6 * ((1) << 8)) as isize),
                &mut *buckets.offset((7 * ((1) << 8)) as isize),
                threads,
            );
        }
        return libsais_final_bwt_scan_right_to_left_8u_omp(
            T,
            SA,
            n,
            k,
            &mut *buckets.offset((7 * ((1) << 8)) as isize),
            threads,
            thread_state,
        );
    }
}
unsafe fn libsais_induce_final_order_32s_6k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_final_sorting_scan_left_to_right_32s_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((4 * k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
    libsais_final_sorting_scan_right_to_left_32s_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((5 * k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
}
unsafe fn libsais_induce_final_order_32s_4k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_final_sorting_scan_left_to_right_32s_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((2 * k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
    libsais_final_sorting_scan_right_to_left_32s_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((3 * k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
}
unsafe fn libsais_induce_final_order_32s_2k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_final_sorting_scan_left_to_right_32s_omp(
        T,
        SA,
        n,
        &mut *buckets.offset((k as fast_sint_t) as isize),
        threads,
        thread_state,
    );
    libsais_final_sorting_scan_right_to_left_32s_omp(
        T,
        SA,
        n,
        &mut *buckets,
        threads,
        thread_state,
    );
}
unsafe fn libsais_induce_final_order_32s_1k(
    mut T: *const sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_count_suffixes_32s(T, n, k, buckets);
    libsais_initialize_buckets_start_32s_1k(k, buckets);
    libsais_final_sorting_scan_left_to_right_32s_omp(T, SA, n, buckets, threads, thread_state);
    libsais_count_suffixes_32s(T, n, k, buckets);
    libsais_initialize_buckets_end_32s_1k(k, buckets);
    libsais_final_sorting_scan_right_to_left_32s_omp(T, SA, n, buckets, threads, thread_state);
}
unsafe fn libsais_renumber_unique_and_nonunique_lms_suffixes_32s(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut f: sa_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) -> sa_sint_t {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAm: *mut sa_sint_t = &mut *SA.offset(m as isize) as *mut sa_sint_t;
    let mut i: sa_sint_t = 0;
    let mut j: sa_sint_t = 0;
    i = omp_block_start as sa_sint_t;
    j = omp_block_start as sa_sint_t + omp_block_size as sa_sint_t
        - 2 * prefetch_distance as sa_sint_t
        - 3;
    while i < j {
        libsais_prefetchr(SA.offset((i as c_long + 3 * prefetch_distance) as isize));
        libsais_prefetchw(SAm.offset(
            (*SA.offset((i as c_long + 2 * prefetch_distance) as isize) as sa_uint_t >> 1) as isize,
        ));
        libsais_prefetchw(SAm.offset(
            (*SA.offset((i as c_long + 2 * prefetch_distance + 1) as isize) as sa_uint_t >> 1)
                as isize,
        ));
        libsais_prefetchw(SAm.offset(
            (*SA.offset((i as c_long + 2 * prefetch_distance + 2) as isize) as sa_uint_t >> 1)
                as isize,
        ));
        libsais_prefetchw(SAm.offset(
            (*SA.offset((i as c_long + 2 * prefetch_distance + 3) as isize) as sa_uint_t >> 1)
                as isize,
        ));
        let mut q0: sa_uint_t = *SA.offset((i as c_long + prefetch_distance) as isize) as sa_uint_t;
        let mut Tq0: *mut sa_sint_t = &mut *T.offset(q0 as isize) as *mut sa_sint_t;
        libsais_prefetchw(if *SAm.offset((q0 >> 1) as isize) < 0 {
            Tq0
        } else {
            &mut *SAm.offset((q0 >> 1) as isize) as *mut sa_sint_t
        });
        let mut q1: sa_uint_t =
            *SA.offset((i as c_long + prefetch_distance + 1) as isize) as sa_uint_t;
        let mut Tq1: *mut sa_sint_t = &mut *T.offset(q1 as isize) as *mut sa_sint_t;
        libsais_prefetchw(if *SAm.offset((q1 >> 1) as isize) < 0 {
            Tq1
        } else {
            &mut *SAm.offset((q1 >> 1) as isize) as *mut sa_sint_t
        });
        let mut q2: sa_uint_t =
            *SA.offset((i as c_long + prefetch_distance + 2) as isize) as sa_uint_t;
        let mut Tq2: *mut sa_sint_t = &mut *T.offset(q2 as isize) as *mut sa_sint_t;
        libsais_prefetchw(if *SAm.offset((q2 >> 1) as isize) < 0 {
            Tq2
        } else {
            &mut *SAm.offset((q2 >> 1) as isize) as *mut sa_sint_t
        });
        let mut q3: sa_uint_t =
            *SA.offset((i as c_long + prefetch_distance + 3) as isize) as sa_uint_t;
        let mut Tq3: *mut sa_sint_t = &mut *T.offset(q3 as isize) as *mut sa_sint_t;
        libsais_prefetchw(if *SAm.offset((q3 >> 1) as isize) < 0 {
            Tq3
        } else {
            &mut *SAm.offset((q3 >> 1) as isize) as *mut sa_sint_t
        });
        let mut p0: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        let mut s0: sa_sint_t = *SAm.offset((p0 >> 1) as isize);
        if s0 < 0 {
            let fresh184 = &mut (*T.offset(p0 as isize));
            *fresh184 |= -(2147483647) - 1;
            f += 1;
            s0 = i + (-(2147483647) - 1) + f;
        }
        *SAm.offset((p0 >> 1) as isize) = s0 - f;
        let mut p1: sa_uint_t = *SA.offset((i + 1) as isize) as sa_uint_t;
        let mut s1: sa_sint_t = *SAm.offset((p1 >> 1) as isize);
        if s1 < 0 {
            let fresh185 = &mut (*T.offset(p1 as isize));
            *fresh185 |= -(2147483647) - 1;
            f += 1;
            s1 = i + 1 + (-(2147483647) - 1) + f;
        }
        *SAm.offset((p1 >> 1) as isize) = s1 - f;
        let mut p2: sa_uint_t = *SA.offset((i + 2) as isize) as sa_uint_t;
        let mut s2: sa_sint_t = *SAm.offset((p2 >> 1) as isize);
        if s2 < 0 {
            let fresh186 = &mut (*T.offset(p2 as isize));
            *fresh186 |= -(2147483647) - 1;
            f += 1;
            s2 = i + 2 + (-(2147483647) - 1) + f;
        }
        *SAm.offset((p2 >> 1) as isize) = s2 - f;
        let mut p3: sa_uint_t = *SA.offset((i + 3) as isize) as sa_uint_t;
        let mut s3: sa_sint_t = *SAm.offset((p3 >> 1) as isize);
        if s3 < 0 {
            let fresh187 = &mut (*T.offset(p3 as isize));
            *fresh187 |= -(2147483647) - 1;
            f += 1;
            s3 = i + 3 + (-(2147483647) - 1) + f;
        }
        *SAm.offset((p3 >> 1) as isize) = s3 - f;
        i += 4;
    }
    j += 2 * prefetch_distance as sa_sint_t + 3;
    while i < j {
        let mut p: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        let mut s: sa_sint_t = *SAm.offset((p >> 1) as isize);
        if s < 0 {
            let fresh188 = &mut (*T.offset(p as isize));
            *fresh188 |= -(2147483647) - 1;
            f += 1;
            s = i + (-(2147483647) - 1) + f;
        }
        *SAm.offset((p >> 1) as isize) = s - f;
        i += 1;
    }
    f
}
unsafe fn libsais_compact_unique_and_nonunique_lms_suffixes_32s(
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut pl: *mut fast_sint_t,
    mut pr: *mut fast_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAl: *mut sa_uint_t = &mut *SA as *mut sa_sint_t as *mut sa_uint_t;
    let mut SAr: *mut sa_uint_t = &mut *SA as *mut sa_sint_t as *mut sa_uint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let mut l: fast_sint_t = *pl - 1;
    let mut r: fast_sint_t = *pr - 1;
    i = m as fast_sint_t + omp_block_start + omp_block_size - 1;
    j = m as fast_sint_t + omp_block_start + 3;
    while i >= j {
        libsais_prefetchr(SA.offset((i - prefetch_distance) as isize));
        let mut p0: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        *SAl.offset(l as isize) = p0 & 2147483647;
        l -= ((p0 as sa_sint_t) < 0) as c_int as c_long;
        *SAr.offset(r as isize) = p0.wrapping_sub(1);
        r -= (p0 as sa_sint_t > 0) as c_int as c_long;
        let mut p1: sa_uint_t = *SA.offset((i - 1) as isize) as sa_uint_t;
        *SAl.offset(l as isize) = p1 & 2147483647;
        l -= ((p1 as sa_sint_t) < 0) as c_int as c_long;
        *SAr.offset(r as isize) = p1.wrapping_sub(1);
        r -= (p1 as sa_sint_t > 0) as c_int as c_long;
        let mut p2: sa_uint_t = *SA.offset((i - 2) as isize) as sa_uint_t;
        *SAl.offset(l as isize) = p2 & 2147483647;
        l -= ((p2 as sa_sint_t) < 0) as c_int as c_long;
        *SAr.offset(r as isize) = p2.wrapping_sub(1);
        r -= (p2 as sa_sint_t > 0) as c_int as c_long;
        let mut p3: sa_uint_t = *SA.offset((i - 3) as isize) as sa_uint_t;
        *SAl.offset(l as isize) = p3 & 2147483647;
        l -= ((p3 as sa_sint_t) < 0) as c_int as c_long;
        *SAr.offset(r as isize) = p3.wrapping_sub(1);
        r -= (p3 as sa_sint_t > 0) as c_int as c_long;
        i -= 4;
    }
    j -= 3;
    while i >= j {
        let mut p: sa_uint_t = *SA.offset(i as isize) as sa_uint_t;
        *SAl.offset(l as isize) = p & 2147483647;
        l -= ((p as sa_sint_t) < 0) as c_int as c_long;
        *SAr.offset(r as isize) = p.wrapping_sub(1);
        r -= (p as sa_sint_t > 0) as c_int as c_long;
        i -= 1;
    }
    *pl = l + 1;
    *pr = r + 1;
}
unsafe fn libsais_renumber_unique_and_nonunique_lms_suffixes_32s_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut f: sa_sint_t = 0;
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (m as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        m as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        f = libsais_renumber_unique_and_nonunique_lms_suffixes_32s(
            T,
            SA,
            m,
            0,
            omp_block_start,
            omp_block_size,
        );
    }
    f
}
unsafe fn libsais_compact_unique_and_nonunique_lms_suffixes_32s_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut f: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t =
        ((n as fast_sint_t >> 1) / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        (n as fast_sint_t >> 1) - omp_block_start
    };
    if omp_num_threads == 1 {
        let mut l: fast_sint_t = m as fast_sint_t;
        let mut r: fast_sint_t = n as fast_sint_t + fs as fast_sint_t;
        libsais_compact_unique_and_nonunique_lms_suffixes_32s(
            SA,
            m,
            &mut l,
            &mut r,
            omp_block_start,
            omp_block_size,
        );
    }
    memcpy(
        &mut *SA.offset((n as fast_sint_t + fs as fast_sint_t - m as fast_sint_t) as isize)
            as *mut sa_sint_t as *mut c_void,
        &mut *SA.offset((m as fast_sint_t - f as fast_sint_t) as isize) as *mut sa_sint_t
            as *const c_void,
        (f as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
    );
}
unsafe fn libsais_compact_lms_suffixes_32s_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut f: sa_sint_t =
        libsais_renumber_unique_and_nonunique_lms_suffixes_32s_omp(T, SA, m, threads, thread_state);
    libsais_compact_unique_and_nonunique_lms_suffixes_32s_omp(
        SA,
        n,
        m,
        fs,
        f,
        threads,
        thread_state,
    );
    f
}
unsafe fn libsais_merge_unique_lms_suffixes_32s(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut l: fast_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAnm: *const sa_sint_t =
        &mut *SA.offset((n as fast_sint_t - m as fast_sint_t - 1 + l) as isize) as *mut sa_sint_t;
    let mut i: sa_sint_t = 0;
    let mut j: sa_sint_t = 0;
    let fresh189 = SAnm;
    SAnm = SAnm.offset(1);
    let mut tmp: fast_sint_t = *fresh189 as fast_sint_t;
    i = omp_block_start as sa_sint_t;
    j = omp_block_start as sa_sint_t + omp_block_size as sa_sint_t - 6;
    while i < j {
        libsais_prefetchr(T.offset((i as c_long + prefetch_distance) as isize));
        let mut c0: sa_sint_t = *T.offset(i as isize);
        if c0 < 0 {
            *T.offset(i as isize) = c0 & 2147483647;
            *SA.offset(tmp as isize) = i;
            i += 1;
            let fresh190 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh190 as fast_sint_t;
        }
        let mut c1: sa_sint_t = *T.offset((i + 1) as isize);
        if c1 < 0 {
            *T.offset((i + 1) as isize) = c1 & 2147483647;
            *SA.offset(tmp as isize) = i + 1;
            i += 1;
            let fresh191 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh191 as fast_sint_t;
        }
        let mut c2: sa_sint_t = *T.offset((i + 2) as isize);
        if c2 < 0 {
            *T.offset((i + 2) as isize) = c2 & 2147483647;
            *SA.offset(tmp as isize) = i + 2;
            i += 1;
            let fresh192 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh192 as fast_sint_t;
        }
        let mut c3: sa_sint_t = *T.offset((i + 3) as isize);
        if c3 < 0 {
            *T.offset((i + 3) as isize) = c3 & 2147483647;
            *SA.offset(tmp as isize) = i + 3;
            i += 1;
            let fresh193 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh193 as fast_sint_t;
        }
        i += 4;
    }
    j += 6;
    while i < j {
        let mut c: sa_sint_t = *T.offset(i as isize);
        if c < 0 {
            *T.offset(i as isize) = c & 2147483647;
            *SA.offset(tmp as isize) = i;
            i += 1;
            let fresh194 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh194 as fast_sint_t;
        }
        i += 1;
    }
}
unsafe fn libsais_merge_nonunique_lms_suffixes_32s(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut l: fast_sint_t,
    mut omp_block_start: fast_sint_t,
    mut omp_block_size: fast_sint_t,
) {
    let prefetch_distance: fast_sint_t = 32 as fast_sint_t;
    let mut SAnm: *const sa_sint_t =
        &mut *SA.offset((n as fast_sint_t - m as fast_sint_t - 1 + l) as isize) as *mut sa_sint_t;
    let mut i: fast_sint_t = 0;
    let mut j: fast_sint_t = 0;
    let fresh195 = SAnm;
    SAnm = SAnm.offset(1);
    let mut tmp: sa_sint_t = *fresh195;
    i = omp_block_start;
    j = omp_block_start + omp_block_size - 3;
    while i < j {
        libsais_prefetchr(SA.offset((i + prefetch_distance) as isize));
        if *SA.offset(i as isize) == 0 {
            *SA.offset(i as isize) = tmp;
            let fresh196 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh196;
        }
        if *SA.offset((i + 1) as isize) == 0 {
            *SA.offset((i + 1) as isize) = tmp;
            let fresh197 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh197;
        }
        if *SA.offset((i + 2) as isize) == 0 {
            *SA.offset((i + 2) as isize) = tmp;
            let fresh198 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh198;
        }
        if *SA.offset((i + 3) as isize) == 0 {
            *SA.offset((i + 3) as isize) = tmp;
            let fresh199 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh199;
        }
        i += 4;
    }
    j += 3;
    while i < j {
        if *SA.offset(i as isize) == 0 {
            *SA.offset(i as isize) = tmp;
            let fresh200 = SAnm;
            SAnm = SAnm.offset(1);
            tmp = *fresh200;
        }
        i += 1;
    }
}
unsafe fn libsais_merge_unique_lms_suffixes_32s_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (n as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        n as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_merge_unique_lms_suffixes_32s(
            T,
            SA,
            n,
            m,
            0 as fast_sint_t,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_merge_nonunique_lms_suffixes_32s_omp(
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut f: sa_sint_t,
    mut _threads: sa_sint_t,
    mut _thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    let mut omp_thread_num: fast_sint_t = 0 as fast_sint_t;
    let mut omp_num_threads: fast_sint_t = 1 as fast_sint_t;
    let mut omp_block_stride: fast_sint_t = (m as c_long / omp_num_threads) & -(16) as c_long;
    let mut omp_block_start: fast_sint_t = omp_thread_num * omp_block_stride;
    let mut omp_block_size: fast_sint_t = if omp_thread_num < omp_num_threads - 1 {
        omp_block_stride
    } else {
        m as c_long - omp_block_start
    };
    if omp_num_threads == 1 {
        libsais_merge_nonunique_lms_suffixes_32s(
            SA,
            n,
            m,
            f as fast_sint_t,
            omp_block_start,
            omp_block_size,
        );
    }
}
unsafe fn libsais_merge_compacted_lms_suffixes_32s_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut f: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    libsais_merge_unique_lms_suffixes_32s_omp(T, SA, n, m, threads, thread_state);
    libsais_merge_nonunique_lms_suffixes_32s_omp(SA, n, m, f, threads, thread_state);
}
unsafe fn libsais_reconstruct_compacted_lms_suffixes_32s_2k_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut f: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if f > 0 {
        memmove(
            &mut *SA.offset((n - m - 1) as isize) as *mut sa_sint_t as *mut c_void,
            &mut *SA.offset((n + fs - m) as isize) as *mut sa_sint_t as *const c_void,
            (f as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        libsais_count_and_gather_compacted_lms_suffixes_32s_2k_omp(
            T,
            SA,
            n,
            k,
            buckets,
            threads,
            thread_state,
        );
        libsais_reconstruct_lms_suffixes_omp(SA, n, m - f, threads);
        memcpy(
            &mut *SA.offset((n - m - 1 + f) as isize) as *mut sa_sint_t as *mut c_void,
            &mut *SA as *mut sa_sint_t as *const c_void,
            (m as size_t)
                .wrapping_sub(f as size_t)
                .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        memset(
            &mut *SA as *mut sa_sint_t as *mut c_void,
            0,
            (m as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        libsais_merge_compacted_lms_suffixes_32s_omp(T, SA, n, m, f, threads, thread_state);
    } else {
        libsais_count_and_gather_lms_suffixes_32s_2k(
            T,
            SA,
            n,
            k,
            buckets,
            0 as fast_sint_t,
            n as fast_sint_t,
        );
        libsais_reconstruct_lms_suffixes_omp(SA, n, m, threads);
    };
}
unsafe fn libsais_reconstruct_compacted_lms_suffixes_32s_1k_omp(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut m: sa_sint_t,
    mut fs: sa_sint_t,
    mut f: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) {
    if f > 0 {
        memmove(
            &mut *SA.offset((n - m - 1) as isize) as *mut sa_sint_t as *mut c_void,
            &mut *SA.offset((n + fs - m) as isize) as *mut sa_sint_t as *const c_void,
            (f as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        libsais_gather_compacted_lms_suffixes_32s(T, SA, n);
        libsais_reconstruct_lms_suffixes_omp(SA, n, m - f, threads);
        memcpy(
            &mut *SA.offset((n - m - 1 + f) as isize) as *mut sa_sint_t as *mut c_void,
            &mut *SA as *mut sa_sint_t as *const c_void,
            (m as size_t)
                .wrapping_sub(f as size_t)
                .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        memset(
            &mut *SA as *mut sa_sint_t as *mut c_void,
            0,
            (m as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        libsais_merge_compacted_lms_suffixes_32s_omp(T, SA, n, m, f, threads, thread_state);
    } else {
        libsais_gather_lms_suffixes_32s(T, SA, n);
        libsais_reconstruct_lms_suffixes_omp(SA, n, m, threads);
    };
}
unsafe fn libsais_main_32s_recursion(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut fs: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
    mut local_buffer: *mut sa_sint_t,
) -> sa_sint_t {
    fs = if fs < 2147483647 - n {
        fs
    } else {
        2147483647 - n
    };
    if k > 0 && (fs / k >= 6 || 1024 / k >= 6 && threads == 1) {
        let mut alignment: sa_sint_t = if (fs - 1024) / k >= 6 { 1024 } else { 16 };
        let mut buckets: *mut sa_sint_t = if (fs - alignment) / k >= 6 {
            libsais_align_up(
                &mut *SA.offset(
                    ((n + fs) as c_long - 6 * k as fast_sint_t - alignment as c_long) as isize,
                ) as *mut sa_sint_t as *const c_void,
                (alignment as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            ) as *mut sa_sint_t
        } else {
            &mut *SA.offset(((n + fs) as c_long - 6 * k as fast_sint_t) as isize) as *mut sa_sint_t
        };
        buckets = if 1024 / k >= 6 && threads == 1 {
            local_buffer
        } else {
            buckets
        };
        let mut m: sa_sint_t = libsais_count_and_gather_lms_suffixes_32s_4k_omp(
            T,
            SA,
            n,
            k,
            buckets,
            threads,
            thread_state,
        );
        if m > 1 {
            memset(
                SA as *mut c_void,
                0,
                (n as size_t)
                    .wrapping_sub(m as size_t)
                    .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
            let mut first_lms_suffix: sa_sint_t = *SA.offset((n - m) as isize);
            let mut left_suffixes_count: sa_sint_t =
                libsais_initialize_buckets_for_lms_suffixes_radix_sort_32s_6k(
                    T,
                    k,
                    buckets,
                    first_lms_suffix,
                );
            libsais_radix_sort_lms_suffixes_32s_6k_omp(
                T,
                SA,
                n,
                m,
                &mut *buckets.offset((4 * k as fast_sint_t) as isize),
                threads,
                thread_state,
            );
            if (n / 8192) < k {
                libsais_radix_sort_set_markers_32s_6k_omp(
                    SA,
                    k,
                    &mut *buckets.offset((4 * k as fast_sint_t) as isize),
                    threads,
                );
            }
            if threads > 1 && n >= 65536 {
                memset(
                    &mut *SA.offset((n as fast_sint_t - m as fast_sint_t) as isize)
                        as *mut sa_sint_t as *mut c_void,
                    0,
                    (m as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                );
            }
            libsais_initialize_buckets_for_partial_sorting_32s_6k(
                T,
                k,
                buckets,
                first_lms_suffix,
                left_suffixes_count,
            );
            libsais_induce_partial_order_32s_6k_omp(
                T,
                SA,
                n,
                k,
                buckets,
                first_lms_suffix,
                left_suffixes_count,
                threads,
                thread_state,
            );
            let mut names: sa_sint_t = if (n / 8192) < k {
                libsais_renumber_and_mark_distinct_lms_suffixes_32s_4k_omp(
                    SA,
                    n,
                    m,
                    threads,
                    thread_state,
                )
            } else {
                libsais_renumber_and_gather_lms_suffixes_omp(SA, n, m, fs, threads, thread_state)
            };
            if names < m {
                let mut f: sa_sint_t = if (n / 8192) < k {
                    libsais_compact_lms_suffixes_32s_omp(T, SA, n, m, fs, threads, thread_state)
                } else {
                    0
                };
                if libsais_main_32s_recursion(
                    SA.offset(n as isize)
                        .offset(fs as isize)
                        .offset(-(m as isize))
                        .offset(f as isize),
                    SA,
                    m - f,
                    names - f,
                    fs + n - 2 * m + f,
                    threads,
                    thread_state,
                    local_buffer,
                ) != 0
                {
                    return -(2);
                }
                libsais_reconstruct_compacted_lms_suffixes_32s_2k_omp(
                    T,
                    SA,
                    n,
                    k,
                    m,
                    fs,
                    f,
                    buckets,
                    threads,
                    thread_state,
                );
            } else {
                libsais_count_lms_suffixes_32s_2k(T, n, k, buckets);
            }
            libsais_initialize_buckets_start_and_end_32s_4k(k, buckets);
            libsais_place_lms_suffixes_histogram_32s_4k(SA, n, k, m, buckets);
            libsais_induce_final_order_32s_4k(T, SA, n, k, buckets, threads, thread_state);
        } else {
            *SA = *SA.offset((n - 1) as isize);
            libsais_initialize_buckets_start_and_end_32s_6k(k, buckets);
            libsais_place_lms_suffixes_histogram_32s_6k(SA, n, k, m, buckets);
            libsais_induce_final_order_32s_6k(T, SA, n, k, buckets, threads, thread_state);
        }
        0
    } else if k > 0 && n <= 2147483647 / 2 && (fs / k >= 4 || 1024 / k >= 4 && threads == 1) {
        let mut alignment_0: sa_sint_t = if (fs - 1024) / k >= 4 { 1024 } else { 16 };
        let mut buckets_0: *mut sa_sint_t = if (fs - alignment_0) / k >= 4 {
            libsais_align_up(
                &mut *SA.offset(
                    ((n + fs) as c_long - 4 * k as fast_sint_t - alignment_0 as c_long) as isize,
                ) as *mut sa_sint_t as *const c_void,
                (alignment_0 as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            ) as *mut sa_sint_t
        } else {
            &mut *SA.offset(((n + fs) as c_long - 4 * k as fast_sint_t) as isize) as *mut sa_sint_t
        };
        buckets_0 = if 1024 / k >= 4 && threads == 1 {
            local_buffer
        } else {
            buckets_0
        };
        let mut m_0: sa_sint_t = libsais_count_and_gather_lms_suffixes_32s_2k_omp(
            T,
            SA,
            n,
            k,
            buckets_0,
            threads,
            thread_state,
        );
        if m_0 > 1 {
            libsais_initialize_buckets_for_radix_and_partial_sorting_32s_4k(
                T,
                k,
                buckets_0,
                *SA.offset((n - m_0) as isize),
            );
            libsais_radix_sort_lms_suffixes_32s_2k_omp(
                T,
                SA,
                n,
                m_0,
                &mut *buckets_0.offset(1),
                threads,
                thread_state,
            );
            libsais_radix_sort_set_markers_32s_4k_omp(SA, k, &mut *buckets_0.offset(1), threads);
            libsais_place_lms_suffixes_interval_32s_4k(SA, n, k, m_0 - 1, buckets_0);
            libsais_induce_partial_order_32s_4k_omp(T, SA, n, k, buckets_0, threads, thread_state);
            let mut names_0: sa_sint_t = libsais_renumber_and_mark_distinct_lms_suffixes_32s_4k_omp(
                SA,
                n,
                m_0,
                threads,
                thread_state,
            );
            if names_0 < m_0 {
                let mut f_0: sa_sint_t =
                    libsais_compact_lms_suffixes_32s_omp(T, SA, n, m_0, fs, threads, thread_state);
                if libsais_main_32s_recursion(
                    SA.offset(n as isize)
                        .offset(fs as isize)
                        .offset(-(m_0 as isize))
                        .offset(f_0 as isize),
                    SA,
                    m_0 - f_0,
                    names_0 - f_0,
                    fs + n - 2 * m_0 + f_0,
                    threads,
                    thread_state,
                    local_buffer,
                ) != 0
                {
                    return -(2);
                }
                libsais_reconstruct_compacted_lms_suffixes_32s_2k_omp(
                    T,
                    SA,
                    n,
                    k,
                    m_0,
                    fs,
                    f_0,
                    buckets_0,
                    threads,
                    thread_state,
                );
            } else {
                libsais_count_lms_suffixes_32s_2k(T, n, k, buckets_0);
            }
        } else {
            *SA = *SA.offset((n - 1) as isize);
        }
        libsais_initialize_buckets_start_and_end_32s_4k(k, buckets_0);
        libsais_place_lms_suffixes_histogram_32s_4k(SA, n, k, m_0, buckets_0);
        libsais_induce_final_order_32s_4k(T, SA, n, k, buckets_0, threads, thread_state);
        return 0;
    } else if k > 0 && (fs / k >= 2 || 1024 / k >= 2 && threads == 1) {
        let mut alignment_1: sa_sint_t = if (fs - 1024) / k >= 2 { 1024 } else { 16 };
        let mut buckets_1: *mut sa_sint_t = if (fs - alignment_1) / k >= 2 {
            libsais_align_up(
                &mut *SA.offset(
                    ((n + fs) as c_long - 2 * k as fast_sint_t - alignment_1 as c_long) as isize,
                ) as *mut sa_sint_t as *const c_void,
                (alignment_1 as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            ) as *mut sa_sint_t
        } else {
            &mut *SA.offset(((n + fs) as c_long - 2 * k as fast_sint_t) as isize) as *mut sa_sint_t
        };
        buckets_1 = if 1024 / k >= 2 && threads == 1 {
            local_buffer
        } else {
            buckets_1
        };
        let mut m_1: sa_sint_t = libsais_count_and_gather_lms_suffixes_32s_2k_omp(
            T,
            SA,
            n,
            k,
            buckets_1,
            threads,
            thread_state,
        );
        if m_1 > 1 {
            libsais_initialize_buckets_for_lms_suffixes_radix_sort_32s_2k(
                T,
                k,
                buckets_1,
                *SA.offset((n - m_1) as isize),
            );
            libsais_radix_sort_lms_suffixes_32s_2k_omp(
                T,
                SA,
                n,
                m_1,
                &mut *buckets_1.offset(1),
                threads,
                thread_state,
            );
            libsais_place_lms_suffixes_interval_32s_2k(SA, n, k, m_1 - 1, buckets_1);
            libsais_initialize_buckets_start_and_end_32s_2k(k, buckets_1);
            libsais_induce_partial_order_32s_2k_omp(T, SA, n, k, buckets_1, threads, thread_state);
            let mut names_1: sa_sint_t =
                libsais_renumber_and_mark_distinct_lms_suffixes_32s_1k_omp(T, SA, n, m_1, threads);
            if names_1 < m_1 {
                let mut f_1: sa_sint_t =
                    libsais_compact_lms_suffixes_32s_omp(T, SA, n, m_1, fs, threads, thread_state);
                if libsais_main_32s_recursion(
                    SA.offset(n as isize)
                        .offset(fs as isize)
                        .offset(-(m_1 as isize))
                        .offset(f_1 as isize),
                    SA,
                    m_1 - f_1,
                    names_1 - f_1,
                    fs + n - 2 * m_1 + f_1,
                    threads,
                    thread_state,
                    local_buffer,
                ) != 0
                {
                    return -(2);
                }
                libsais_reconstruct_compacted_lms_suffixes_32s_2k_omp(
                    T,
                    SA,
                    n,
                    k,
                    m_1,
                    fs,
                    f_1,
                    buckets_1,
                    threads,
                    thread_state,
                );
            } else {
                libsais_count_lms_suffixes_32s_2k(T, n, k, buckets_1);
            }
        } else {
            *SA = *SA.offset((n - 1) as isize);
        }
        libsais_initialize_buckets_end_32s_2k(k, buckets_1);
        libsais_place_lms_suffixes_histogram_32s_2k(SA, n, k, m_1, buckets_1);
        libsais_initialize_buckets_start_and_end_32s_2k(k, buckets_1);
        libsais_induce_final_order_32s_2k(T, SA, n, k, buckets_1, threads, thread_state);
        return 0;
    } else {
        let mut buffer: *mut sa_sint_t = if fs < k {
            libsais_alloc_aligned(
                (k as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                4096 as size_t,
            ) as *mut sa_sint_t
        } else {
            std::ptr::null_mut::<c_void>() as *mut sa_sint_t
        };
        let mut alignment_2: sa_sint_t = if fs - 1024 >= k { 1024 } else { 16 };
        let mut buckets_2: *mut sa_sint_t = if fs - alignment_2 >= k {
            libsais_align_up(
                &mut *SA.offset((n + fs - k - alignment_2) as isize) as *mut sa_sint_t
                    as *const c_void,
                (alignment_2 as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            ) as *mut sa_sint_t
        } else if fs >= k {
            &mut *SA.offset((n + fs - k) as isize) as *mut sa_sint_t
        } else {
            buffer
        };
        if buckets_2.is_null() {
            return -(2);
        }
        memset(
            SA as *mut c_void,
            0,
            (n as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
        libsais_count_suffixes_32s(T, n, k, buckets_2);
        libsais_initialize_buckets_end_32s_1k(k, buckets_2);
        let mut m_2: sa_sint_t = libsais_radix_sort_lms_suffixes_32s_1k(T, SA, n, buckets_2);
        if m_2 > 1 {
            libsais_induce_partial_order_32s_1k_omp(T, SA, n, k, buckets_2, threads, thread_state);
            let mut names_2: sa_sint_t =
                libsais_renumber_and_mark_distinct_lms_suffixes_32s_1k_omp(T, SA, n, m_2, threads);
            if names_2 < m_2 {
                if !buffer.is_null() {
                    libsais_free_aligned(buffer as *mut c_void);
                    buckets_2 = std::ptr::null_mut::<sa_sint_t>();
                }
                let mut f_2: sa_sint_t =
                    libsais_compact_lms_suffixes_32s_omp(T, SA, n, m_2, fs, threads, thread_state);
                if libsais_main_32s_recursion(
                    SA.offset(n as isize)
                        .offset(fs as isize)
                        .offset(-(m_2 as isize))
                        .offset(f_2 as isize),
                    SA,
                    m_2 - f_2,
                    names_2 - f_2,
                    fs + n - 2 * m_2 + f_2,
                    threads,
                    thread_state,
                    local_buffer,
                ) != 0
                {
                    return -(2);
                }
                libsais_reconstruct_compacted_lms_suffixes_32s_1k_omp(
                    T,
                    SA,
                    n,
                    m_2,
                    fs,
                    f_2,
                    threads,
                    thread_state,
                );
                if buckets_2.is_null() {
                    buffer = libsais_alloc_aligned(
                        (k as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
                        4096 as size_t,
                    ) as *mut sa_sint_t;
                    buckets_2 = buffer;
                }
                if buckets_2.is_null() {
                    return -(2);
                }
            }
            libsais_count_suffixes_32s(T, n, k, buckets_2);
            libsais_initialize_buckets_end_32s_1k(k, buckets_2);
            libsais_place_lms_suffixes_interval_32s_1k(T, SA, k, m_2, buckets_2);
        }
        libsais_induce_final_order_32s_1k(T, SA, n, k, buckets_2, threads, thread_state);
        libsais_free_aligned(buffer as *mut c_void);
        return 0;
    }
}
unsafe fn libsais_main_32s_entry(
    mut T: *mut sa_sint_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut k: sa_sint_t,
    mut fs: sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    let mut local_buffer: [sa_sint_t; 1024] = [0; 1024];
    libsais_main_32s_recursion(
        T,
        SA,
        n,
        k,
        fs,
        threads,
        thread_state,
        local_buffer.as_mut_ptr(),
    )
}
unsafe fn libsais_main_8u(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut buckets: *mut sa_sint_t,
    mut flags: sa_sint_t,
    mut r: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut fs: sa_sint_t,
    mut freq: *mut sa_sint_t,
    mut threads: sa_sint_t,
    mut thread_state: *mut LIBSAIS_THREAD_STATE,
) -> sa_sint_t {
    fs = if fs < 2147483647 - n {
        fs
    } else {
        2147483647 - n
    };
    let mut m: sa_sint_t =
        libsais_count_and_gather_lms_suffixes_8u_omp(T, SA, n, buckets, threads, thread_state);
    let mut k: sa_sint_t = libsais_initialize_buckets_start_and_end_8u(buckets, freq);
    if flags & 2 != 0 && (*buckets != 0 || *buckets.offset(2) != 0 || *buckets.offset(3) != 1) {
        return -(1);
    }
    if m > 0 {
        let mut first_lms_suffix: sa_sint_t = *SA.offset((n - m) as isize);
        let mut left_suffixes_count: sa_sint_t =
            libsais_initialize_buckets_for_lms_suffixes_radix_sort_8u(T, buckets, first_lms_suffix);
        if threads > 1 && n >= 65536 {
            memset(
                SA as *mut c_void,
                0,
                (n as size_t)
                    .wrapping_sub(m as size_t)
                    .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        libsais_radix_sort_lms_suffixes_8u_omp(T, SA, n, m, flags, buckets, threads, thread_state);
        if threads > 1 && n >= 65536 {
            memset(
                &mut *SA.offset((n as fast_sint_t - m as fast_sint_t) as isize) as *mut sa_sint_t
                    as *mut c_void,
                0,
                (m as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
            );
        }
        libsais_initialize_buckets_for_partial_sorting_8u(
            T,
            buckets,
            first_lms_suffix,
            left_suffixes_count,
        );
        libsais_induce_partial_order_8u_omp(
            T,
            SA,
            n,
            k,
            flags,
            buckets,
            first_lms_suffix,
            left_suffixes_count,
            threads,
            thread_state,
        );
        let mut names: sa_sint_t =
            libsais_renumber_and_gather_lms_suffixes_omp(SA, n, m, fs, threads, thread_state);
        if names < m {
            if libsais_main_32s_entry(
                SA.offset(n as isize)
                    .offset(fs as isize)
                    .offset(-(m as isize)),
                SA,
                m,
                names,
                fs + n - 2 * m,
                threads,
                thread_state,
            ) != 0
            {
                return -(2);
            }
            libsais_gather_lms_suffixes_8u_omp(T, SA, n, threads, thread_state);
            libsais_reconstruct_lms_suffixes_omp(SA, n, m, threads);
        }
        libsais_place_lms_suffixes_interval_8u(SA, n, m, flags, buckets);
    } else {
        memset(
            SA as *mut c_void,
            0,
            (n as size_t).wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        );
    }
    libsais_induce_final_order_8u_omp(T, SA, n, k, flags, r, I, buckets, threads, thread_state)
}
unsafe fn libsais_main(
    mut T: *const uint8_t,
    mut SA: *mut sa_sint_t,
    mut n: sa_sint_t,
    mut flags: sa_sint_t,
    mut r: sa_sint_t,
    mut I: *mut sa_sint_t,
    mut fs: sa_sint_t,
    mut freq: *mut sa_sint_t,
    mut threads: sa_sint_t,
) -> sa_sint_t {
    let mut thread_state: *mut LIBSAIS_THREAD_STATE = if threads > 1 {
        libsais_alloc_thread_state(threads)
    } else {
        std::ptr::null_mut::<LIBSAIS_THREAD_STATE>()
    };
    let mut buckets: *mut sa_sint_t = libsais_alloc_aligned(
        (8 as size_t)
            .wrapping_mul(((1) << 8) as c_ulong)
            .wrapping_mul(size_of::<sa_sint_t>() as c_ulong),
        4096 as size_t,
    ) as *mut sa_sint_t;
    let mut index: sa_sint_t = if !buckets.is_null() && (!thread_state.is_null() || threads == 1) {
        libsais_main_8u(
            T,
            SA,
            n,
            buckets,
            flags,
            r,
            I,
            fs,
            freq,
            threads,
            thread_state,
        )
    } else {
        -(2)
    };
    libsais_free_aligned(buckets as *mut c_void);
    libsais_free_thread_state(thread_state);
    index
}
pub unsafe fn libsais(
    mut T: *const uint8_t,
    mut SA: *mut int32_t,
    mut n: int32_t,
    mut fs: int32_t,
    mut freq: *mut int32_t,
) -> int32_t {
    if T.is_null() || SA.is_null() || n < 0 || fs < 0 {
        return -(1);
    } else if n <= 1 {
        if !freq.is_null() {
            memset(
                freq as *mut c_void,
                0,
                (((1) << 8) as c_ulong).wrapping_mul(size_of::<int32_t>() as c_ulong),
            );
        }
        if n == 1 {
            *SA = 0;
            if !freq.is_null() {
                let fresh201 = &mut (*freq.offset(*T as isize));
                *fresh201 += 1;
            }
        }
        return 0;
    }
    libsais_main(
        T,
        SA,
        n,
        0,
        0,
        std::ptr::null_mut::<sa_sint_t>(),
        fs,
        freq,
        1,
    )
}
