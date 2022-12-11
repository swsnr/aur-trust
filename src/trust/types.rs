// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and traits for representing and checking trust.

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
}
