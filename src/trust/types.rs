// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types for representing trust.

use crate::lattice::{HasBottom, MeetSemiLattice};

/// Trust in an AUR package.
///
/// The enum variants form a partial order so that the lower bound of two trust values indicates an
/// overall trust value.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Trust {
    /// The package is not trusted.
    Untrusted = 0,
    /// The package is trusted.
    Trusted = 1,
    /// Trust for the package is not fully determined yet.
    Indeterminate = 2,
}

impl HasBottom for Trust {
    /// Untrusted, i.e. the package is definitely not trusted.
    fn bottom() -> Self {
        Trust::Untrusted
    }
}

impl Default for Trust {
    fn default() -> Self {
        Self::Indeterminate
    }
}

impl MeetSemiLattice for Trust {
    fn meet(self, other: Self) -> Self {
        self.min(other)
    }
}

/// The verdict whether a package is trusted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustVerdict {
    trust: Trust,
    reasons: Vec<String>,
}

impl TrustVerdict {
    /// A trusted verdict without any reason.
    pub fn trusted() -> TrustVerdict {
        TrustVerdict::default().set_trust(Trust::Trusted)
    }

    /// An untrusted verdict without any reason.
    pub fn untrusted() -> TrustVerdict {
        TrustVerdict::default().set_trust(Trust::Untrusted)
    }

    /// Whether the package is trusted.
    pub fn trust(&self) -> Trust {
        self.trust
    }

    /// The reasons for this verdict.
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }

    /// Set the trust verdict.
    pub fn set_trust(self, trust: Trust) -> Self {
        Self { trust, ..self }
    }

    /// Add a reason to this verdict.
    pub fn add_reason(mut self, reason: String) -> Self {
        self.reasons.push(reason);
        self
    }
}

impl Default for TrustVerdict {
    /// The default verdict: Trust is still indeterminate, and there are no special reasons.
    fn default() -> Self {
        Self {
            trust: Trust::Indeterminate,
            reasons: Vec::new(),
        }
    }
}

impl MeetSemiLattice for TrustVerdict {
    /// Determine the lower bound of two trust verdicts.
    ///
    /// Retain all reasons for the lower bound, and discard other reasons.
    fn meet(self, other: Self) -> Self {
        let trust = self.trust.meet(other.trust);
        let mut reasons = Vec::with_capacity(self.reasons.len() + other.reasons.len());
        if self.trust == trust {
            reasons.extend(self.reasons.into_iter());
        }
        if other.trust == trust {
            reasons.extend(other.reasons.into_iter());
        }
        // Sort to establish commutativity
        reasons.sort();
        Self { trust, reasons }
    }
}

#[cfg(test)]
mod test {
    use crate::lattice::*;
    use crate::trust::{Trust, TrustVerdict};
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;

    #[test]
    fn trust_default() {
        assert_eq!(Trust::default(), Trust::Indeterminate)
    }

    #[test]
    fn trust_bottom() {
        assert_eq!(Trust::bottom(), Trust::Untrusted)
    }

    #[test]
    fn trust_ord() {
        assert!(Trust::Indeterminate >= Trust::Indeterminate);
        assert!(Trust::Indeterminate > Trust::Trusted);
        assert!(Trust::Indeterminate > Trust::Untrusted);
        assert!(Trust::Trusted < Trust::Indeterminate);
        assert!(Trust::Trusted >= Trust::Trusted);
        assert!(Trust::Trusted > Trust::Untrusted);
        assert!(Trust::Untrusted < Trust::Trusted);
        assert!(Trust::Untrusted < Trust::Indeterminate);
        assert!(Trust::Untrusted >= Trust::Untrusted);
    }

    impl Arbitrary for Trust {
        fn arbitrary(g: &mut Gen) -> Self {
            g.choose(&[Trust::Trusted, Trust::Indeterminate, Trust::Untrusted])
                .unwrap()
                .to_owned()
        }
    }

    impl Arbitrary for TrustVerdict {
        fn arbitrary(g: &mut Gen) -> Self {
            let trust = Trust::arbitrary(g);
            let reasons = Arbitrary::arbitrary(g);
            TrustVerdict { trust, reasons }
        }
    }

    #[quickcheck]
    fn trust_meet_commutative(left: Trust, right: Trust) {
        assert_eq!(left.meet(right), right.meet(left))
    }

    #[quickcheck]
    fn trust_meet_bt(left: Trust, right: Trust) {
        let bottom = left.meet(right);
        assert!(bottom <= left, "{:?} <= {:?}", bottom, left);
        assert!(bottom <= right, "{:?} <= {:?}", bottom, right);
    }

    #[quickcheck]
    fn trust_meet_bottom(t: Trust) {
        assert_eq!(t.meet(Trust::bottom()), Trust::Untrusted);
    }

    #[quickcheck]
    fn trust_verdict_meet_commutative(l: TrustVerdict, r: TrustVerdict) {
        assert_eq!(l.clone().meet(r.clone()), r.meet(l));
    }

    #[quickcheck]
    fn trust_verdict_meet_lt(l: TrustVerdict, r: TrustVerdict) {
        let lower = l.clone().meet(r.clone());
        assert_eq!(lower.trust, l.trust.meet(r.trust));
        if l.trust == lower.trust {
            for reason in l.reasons {
                assert!(
                    lower.reasons.contains(&reason),
                    "{} in {:?}",
                    &reason,
                    &lower.trust
                );
            }
        }
        if r.trust == lower.trust {
            for reason in r.reasons {
                assert!(
                    lower.reasons.contains(&reason),
                    "{} in {:?}",
                    &reason,
                    &lower.trust
                );
            }
        }
    }
}
