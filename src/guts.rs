use crate::*;
use core::mem;

// Variants other than Portable are unreachable in no_std, unless CPU features
// are explicitly enabled for the build with e.g. RUSTFLAGS="-C target-feature=avx2".
// This might change in the future if is_x86_feature_detected moves into libcore.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Platform {
    Portable,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    SSE41,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    AVX2,
}

#[derive(Clone, Copy, Debug)]
pub struct Implementation(Platform);

impl Implementation {
    pub fn detect() -> Self {
        // Try the different implementations in order of how fast/modern they
        // are. Currently on non-x86, everything just uses portable.
        if let Some(avx2_impl) = Self::avx2_if_supported() {
            avx2_impl
        } else if let Some(sse41_impl) = Self::sse41_if_supported() {
            sse41_impl
        } else {
            Self::portable()
        }
    }

    pub fn portable() -> Self {
        Implementation(Platform::Portable)
    }

    #[allow(unreachable_code)]
    pub fn avx2_if_supported() -> Option<Self> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // Check whether AVX2 support is assumed by the build (using
            // something like RUSTFLAGS="-C target-feature=avx2" or
            // RUSTFLAGS="-C target-cpu=native"). This isn't the common case,
            // but it's the only way to use AVX2 with no_std, at least until
            // more features get stabilized in the future.
            #[cfg(target_feature = "avx2")]
            {
                return Some(Implementation(Platform::AVX2));
            }
            // If AVX2 support isn't assumed (it's usually not), do dynamic
            // feature detection to see if we can use it on the current system.
            #[cfg(feature = "std")]
            {
                if is_x86_feature_detected!("avx2") {
                    return Some(Implementation(Platform::AVX2));
                }
            }
        }
        None
    }

    #[allow(unreachable_code)]
    pub fn sse41_if_supported() -> Option<Self> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // As with AVX2, check whether SSE4.1 is assumed by the
            // build.
            #[cfg(target_feature = "sse4.1")]
            {
                return Some(Implementation(Platform::SSE41));
            }
            // As with AVX2, if support isn't assumed, dynamically check for
            // it.
            #[cfg(feature = "std")]
            {
                if is_x86_feature_detected!("sse4.1") {
                    return Some(Implementation(Platform::SSE41));
                }
            }
        }
        None
    }

    pub fn compress(
        &self,
        state_words: &mut [u64; 8],
        msg: &[u8; BLOCKBYTES],
        count: u128,
        lastblock: u64,
        lastnode: u64,
    ) {
        match self.0 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::AVX2 => unsafe {
                avx2::compress(state_words, msg, count, lastblock, lastnode)
            },
            // The SSE4.1 implementation of compress hasn't yet been ported
            // from https://github.com/BLAKE2/BLAKE2/blob/master/sse/blake2b-round.h,
            // so for SSE4.1 falls back to portable.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::SSE41 => portable::compress(state_words, msg, count, lastblock, lastnode),
            Platform::Portable => portable::compress(state_words, msg, count, lastblock, lastnode),
        }
    }

    pub fn transpose2(&self, words0: &[u64; 8], words1: &[u64; 8]) -> [u64x2; 8] {
        // Currently there's only the portable implementation of transpose2.
        portable::transpose2(words0, words1)
    }

    pub fn untranspose2(&self, transposed: &[u64x2; 8], out0: &mut [u64; 8], out1: &mut [u64; 8]) {
        // Currently there's only the portable implementation of untranspose2.
        portable::untranspose2(transposed, out0, out1)
    }

    pub fn compress2(
        &self,
        transposed_state_words: &mut [u64x2; 8],
        msg0: &[u8; BLOCKBYTES],
        msg1: &[u8; BLOCKBYTES],
        count_low: &u64x2,
        count_high: &u64x2,
        lastblock: &u64x2,
        lastnode: &u64x2,
    ) {
        match self.0 {
            // Currently there's no AVX2 implementation of compress2, fall back to SSE4.1.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::AVX2 | Platform::SSE41 => unsafe {
                sse41::compress2_transposed(
                    transposed_state_words,
                    msg0,
                    msg1,
                    count_low,
                    count_high,
                    lastblock,
                    lastnode,
                );
            },
            Platform::Portable => {
                portable::compress2_transposed(
                    transposed_state_words,
                    msg0,
                    msg1,
                    count_low,
                    count_high,
                    lastblock,
                    lastnode,
                );
            }
        }
    }

    pub fn transpose4(
        &self,
        words0: &[u64; 8],
        words1: &[u64; 8],
        words2: &[u64; 8],
        words3: &[u64; 8],
    ) -> [u64x4; 8] {
        match self.0 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::AVX2 => unsafe { avx2::transpose4(words0, words1, words2, words3) },
            // There is no SSE4.1 implementation of transpose4 yet.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::SSE41 => portable::transpose4(words0, words1, words2, words3),
            Platform::Portable => portable::transpose4(words0, words1, words2, words3),
        }
    }

    pub fn untranspose4(
        &self,
        transposed: &[u64x4; 8],
        out0: &mut [u64; 8],
        out1: &mut [u64; 8],
        out2: &mut [u64; 8],
        out3: &mut [u64; 8],
    ) {
        match self.0 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::AVX2 => unsafe { avx2::untranspose4(transposed, out0, out1, out2, out3) },
            // There is no SSE4.1 implementation of untranspose4 yet.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::SSE41 => portable::untranspose4(transposed, out0, out1, out2, out3),
            Platform::Portable => portable::untranspose4(transposed, out0, out1, out2, out3),
        }
    }

    pub fn compress4(
        &self,
        transposed_state_words: &mut [u64x4; 8],
        msg0: &[u8; BLOCKBYTES],
        msg1: &[u8; BLOCKBYTES],
        msg2: &[u8; BLOCKBYTES],
        msg3: &[u8; BLOCKBYTES],
        count_low: &u64x4,
        count_high: &u64x4,
        lastblock: &u64x4,
        lastnode: &u64x4,
    ) {
        match self.0 {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::AVX2 => unsafe {
                avx2::compress4_transposed(
                    transposed_state_words,
                    msg0,
                    msg1,
                    msg2,
                    msg3,
                    count_low,
                    count_high,
                    lastblock,
                    lastnode,
                );
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            Platform::SSE41 => unsafe {
                sse41::compress4_transposed(
                    transposed_state_words,
                    msg0,
                    msg1,
                    msg2,
                    msg3,
                    count_low,
                    count_high,
                    lastblock,
                    lastnode,
                );
            },
            Platform::Portable => {
                portable::compress4_transposed(
                    transposed_state_words,
                    msg0,
                    msg1,
                    msg2,
                    msg3,
                    count_low,
                    count_high,
                    lastblock,
                    lastnode,
                );
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, align(16))]
pub struct u64x2(pub [u64; 2]);

impl core::ops::Deref for u64x2 {
    type Target = [u64; 2];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for u64x2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, align(32))]
pub struct u64x4(pub [u64; 4]);

impl u64x4 {
    #[inline(always)]
    pub(crate) fn split(&self) -> &[u64x2; 2] {
        // Safety note: The 32-byte alignment of u64x4 guarantees that each
        // half of it will be 16-byte aligned, and the C repr guarantees that
        // the layout is exactly four packed u64's.
        unsafe { mem::transmute(self) }
    }

    #[inline(always)]
    pub(crate) fn split_mut(&mut self) -> &mut [u64x2; 2] {
        // Safety note: The 32-byte alignment of u64x4 guarantees that each
        // half of it will be 16-byte aligned, and the C repr guarantees that
        // the layout is exactly four packed u64's.
        unsafe { mem::transmute(self) }
    }
}

impl core::ops::Deref for u64x4 {
    type Target = [u64; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for u64x4 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_detection() {
        assert_eq!(Platform::Portable, Implementation::portable().0);

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        #[cfg(feature = "std")]
        {
            if is_x86_feature_detected!("avx2") {
                assert_eq!(Platform::AVX2, Implementation::detect().0);
                assert_eq!(
                    Platform::AVX2,
                    Implementation::avx2_if_supported().unwrap().0
                );
                assert_eq!(
                    Platform::SSE41,
                    Implementation::sse41_if_supported().unwrap().0
                );
            } else if is_x86_feature_detected!("sse4.1") {
                assert_eq!(Platform::SSE41, Implementation::detect().0);
                assert!(Implementation::avx2_if_supported().is_none());
                assert_eq!(
                    Platform::SSE41,
                    Implementation::sse41_if_supported().unwrap().0
                );
            } else {
                assert_eq!(Platform::Portable, Implementation::detect().0);
                assert!(Implementation::avx2_if_supported().is_none());
                assert!(Implementation::sse41_if_supported().is_none());
            }
        }
    }

    fn input_state_words(i: u64) -> [u64; 8] {
        let mut words = [0; 8];
        for j in 0..words.len() {
            words[j] = i + j as u64;
        }
        words
    }

    fn input_msg_block(i: u64) -> [u8; 128] {
        let mut block = [0; 128];
        for j in 0..block.len() {
            block[j] = (i + j as u64) as u8;
        }
        block
    }

    fn exercise_1(imp: Implementation, i: u64) -> [u64; 8] {
        let mut state = input_state_words(i);
        let block = input_msg_block(0x10 + i);
        let count_low = 0x20 + i;
        let count_high = 0x30 + i;
        let count = count_low as u128 + ((count_high as u128) << 64);
        let lastblock = 0x40 + i;
        let lastnode = 0x50 + i;
        imp.compress(&mut state, &block, count, lastblock, lastnode);
        state
    }

    fn exercise_2(imp: Implementation, i: u64) -> [[u64; 8]; 2] {
        let mut state0 = input_state_words(i);
        let mut state1 = input_state_words(i + 1);
        let block0 = input_msg_block(0x10 + i);
        let block1 = input_msg_block(0x10 + i + 1);
        let count_low = u64x2([0x20 + i, 0x20 + i + 1]);
        let count_high = u64x2([0x30 + i, 0x30 + i + 1]);
        let lastblock = u64x2([0x40 + i, 0x40 + i + 1]);
        let lastnode = u64x2([0x50 + i, 0x50 + i + 1]);
        let mut transposed = imp.transpose2(&state0, &state1);
        imp.compress2(
            &mut transposed,
            &block0,
            &block1,
            &count_low,
            &count_high,
            &lastblock,
            &lastnode,
        );
        imp.untranspose2(&transposed, &mut state0, &mut state1);
        [state0, state1]
    }

    fn exercise_4(imp: Implementation, i: u64) -> [[u64; 8]; 4] {
        let mut state0 = input_state_words(i);
        let mut state1 = input_state_words(i + 1);
        let mut state2 = input_state_words(i + 2);
        let mut state3 = input_state_words(i + 3);
        let block0 = input_msg_block(0x10 + i);
        let block1 = input_msg_block(0x10 + i + 1);
        let block2 = input_msg_block(0x10 + i + 2);
        let block3 = input_msg_block(0x10 + i + 3);
        let count_low = u64x4([0x20 + i, 0x20 + i + 1, 0x20 + i + 2, 0x20 + i + 3]);
        let count_high = u64x4([0x30 + i, 0x30 + i + 1, 0x30 + i + 2, 0x30 + i + 3]);
        let lastblock = u64x4([0x40 + i, 0x40 + i + 1, 0x40 + i + 2, 0x40 + i + 3]);
        let lastnode = u64x4([0x50 + i, 0x50 + i + 1, 0x50 + i + 2, 0x50 + i + 3]);
        let mut transposed = imp.transpose4(&state0, &state1, &state2, &state3);
        imp.compress4(
            &mut transposed,
            &block0,
            &block1,
            &block2,
            &block3,
            &count_low,
            &count_high,
            &lastblock,
            &lastnode,
        );
        imp.untranspose4(
            &transposed,
            &mut state0,
            &mut state1,
            &mut state2,
            &mut state3,
        );
        [state0, state1, state2, state3]
    }

    // Make sure the different portable APIs all agree with each other. We
    // don't use known test vectors here; that happens in vector_tests.rs.
    #[test]
    fn test_portable() {
        let portable = Implementation::portable();

        // Gather the one-at-a-time compression results.
        let expected0 = exercise_1(portable, 0);
        let expected1 = exercise_1(portable, 1);
        let expected2 = exercise_1(portable, 2);
        let expected3 = exercise_1(portable, 3);

        // Check that compress2 gives the same answer.
        let two_at_a_time01 = exercise_2(portable, 0);
        let two_at_a_time12 = exercise_2(portable, 2);
        assert_eq!(expected0, two_at_a_time01[0]);
        assert_eq!(expected1, two_at_a_time01[1]);
        assert_eq!(expected2, two_at_a_time12[0]);
        assert_eq!(expected3, two_at_a_time12[1]);

        // Check that compress4 gives the same answer.
        let four_at_a_time = exercise_4(portable, 0);
        assert_eq!(expected0, four_at_a_time[0]);
        assert_eq!(expected1, four_at_a_time[1]);
        assert_eq!(expected2, four_at_a_time[2]);
        assert_eq!(expected3, four_at_a_time[3]);
    }

    // Make sure that SSE41 agrees with portable. We don't use known test
    // vectors here; that happens in vector_tests.rs.
    #[test]
    fn test_sse41() {
        let portable = Implementation::portable();
        let sse41 = if let Some(imp) = Implementation::sse41_if_supported() {
            imp
        } else {
            // No SSE4.1 support. Short circuit the test.
            return;
        };

        assert_eq!(exercise_1(portable, 0), exercise_1(sse41, 0));
        assert_eq!(exercise_2(portable, 0), exercise_2(sse41, 0));
        assert_eq!(exercise_4(portable, 0), exercise_4(sse41, 0));
    }

    // Make sure that AVX2 agrees with portable. We don't use known test
    // vectors here; that happens in vector_tests.rs.
    #[test]
    fn test_avx2() {
        let portable = Implementation::portable();
        let avx2 = if let Some(imp) = Implementation::avx2_if_supported() {
            imp
        } else {
            // No AVX2 support. Short circuit the test.
            return;
        };

        assert_eq!(exercise_1(portable, 0), exercise_1(avx2, 0));
        assert_eq!(exercise_2(portable, 0), exercise_2(avx2, 0));
        assert_eq!(exercise_4(portable, 0), exercise_4(avx2, 0));
    }
}
