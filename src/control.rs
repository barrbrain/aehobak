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

#[derive(Debug, PartialEq)]
pub struct Bsdiff {
    pub add: u64,
    pub copy: u64,
    pub seek: i64,
}

impl From<&Aehobak> for Bsdiff {
    fn from(control: &Aehobak) -> Self {
        Bsdiff {
            add: control.add.into(),
            copy: control.copy.into(),
            seek: control.seek.into(),
        }
    }
}

impl TryFrom<&[u8]> for Bsdiff {
    type Error = std::array::TryFromSliceError;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        let _: &[u8; 24] = buf.try_into()?;
        fn to_i64(x: u64) -> i64 {
            if x <= 1 << 63 {
                x as i64
            } else {
                -((1 << 63) ^ x as i64)
            }
        }
        Ok(Self {
            add: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            copy: u64::from_le_bytes(buf[8..16].try_into().unwrap()),
            seek: to_i64(u64::from_le_bytes(buf[16..24].try_into().unwrap())),
        })
    }
}

impl Bsdiff {
    pub fn encode(&self, patch: &mut Vec<u8>) {
        fn to_u64(x: i64) -> u64 {
            if x >= 0 {
                x as u64
            } else {
                (1 << 63) | (x as u64).wrapping_neg()
            }
        }
        patch.extend(&self.add.to_le_bytes());
        patch.extend(&self.copy.to_le_bytes());
        patch.extend(&to_u64(self.seek).to_le_bytes());
    }
}

#[derive(Debug, PartialEq)]
pub struct Aehobak {
    pub add: u32,
    pub copy: u32,
    pub seek: i32,
}

impl TryFrom<Bsdiff> for Aehobak {
    type Error = std::num::TryFromIntError;

    fn try_from(control: Bsdiff) -> Result<Self, Self::Error> {
        Ok(Aehobak {
            add: control.add.try_into()?,
            copy: control.copy.try_into()?,
            seek: control.seek.try_into()?,
        })
    }
}

impl TryFrom<&[u32]> for Aehobak {
    type Error = std::array::TryFromSliceError;

    fn try_from(vbytes: &[u32]) -> Result<Self, Self::Error> {
        let _: &[u32; 3] = vbytes.try_into()?;
        fn to_i32(x: u32) -> i32 {
            (x >> 1) as i32 ^ ((x as i32 & 1) << 31 >> 31)
        }
        Ok(Self {
            add: vbytes[0],
            copy: vbytes[1],
            seek: to_i32(vbytes[2]),
        })
    }
}

impl Aehobak {
    pub fn encode(&self, vbytes: (&mut Vec<u32>, &mut Vec<u32>, &mut Vec<u32>)) {
        fn to_u32(x: i32) -> u32 {
            ((x >> 31) ^ (x << 1)) as u32
        }
        vbytes.0.push(self.add);
        vbytes.1.push(self.copy);
        vbytes.2.push(to_u32(self.seek));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    quickcheck! {
        fn bsdiff_round_trip(add: u64, copy: u64, seek: i64) -> bool {
            let reference = Bsdiff { add, copy, seek};
            let mut patch = Vec::new();
            reference.encode(&mut patch);
            let decoded: Bsdiff = patch.as_slice().try_into().unwrap();
            decoded == reference
        }

        fn aehobak_round_trip(add: u32, copy: u32, seek: i32) -> bool {
            let reference = Aehobak { add, copy, seek};
            let mut adds = Vec::new();
            let mut copies = Vec::new();
            let mut seeks = Vec::new();
            reference.encode((&mut adds, &mut copies, &mut seeks));
            let patch = [adds[0], copies[0], seeks[0]];
            let decoded: Aehobak = patch.as_slice().try_into().unwrap();
            decoded == reference
        }

        fn aehobak_into_bsdiff(add: u32, copy: u32, seek: i32) -> bool {
            let reference = Aehobak { add, copy, seek};
            let bsdiff: Bsdiff = (&reference).into();
            let decoded: Aehobak = bsdiff.try_into().unwrap();
            decoded == reference
        }
    }

    #[test]
    fn bsdiff_vectors() {
        let mut patch = vec![0; 24];
        for (v, (add, copy, seek)) in [
            ((0, 0, 0, 0), (0, 0, 0)),
            ((1, 1, 1, 0), (1, 1, 1)),
            ((0, 0, 1, 128), (0, 0, -1)),
            ((0, 0, 0, 128), (0, 0, i64::MIN)),
        ] {
            patch[0] = v.0;
            patch[8] = v.1;
            patch[16] = v.2;
            patch[23] = v.3;

            let decoded: Bsdiff = patch.as_slice().try_into().unwrap();
            let reference = Bsdiff { add, copy, seek };
            assert_eq!(decoded, reference);
        }
    }

    #[test]
    fn aehobak_vectors() {
        let mut patch = vec![0; 3];
        for (v, (add, copy, seek)) in [
            ((0, 0, 0), (0, 0, 0)),
            ((1, 1, 2), (1, 1, 1)),
            ((0, 0, 1), (0, 0, -1)),
        ] {
            patch[0] = v.0;
            patch[1] = v.1;
            patch[2] = v.2;

            let decoded: Aehobak = patch.as_slice().try_into().unwrap();
            let reference = Aehobak { add, copy, seek };
            assert_eq!(decoded, reference);
        }
    }
}
