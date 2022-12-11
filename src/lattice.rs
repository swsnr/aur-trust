// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Type definitions and utilities for complete lattices.

/// A join semi lattice.
pub trait JoinSemiLattice {
    /// Compute the least upper bound of `self` and `other.
    fn join(self, other: Self) -> Self;
}

/// A meet semi lattice.
pub trait MeetSemiLattice {
    /// Compute the greatest lower bound of `self` and `other.
    fn meet(self, other: Self) -> Self;
}

/// A set which has a top element.
pub trait HasTop {
    /// The element which is greater or equal to all other elements.
    fn top() -> Self;
}

/// A set which has a bottom element.
pub trait HasBottom {
    /// The element which is less or equal to all other elements.
    fn bottom() -> Self;
}
