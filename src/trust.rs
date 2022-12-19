// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Determine whether AUR packages are trusted.

mod types;

use crate::lattice::MeetSemiLattice;
use std::collections::HashSet;
pub use types::{Trust, TrustVerdict};

/// The validity of a Git signature, according to git.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SignatureValidity {
    /// The signature was good and valid.
    Good,
    /// The signature was bad, i.e. an invalid format.
    Bad,
    /// The signature was good, but its validity is unknown.
    UnknownValidity,
    /// The signature is expired.
    ExpiredSignature,
    /// The signature is good, but the signing key is expired.
    ExpiredKey,
    /// The signing key was revoked.
    RevokedKey,
}

/// The state of a signature on a commit.
#[derive(Clone, Eq, PartialEq)]
pub struct CommitSignature {
    /// The validity of the signature.
    pub validity: SignatureValidity,
    /// The name of the signer of the commit.
    pub signer: String,
    /// The key that was used to sign the commit.
    pub key: String,
}

/// The database of trusted entities.
#[derive(Clone, Eq, PartialEq)]
pub struct TrustDatabase {
    /// A set of trusted maintainers.
    trusted_maintainers: HashSet<String>,
}

impl TrustDatabase {
    /// Set trusted maintainers.
    pub fn set_trusted_maintainers(mut self, maintainers: HashSet<String>) -> Self {
        self.trusted_maintainers = maintainers;
        self
    }

    /// Add a single maintainer as trusted maintainer.
    pub fn trust_maintainer(mut self, maintainer: String) -> Self {
        self.trusted_maintainers.insert(maintainer);
        self
    }
}

impl Default for TrustDatabase {
    fn default() -> Self {
        Self {
            trusted_maintainers: HashSet::new(),
        }
    }
}

/// A git commit.
#[derive(Clone, Eq, PartialEq)]
pub struct GitCommit {
    /// The abbreviated hash of the commit.
    abbrev_sha1: String,
    /// The signature on the commit, if any.
    signature: Option<CommitSignature>,
}

/// A package with associated evidence for checking trust.
#[derive(Clone, Eq, PartialEq)]
pub struct PackageWithEvidence {
    /// The name of the package
    name: String,
    /// The set of maintainers of this package.
    maintainers: HashSet<String>,
    /// The head commit of the package repo.
    head_commit: GitCommit,
}

/// Check a signature on the HEAD `commit` of a package.
///
/// If the commit has no signature, return an indeterminate verdict.
///
/// If the commit has a signature return a trusted verdict if and only if the signature is good and
/// valid, ie, if the key is trusted.  Otherwise return an untrusted verdict.
pub fn check_commit_signature(commit: &GitCommit) -> TrustVerdict {
    commit.signature.as_ref().map_or_else(
        || {
            TrustVerdict::default().add_reason(format!(
                "HEAD commit {} has no signature",
                &commit.abbrev_sha1
            ))
        },
        |signature| match signature.validity {
            SignatureValidity::Good => TrustVerdict::trusted().add_reason(format!(
                "HEAD commit {} signed by {} with key {}",
                &commit.abbrev_sha1, signature.signer, signature.key
            )),
            SignatureValidity::Bad => TrustVerdict::untrusted().add_reason(format!(
                "HEAD commit {} had bad signature",
                &commit.abbrev_sha1
            )),
            SignatureValidity::UnknownValidity => TrustVerdict::untrusted().add_reason(format!(
                "Validity of signature of {} with key {} on HEAD commit {} is not known",
                signature.signer, signature.key, &commit.abbrev_sha1,
            )),
            SignatureValidity::ExpiredSignature => TrustVerdict::untrusted().add_reason(format!(
                "Signature of {} with key {} on HEAD commit {} is expired",
                signature.signer, signature.key, &commit.abbrev_sha1,
            )),
            SignatureValidity::ExpiredKey => TrustVerdict::untrusted().add_reason(format!(
                "Signature of {} on HEAD commit {} was made with expired key {}",
                signature.signer, &commit.abbrev_sha1, signature.key
            )),
            SignatureValidity::RevokedKey => TrustVerdict::untrusted().add_reason(format!(
                "Signature of {} on HEAD commit {} was made with revoked key {}",
                signature.signer, &commit.abbrev_sha1, signature.key
            )),
        },
    )
}

/// Check whether maintainers are trusted.
///
/// Return a trusted verdict if and only if all maintainers of `package` are contained in the set of
/// trusted maintainers in `trustdb`.
///
/// Otherwise return an indeterminate verdict.  In particular, return an indeterminate verdict if
/// `package` has no maintainers, i.e. if `package` is orphaned or if fetching its maintainers
/// failed, or if one or more maintainers are not trusted.  In the latter case deliberately return
/// an indeterminate verdict because even though maintainers may not be explicitly trusted, the
/// package per se could still have a trusted HEAD signature which is sufficient to mark the package
/// as trusted.
pub fn check_maintainers(trustdb: &TrustDatabase, maintainers: &HashSet<String>) -> TrustVerdict {
    if maintainers.is_empty() {
        TrustVerdict::default().add_reason("Maintainers unknown".to_owned())
    } else {
        if maintainers.is_subset(&trustdb.trusted_maintainers) {
            TrustVerdict::default()
                .set_trust(Trust::Trusted)
                .add_reason("All maintainers trusted".to_owned())
        } else {
            maintainers.difference(&trustdb.trusted_maintainers).fold(
                TrustVerdict::default(),
                |verdict, maintainer| {
                    verdict.add_reason(format!("Maintainer {} is not trusted", maintainer))
                },
            )
        }
    }
}

/// Obtain the combined verdict out of all given verdicts.
///
/// If the lower bound of all `verdicts` is untrusted return the lower bound, because an explicitly
/// untrusted verdict marks a package as untrusted.
///
/// Otherwise return the upper bound, i.e. indeterminate or trusted depending on the verdicts.
pub fn combined_verdict<I>(verdicts: I) -> TrustVerdict
where
    I: IntoIterator<Item = TrustVerdict>,
{
    verdicts
        .into_iter()
        .fold(TrustVerdict::default(), |l, r| l.meet(r))
}

/// Check the trust in the given `package`.
///
/// Check the trust in the HEAD commit signature and the trust in the registered maintainers.
/// If either is untrusted return an untrusted verdict with corresponding reasons, otherwise return
/// the upper bound of both verdicts with corresponding reasons.
pub fn check_trust(trustdb: &TrustDatabase, package: &PackageWithEvidence) -> TrustVerdict {
    let commit_verdict = check_commit_signature(&package.head_commit);
    let maintainer_verdict = check_maintainers(trustdb, &package.maintainers);

    combined_verdict(vec![commit_verdict, maintainer_verdict])
}

#[cfg(test)]
mod test {
    mod check_maintainers {
        use crate::trust::*;
        use pretty_assertions::assert_eq;
        use std::collections::HashSet;

        #[test]
        fn empty_maintainers() {
            let db = TrustDatabase::default().trust_maintainer("foo".to_owned());
            let verdict = TrustVerdict::default().add_reason("Maintainers unknown".to_owned());
            assert_eq!(check_maintainers(&db, &HashSet::new()), verdict);
        }

        #[test]
        fn some_maintainer_not_trusted() {
            let db = TrustDatabase::default().trust_maintainer("foo".to_owned());
            let maintainers = HashSet::from_iter(["foo".to_owned(), "bar".to_owned()]);
            let verdict =
                TrustVerdict::default().add_reason("Maintainer bar is not trusted".to_owned());
            assert_eq!(check_maintainers(&db, &maintainers), verdict);
        }

        #[test]
        fn all_maintainers_trusted() {
            let db = TrustDatabase::default()
                .trust_maintainer("foo".to_owned())
                .trust_maintainer("bar".to_owned())
                .trust_maintainer("spam".to_owned());
            let maintainers = HashSet::from_iter(["foo".to_owned(), "bar".to_owned()]);
            let verdict = TrustVerdict::default()
                .set_trust(Trust::Trusted)
                .add_reason("All maintainers trusted".to_owned());
            assert_eq!(check_maintainers(&db, &maintainers), verdict);
        }
    }

    mod check_head_signature {
        use crate::trust::*;
        use pretty_assertions::assert_eq;
        use quickcheck::{Arbitrary, Gen};
        use quickcheck_macros::quickcheck;

        impl Arbitrary for SignatureValidity {
            fn arbitrary(g: &mut Gen) -> Self {
                g.choose(&[
                    SignatureValidity::Good,
                    SignatureValidity::Bad,
                    SignatureValidity::UnknownValidity,
                    SignatureValidity::ExpiredSignature,
                    SignatureValidity::ExpiredKey,
                    SignatureValidity::RevokedKey,
                ])
                .unwrap()
                .to_owned()
            }
        }

        #[quickcheck]
        fn validity_trusted(validity: SignatureValidity) {
            let commit = GitCommit {
                abbrev_sha1: "d62e888".to_owned(),
                signature: Some(CommitSignature {
                    validity,
                    signer: "Jane Doe <j.doe@example.com>".to_string(),
                    key: "SHA256:xBUrqiiYS+mY5fCndm8Ye+SDU3Gr578hRbUL7ZzHbiY".to_string(),
                }),
            };
            let verdict = check_commit_signature(&commit);
            match validity {
                SignatureValidity::Good => {
                    assert_eq!(verdict.trust(), Trust::Trusted);
                    assert_eq!(verdict.reasons(), &["HEAD commit d62e888 signed by Jane Doe <j.doe@example.com> with key SHA256:xBUrqiiYS+mY5fCndm8Ye+SDU3Gr578hRbUL7ZzHbiY".to_string()])
                }
                _ => {
                    assert_eq!(verdict.trust(), Trust::Untrusted);
                }
            }
        }

        #[test]
        fn unsigned_indeterminate() {
            let commit = GitCommit {
                abbrev_sha1: "d62e888".to_owned(),
                signature: None,
            };
            let verdict = check_commit_signature(&commit);
            assert_eq!(verdict.trust(), Trust::Indeterminate);
            assert_eq!(
                verdict.reasons(),
                vec!["HEAD commit d62e888 has no signature".to_owned()]
            );
        }
    }
}
