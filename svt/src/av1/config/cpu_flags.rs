#![allow(missing_docs)]

use bitflags::bitflags;

/// Enabled assembly instruction sets.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct CpuFlags(u64);

#[cfg(target_arch = "x86_64")]
bitflags! {
    impl CpuFlags: u64 {
        const EB_CPU_FLAGS_MMX = 1 << 0;
        const EB_CPU_FLAGS_SSE = 1 << 1;
        const EB_CPU_FLAGS_SSE2 = 1 << 2;
        const EB_CPU_FLAGS_SSE3 = 1 << 3;
        const EB_CPU_FLAGS_SSSE3 = 1 << 4;
        const EB_CPU_FLAGS_SSE41 = 1 << 5;
        const EB_CPU_FLAGS_SSE42 = 1 << 6;
        const EB_CPU_FLAGS_AVX = 1 << 7;
        const EB_CPU_FLAGS_AVX2 = 1 << 8;
        const EB_CPU_FLAGS_AVX512F = 1 << 9;
        const EB_CPU_FLAGS_AVX512ICD = 1 << 10;
        const EB_CPU_FLAGS_AVX512DQ = 1 << 11;
        const EB_CPU_FLAGS_AVX512ER = 1 << 12;
        const EB_CPU_FLAGS_AVX512PF = 1 << 13;
        const EB_CPU_FLAGS_AVX512BW = 1 << 14;
        const EB_CPU_FLAGS_AVX512VL = 1 << 15;
        const EB_CPU_FLAGS_INVALID = 1 << 63;
        const EB_CPU_FLAGS_ALL = u64::MAX;
    }
}

#[cfg(target_arch = "aarch64")]
bitflags! {
    impl CpuFlags: u64 {
        const EB_CPU_FLAGS_NEON = 1 << 0;
        const EB_CPU_FLAGS_INVALID = 1 << 63;
        const EB_CPU_FLAGS_ALL = u64::MAX;
    }
}
