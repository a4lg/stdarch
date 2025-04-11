//! Run-time feature detection for RISC-V on Linux.

use super::auxvec;
use crate::detect::{Feature, bit, cache};

/// Imply features by the given set of enabled features.
///
/// Note that it does not perform any consistency checks including existence of
/// conflicting extensions and/or complicated requirements.  Eliminating such
/// inconsistencies is the responsibility of the feature detection logic and
/// its provider(s).
pub(crate) fn imply_features(mut value: cache::Initializer) -> cache::Initializer {
    loop {
        // Check convergence of feature flags later.
        let prev = value;

        // Expect that the optimizer turns repeated operations into
        // a fewer number of bit-manipulation operations.
        macro_rules! imply {
            // Regular implication:
            // A1 => (B1[, B2...]), A2 => (B1[, B2...]) and so on.
            ($($from: ident)|+ => $($to: ident)&+) => {
                if [$(Feature::$from as u32),+].iter().any(|&x| value.test(x)) {
                    $(
                        value.set(Feature::$to as u32);
                    )+
                }
            };
            // Implication with multiple requirements:
            // A1 && A2 ... => (B1[, B2...]).
            ($($from: ident)&+ => $($to: ident)&+) => {
                if [$(Feature::$from as u32),+].iter().all(|&x| value.test(x)) {
                    $(
                        value.set(Feature::$to as u32);
                    )+
                }
            };
        }
        macro_rules! group {
            ($group: ident == $($member: ident)&+) => {
                // Forward implication as defined in the specifications.
                imply!($group => $($member)&+);
                // Reverse implication to "group extension" from its members.
                // This is not a part of specifications but convenient for
                // feature detection and implemented in e.g. LLVM.
                imply!($($member)&+ => $group);
            };
        }

        group!(zkn == zbkb & zbkc & zbkx & zkne & zknd & zknh);
        group!(zks == zbkb & zbkc & zbkx & zksed & zksh);
        group!(zk == zkn & zkr & zkt);

        group!(a == zalrsc & zaamo);

        group!(b == zba & zbb & zbs);

        imply!(zhinx => zhinxmin);
        imply!(zdinx | zhinxmin => zfinx);

        imply!(zfh => zfhmin);
        imply!(q => d);
        imply!(d | zfhmin => f);

        imply!(zicntr | zihpm | zkr | f | zfinx => zicsr);
        imply!(s | h => zicsr);

        // Loop until the feature flags converge.
        if prev == value {
            return value;
        }
    }
}

/// Read list of supported features from the auxiliary vector.
pub(crate) fn detect_features() -> cache::Initializer {
    let mut value = cache::Initializer::default();
    let mut enable_feature = |feature, enable| {
        if enable {
            value.set(feature as u32);
        }
    };

    // Use auxiliary vector to enable single-letter ISA extensions.
    // The values are part of the platform-specific [asm/hwcap.h][hwcap]
    //
    // [hwcap]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/arch/riscv/include/uapi/asm/hwcap.h?h=v6.14
    let auxv = auxvec::auxv().expect("read auxvec"); // should not fail on RISC-V platform
    #[allow(clippy::eq_op)]
    enable_feature(Feature::a, bit::test(auxv.hwcap, (b'a' - b'a').into()));
    enable_feature(Feature::c, bit::test(auxv.hwcap, (b'c' - b'a').into()));
    enable_feature(Feature::d, bit::test(auxv.hwcap, (b'd' - b'a').into()));
    enable_feature(Feature::f, bit::test(auxv.hwcap, (b'f' - b'a').into()));
    enable_feature(Feature::h, bit::test(auxv.hwcap, (b'h' - b'a').into()));
    enable_feature(Feature::m, bit::test(auxv.hwcap, (b'm' - b'a').into()));

    // Handle base ISA.
    let has_i = bit::test(auxv.hwcap, (b'i' - b'a').into());
    // If future RV128I is supported, implement with `enable_feature` here.
    // Note that we should use `target_arch` instead of `target_pointer_width`
    // to avoid misdetection caused by experimental ABIs such as RV64ILP32.
    #[cfg(target_arch = "riscv64")]
    enable_feature(Feature::rv64i, has_i);
    #[cfg(target_arch = "riscv32")]
    enable_feature(Feature::rv32i, has_i);
    // FIXME: e is not exposed in any of asm/hwcap.h, uapi/asm/hwcap.h, uapi/asm/hwprobe.h
    #[cfg(target_arch = "riscv32")]
    enable_feature(Feature::rv32e, bit::test(auxv.hwcap, (b'e' - b'a').into()));

    // FIXME: Auxvec does not show supervisor feature support, but this mode may be useful
    // to detect when Rust is used to write Linux kernel modules.
    // These should be more than Auxvec way to detect supervisor features.

    imply_features(value)
}
