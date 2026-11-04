use std::path::PathBuf;
use zsplit_domain::{DomainError, Network};

/// Paths refer to protected local files; a seed is never accepted through an HTTP request.
#[derive(Clone)]
pub struct WalletConfig {
    pub data_dir: PathBuf,
    pub seed_file: PathBuf,
    pub lightwalletd_url: String,
    pub network: Network,
    /// First height to scan. None creates new accounts at the current chain tip.
    pub birthday_height: Option<u32>,
    pub confirmations: u32,
    pub sapling_spend_params: Option<PathBuf>,
    pub sapling_output_params: Option<PathBuf>,
}

impl WalletConfig {
    pub fn validate(&self) -> Result<(), DomainError> {
        // Regtest activation heights cannot safely be inferred from the testnet schedule.
        if self.network != Network::Testnet || self.confirmations == 0 {
            return Err(DomainError::InvalidInput);
        }
        let endpoint = tonic::transport::Endpoint::from_shared(self.lightwalletd_url.clone())
            .map_err(|_| DomainError::InvalidInput)?;
        let uri = endpoint.uri();
        if uri.host().is_none()
            || !matches!(uri.scheme_str(), Some("https" | "http"))
            || (uri.scheme_str() == Some("http")
                && !matches!(uri.host(), Some("127.0.0.1" | "localhost" | "[::1]")))
            || uri.authority().is_some_and(|a| a.as_str().contains('@'))
            || uri.query().is_some()
            || uri.path() != "/"
        {
            return Err(DomainError::InvalidInput);
        }
        if self.birthday_height == Some(0)
            || self.sapling_spend_params.is_some() != self.sapling_output_params.is_some()
        {
            return Err(DomainError::InvalidInput);
        }
        Ok(())
    }
}
