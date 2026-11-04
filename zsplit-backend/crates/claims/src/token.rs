use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;
use zsplit_domain::DomainError;

pub const TOKEN_BYTES: usize = 32;
pub const TOKEN_ENCODED_LEN: usize = 43;
const DIGEST_CONTEXT: &[u8] = b"zsplit:claim-ticket:v1\0";

/// Raw tickets must only be returned in the issuance response, never persisted.
/// This type intentionally does not implement `Debug`, `Display`, or serialization.
pub struct IssuedToken {
    pub token: Zeroizing<String>,
    pub digest: [u8; 32],
}

/// HMAC key material is zeroized on drop. Use a separate key for receiver encryption.
pub struct TokenService {
    key: Zeroizing<[u8; 32]>,
}

impl TokenService {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key: Zeroizing::new(key),
        }
    }

    /// An OS entropy-source failure fails closed through `OsRng`; no weak fallback.
    pub fn issue(&self) -> IssuedToken {
        let mut bytes = Zeroizing::new([0u8; TOKEN_BYTES]);
        OsRng.fill_bytes(bytes.as_mut());
        IssuedToken {
            token: Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes.as_ref())),
            digest: self.digest_bytes(bytes.as_ref()),
        }
    }

    /// Only the canonical 256-bit URL-safe encoding is accepted.
    pub fn digest(&self, token: &str) -> Result<[u8; 32], DomainError> {
        if token.len() != TOKEN_ENCODED_LEN {
            return Err(DomainError::InvalidInput);
        }
        let decoded = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(token)
                .map_err(|_| DomainError::InvalidInput)?,
        );
        if decoded.len() != TOKEN_BYTES || URL_SAFE_NO_PAD.encode(decoded.as_slice()) != token {
            return Err(DomainError::InvalidInput);
        }
        Ok(self.digest_bytes(decoded.as_slice()))
    }

    pub fn verify(&self, token: &str, expected_digest: &[u8]) -> bool {
        match self.digest(token) {
            Ok(actual) => bool::from(actual.as_slice().ct_eq(expected_digest)),
            Err(_) => false,
        }
    }

    fn digest_bytes(&self, token: &[u8]) -> [u8; 32] {
        // HMAC accepts every key length, including the fixed 32-byte key here.
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.key.as_ref()).expect("HMAC accepts 32-byte keys");
        mac.update(DIGEST_CONTEXT);
        mac.update(token);
        mac.finalize().into_bytes().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn fifty_tickets_are_unique_canonical_and_verifiable() {
        let service = TokenService::new([19; 32]);
        let mut tokens = HashSet::new();
        let mut digests = HashSet::new();
        for _ in 0..50 {
            let ticket = service.issue();
            assert_eq!(ticket.token.len(), TOKEN_ENCODED_LEN);
            assert!(service.verify(&ticket.token, &ticket.digest));
            assert!(tokens.insert(ticket.token.to_string()));
            assert!(digests.insert(ticket.digest));
            assert_ne!(
                ticket.digest.as_slice(),
                URL_SAFE_NO_PAD.decode(ticket.token.as_str()).unwrap()
            );
        }
    }

    #[test]
    fn invalid_and_noncanonical_tokens_never_verify() {
        let service = TokenService::new([19; 32]);
        let ticket = service.issue();
        for bad in [
            "".to_owned(),
            "A".repeat(42),
            "A".repeat(44),
            "=".repeat(43),
            format!("{}=", ticket.token.as_str()),
            format!("{}B", "A".repeat(42)),
            "é".repeat(43),
        ] {
            assert!(service.digest(&bad).is_err());
            assert!(!service.verify(&bad, &ticket.digest));
        }
        assert!(!service.verify(&ticket.token, &[0; 31]));
        assert!(!service.verify(&ticket.token, &[0; 32]));
        assert!(!TokenService::new([20; 32]).verify(&ticket.token, &ticket.digest));
    }
}
