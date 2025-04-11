//! Run-time feature detection utility for RISC-V.
//!
//! On RISC-V, full feature detection needs a help of one or more
//! feature detection mechanisms (usually provided by the operating system).
//!
//! RISC-V architecture defines many extensions and some have dependency to others.
//! More importantly, some of them cannot be enabled without resolving such
//! dependencies due to limited set of features that such mechanisms provide.
//!
//! This module provides an OS-independent utility to process such relations
//! between RISC-V extensions.

use crate::detect::{Feature, cache};

/// Imply features by the given set of enabled features.
///
/// Note that it does not perform any consistency checks including existence of
/// conflicting extensions and/or complicated requirements.  Eliminating such
/// inconsistencies is the responsibility of the feature detection logic and
/// its provider(s).
pub(crate) fn imply_features(mut value: cache::Initializer) -> cache::Initializer {
    loop {
        // Check convergence of the feature flags later.
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
