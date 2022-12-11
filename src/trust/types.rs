// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types for representing trust.

use crate::lattice::{HasBottom, HasTop, JoinSemiLattice, MeetSemiLattice};

/// Trust in an AUR package.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Trust {
    /// The package is not trusted.
    Untrusted = 0,
    /// Trust for the package is not fully determined yet.
    Indeterminate = 1,
    /// The package is trusted.
    Trusted = 2,
}

impl HasTop for Trust {
    /// [`Trust::Trusted`], as the top element of the [`Trust`] enum.
    fn top() -> Self {
        Trust::Trusted
    }
}

impl HasBottom for Trust {
    /// [`Trust::Untrusted`], as the bottom element of the [`Trust`] enum.
    fn bottom() -> Self {
        Trust::Untrusted
    }
}

impl Default for Trust {
    /// By default, trust is [`Trust::Indeterminate`].
    fn default() -> Self {
        Trust::Indeterminate
    }
}

impl JoinSemiLattice for Trust {
    fn join(self, other: Self) -> Self {
        self.max(other)
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
        Self { trust, reasons }
    }
}

impl JoinSemiLattice for TrustVerdict {
    /// Determine the upper bound of two trust verdicts.
    ///
    /// Retain all reasons for the lower bound, and discard other reasons.
    fn join(self, other: Self) -> Self {
        let trust = self.trust.join(other.trust);
        let mut reasons = Vec::with_capacity(self.reasons.len() + other.reasons.len());
        if self.trust == trust {
            reasons.extend(self.reasons.into_iter());
        }
        if other.trust == trust {
            reasons.extend(other.reasons.into_iter());
        }
        Self { trust, reasons }
    }
}

#[cfg(test)]
mod test {
    use crate::lattice::*;
    use crate::trust::Trust;
    use quickcheck::Gen;
    use quickcheck_macros::quickcheck;

    #[test]
    fn trust_default() {
        assert_eq!(Trust::default(), Trust::Indeterminate)
    }

    #[test]
    fn trust_top() {
        assert_eq!(Trust::top(), Trust::Trusted)
    }

    #[test]
    fn trust_bottom() {
        assert_eq!(Trust::bottom(), Trust::Untrusted)
    }

    #[test]
    fn trust_ord() {
        assert!(Trust::Trusted >= Trust::Trusted);
        assert!(Trust::Trusted > Trust::Indeterminate);
        assert!(Trust::Trusted > Trust::Untrusted);
        assert!(Trust::Indeterminate < Trust::Trusted);
        assert!(Trust::Indeterminate >= Trust::Indeterminate);
        assert!(Trust::Indeterminate > Trust::Untrusted);
        assert!(Trust::Untrusted < Trust::Trusted);
        assert!(Trust::Untrusted < Trust::Indeterminate);
        assert!(Trust::Untrusted >= Trust::Untrusted);
    }

    impl quickcheck::Arbitrary for Trust {
        fn arbitrary(g: &mut Gen) -> Self {
            g.choose(&[Trust::Trusted, Trust::Indeterminate, Trust::Untrusted])
                .unwrap()
                .to_owned()
        }
    }

    #[quickcheck]
    fn trust_join_gt(left: Trust, right: Trust) {
        let top = left.join(right);
        assert!(top >= left, "{:?} >= {:?}", top, left);
        assert!(top >= right, "{:?} >= {:?}", top, right);
    }

    #[quickcheck]
    fn trust_join_top(t: Trust) {
        assert_eq!(t.join(Trust::top()), Trust::Trusted);
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

    #[test]
    fn trust_verdict_join_gt() {
        todo!()
    }

    #[test]
    fn trust_verdict_meet_lt() {
        todo!()
    }
}
