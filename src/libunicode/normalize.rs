// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Functions for computing canonical and compatible decompositions for Unicode characters.

use core::cmp::Ordering::{Equal, Less, Greater};
use core::ops::FnMut;
use core::option::Option;
use core::option::Option::{Some, None};
use core::slice::SliceExt;
use core::result::Result::{Ok, Err};
use tables::normalization::{canonical_table, compatibility_table, composition_table};

fn bsearch_table<T>(c: char, r: &'static [(char, &'static [T])]) -> Option<&'static [T]> {
    match r.binary_search_by(|&(val, _)| {
        if c == val { Equal }
        else if val < c { Less }
        else { Greater }
    }) {
        Ok(idx) => {
            let (_, result) = r[idx];
            Some(result)
        }
        Err(_) => None
    }
}

/// Compute canonical Unicode decomposition for character
pub fn decompose_canonical<F>(c: char, mut i: F) where F: FnMut(char) { d(c, &mut i, false); }

/// Compute canonical or compatible Unicode decomposition for character
pub fn decompose_compatible<F>(c: char, mut i: F) where F: FnMut(char) { d(c, &mut i, true); }

// FIXME(#19596) This is a workaround, we should use `F` instead of `&mut F`
fn d<F>(c: char, i: &mut F, k: bool) where F: FnMut(char) {
    // 7-bit ASCII never decomposes
    if c <= '\x7f' { (*i)(c); return; }

    // Perform decomposition for Hangul
    if (c as u32) >= S_BASE && (c as u32) < (S_BASE + S_COUNT) {
        decompose_hangul(c, i);
        return;
    }

    // First check the canonical decompositions
    match bsearch_table(c, canonical_table) {
        Some(canon) => {
            for x in canon {
                d(*x, i, k);
            }
            return;
        }
        None => ()
    }

    // Bottom out if we're not doing compat.
    if !k { (*i)(c); return; }

    // Then check the compatibility decompositions
    match bsearch_table(c, compatibility_table) {
        Some(compat) => {
            for x in compat {
                d(*x, i, k);
            }
            return;
        }
        None => ()
    }

    // Finally bottom out.
    (*i)(c);
}

pub fn compose(a: char, b: char) -> Option<char> {
    compose_hangul(a, b).or_else(|| {
        match bsearch_table(a, composition_table) {
            None => None,
            Some(candidates) => {
                match candidates.binary_search_by(|&(val, _)| {
                    if b == val { Equal }
                    else if val < b { Less }
                    else { Greater }
                }) {
                    Ok(idx) => {
                        let (_, result) = candidates[idx];
                        Some(result)
                    }
                    Err(_) => None
                }
            }
        }
    })
}

// Constants from Unicode 6.3.0 Section 3.12 Conjoining Jamo Behavior
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = (V_COUNT * T_COUNT);
const S_COUNT: u32 = (L_COUNT * N_COUNT);

// FIXME(#19596) This is a workaround, we should use `F` instead of `&mut F`
// Decompose a precomposed Hangul syllable
#[inline(always)]
fn decompose_hangul<F>(s: char, f: &mut F) where F: FnMut(char) {
    use core::mem::transmute;

    let si = s as u32 - S_BASE;

    let li = si / N_COUNT;
    unsafe {
        (*f)(transmute(L_BASE + li));

        let vi = (si % N_COUNT) / T_COUNT;
        (*f)(transmute(V_BASE + vi));

        let ti = si % T_COUNT;
        if ti > 0 {
            (*f)(transmute(T_BASE + ti));
        }
    }
}

// Compose a pair of Hangul Jamo
#[inline(always)]
fn compose_hangul(a: char, b: char) -> Option<char> {
    use core::mem::transmute;
    let l = a as u32;
    let v = b as u32;
    // Compose an LPart and a VPart
    if L_BASE <= l && l < (L_BASE + L_COUNT) && V_BASE <= v && v < (V_BASE + V_COUNT) {
        let r = S_BASE + (l - L_BASE) * N_COUNT + (v - V_BASE) * T_COUNT;
        return unsafe { Some(transmute(r)) };
    }
    // Compose an LVPart and a TPart
    if S_BASE <= l && l <= (S_BASE+S_COUNT-T_COUNT) && T_BASE <= v && v < (T_BASE+T_COUNT) {
        let r = l + (v - T_BASE);
        return unsafe { Some(transmute(r)) };
    }
    None
}
