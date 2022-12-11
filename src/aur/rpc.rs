// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use serde::Deserialize;
use thiserror::Error;
use tracing::{event, instrument, Level};

/// The user agent to use for RPC requests to the AUR.
static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
/// The letsencrypt root certificate.
///
/// The AUR RPC endpoint uses letsencrypt certificates, so this is the only relevant root
/// certificate we require for AUR RPC requests.
///
/// Embedding this certificate makes aur-trust self-contained and independent from system trust
/// stores, and increases security because we avoid all other shady CAs that might be installed
/// somewhere.
static LETSENCRYPT_ROOT: &[u8] = include_bytes!("isrgrootx1.der");

fn letsencrypt_root() -> reqwest::tls::Certificate {
    reqwest::tls::Certificate::from_der(LETSENCRYPT_ROOT).unwrap()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AurPackage {
    /// The package name
    pub name: String,
    /// The main maintainer of the package.
    pub maintainer: String,
    /// All registered co-maintainers of the package.
    #[serde(default)]
    pub co_maintainers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AurInfo {
    /// The number of results returned by AUR.
    resultcount: usize,
    /// The results.
    results: Vec<AurPackage>,
}

#[derive(Error, Debug)]
pub enum AurError {
    /// Reqwest returned an error.
    #[error("reqwest failed")]
    ReqwestError(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, AurError>;

#[derive(Debug, Clone)]
pub struct AurClient {
    client: reqwest::Client,
}

impl AurClient {
    /// Create a new default AUR client.
    ///
    /// This client uses a user agent which identifies aur-trust and its version number, and a
    /// custom TLS configuration which uses only the letsencrypt root certificate required to make
    /// secure AUR RPC connections.
    pub fn new() -> Result<Self> {
        reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .referer(false)
            .use_rustls_tls()
            // Only use letsencrypt root certificate, because that's what AUR uses
            .tls_built_in_root_certs(false)
            .add_root_certificate(letsencrypt_root())
            .min_tls_version(reqwest::tls::Version::TLS_1_3)
            .build()
            .map(Self::from_client)
            .map_err(From::from)
    }

    /// Create an AUR RPC client around the given [`reqwest::Client`].
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// The AUR RPC base URL, for version 5.
    fn base_url(&self) -> reqwest::Url {
        // TODO: Find a way to make this static?
        reqwest::Url::parse("https://aur.archlinux.org/rpc/?v=5")
            .expect("Base URL should definitely be valid!")
    }

    /// Get information about the given `packages`.
    #[instrument(skip_all)]
    pub async fn info<I, S>(&self, packages: I) -> Result<Vec<AurPackage>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut url = self.base_url();
        url.query_pairs_mut().append_pair("type", "info");
        for package in packages {
            url.query_pairs_mut().append_pair("arg[]", package.as_ref());
        }
        event!(Level::DEBUG, "GET {}", &url);
        let info: AurInfo = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if info.resultcount != info.results.len() {
            event!(
                Level::WARN,
                "Inconsistent AUR info response: resultcount {} != results.len {}",
                info.resultcount,
                info.results.len()
            );
        }
        Ok(info.results)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::{assert_eq, assert_str_eq};

    #[tokio::test]
    async fn single_get_single_maintainer() {
        let results = AurClient::new()
            .unwrap()
            .info(&["1password"])
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_str_eq!(results[0].name, "1password");
        assert_str_eq!(results[0].maintainer, "1Password");
        assert!(
            results[0].co_maintainers.is_empty(),
            "Maintainers: {:?}",
            results[0].co_maintainers
        );
    }

    #[tokio::test]
    async fn single_get_with_comaintainers() {
        let results = AurClient::new().unwrap().info(&["aurutils"]).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_str_eq!(results[0].name, "aurutils");
        assert_str_eq!(results[0].maintainer, "Alad");
        assert_eq!(
            results[0].co_maintainers,
            vec!["cgirard", "maximbaz", "rafasc"]
        );
    }

    #[tokio::test]
    async fn multiget() {
        let results = AurClient::new()
            .unwrap()
            .info(&["1password", "dracut-hook-uefi"])
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_str_eq!(results[0].name, "1password");
        assert_str_eq!(results[0].maintainer, "1Password");
        assert_str_eq!(results[1].name, "dracut-hook-uefi");
        assert_str_eq!(results[1].maintainer, "swsnr");
    }
}
