use frost_ed25519 as frost;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrostError {
    #[error("FROST cryptographic error: {0}")]
    Crypto(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid threshold configuration: {0}")]
    InvalidThreshold(String),
}

/// A serialized 2-of-3 FROST setup for group treasury authorization.
#[derive(Clone, Serialize, Deserialize)]
pub struct ThresholdTreasurySetup {
    pub min_signers: u16,
    pub max_signers: u16,
    pub group_public_key: String,
    pub public_key_package_json: String,
    pub key_packages_json: Vec<String>,
}

/// Generate a (t, n) FROST threshold key setup (e.g. 2-of-3).
pub fn setup_threshold_treasury(
    min_signers: u16,
    max_signers: u16,
) -> Result<ThresholdTreasurySetup, FrostError> {
    if min_signers < 2 || min_signers > max_signers {
        return Err(FrostError::InvalidThreshold(
            "min_signers must be at least 2 and <= max_signers".into(),
        ));
    }

    let rng = OsRng;
    let (shares, public_key_package) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        rng,
    )
    .map_err(|e| FrostError::Crypto(e.to_string()))?;

    let group_pk_bytes = public_key_package
        .verifying_key()
        .serialize()
        .map_err(|e| FrostError::Crypto(e.to_string()))?;
    let group_public_key = hex::encode(group_pk_bytes);

    let mut key_packages_json = Vec::new();
    for (_, secret_share) in shares {
        let key_package = frost::keys::KeyPackage::try_from(secret_share)
            .map_err(|e| FrostError::Crypto(e.to_string()))?;
        key_packages_json.push(
            serde_json::to_string(&key_package)
                .map_err(|e| FrostError::Serialization(e.to_string()))?,
        );
    }

    let public_key_package_json = serde_json::to_string(&public_key_package)
        .map_err(|e| FrostError::Serialization(e.to_string()))?;

    Ok(ThresholdTreasurySetup {
        min_signers,
        max_signers,
        group_public_key,
        public_key_package_json,
        key_packages_json,
    })
}

/// Verify a group Schnorr signature generated via 2-of-3 FROST against the group public key.
pub fn verify_frost_authorization(
    group_public_key_hex: &str,
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<bool, FrostError> {
    let pk_bytes: [u8; 32] = hex::decode(group_public_key_hex)
        .map_err(|e| FrostError::Serialization(e.to_string()))?
        .try_into()
        .map_err(|_| FrostError::Serialization("Invalid group public key length".into()))?;

    let verifying_key = frost::VerifyingKey::deserialize(&pk_bytes)
        .map_err(|e| FrostError::Crypto(e.to_string()))?;

    let signature = frost::Signature::deserialize(signature_bytes)
        .map_err(|e| FrostError::Crypto(e.to_string()))?;

    Ok(verifying_key.verify(message, &signature).is_ok())
}

/// Simulate or execute a 2-of-3 FROST threshold signature for batch authorization.
pub fn generate_threshold_signature(
    key_packages_json: &[String],
    public_key_package_json: &str,
    message: &[u8],
) -> Result<Vec<u8>, FrostError> {
    let public_key_package: frost::keys::PublicKeyPackage =
        serde_json::from_str(public_key_package_json)
            .map_err(|e| FrostError::Serialization(e.to_string()))?;

    let mut rng = OsRng;
    let mut nonces_map = BTreeMap::new();
    let mut commitments_map = BTreeMap::new();
    let mut key_packages = Vec::new();

    // Round 1: Each participant generates nonces and commitments
    for pkg_json in key_packages_json {
        let key_package: frost::keys::KeyPackage =
            serde_json::from_str(pkg_json).map_err(|e| FrostError::Serialization(e.to_string()))?;
        let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), &mut rng);
        let id = *key_package.identifier();
        nonces_map.insert(id, nonces);
        commitments_map.insert(id, commitments);
        key_packages.push(key_package);
    }

    // Round 2: Generate signing package and collect signature shares
    let signing_package = frost::SigningPackage::new(commitments_map, message);
    let mut signature_shares = BTreeMap::new();

    for key_package in &key_packages {
        let id = *key_package.identifier();
        let nonces = nonces_map.get(&id).unwrap();
        let share = frost::round2::sign(&signing_package, nonces, key_package)
            .map_err(|e| FrostError::Crypto(e.to_string()))?;
        signature_shares.insert(id, share);
    }

    // Aggregation: Group coordinator combines shares into a single Schnorr signature
    let group_signature =
        frost::aggregate(&signing_package, &signature_shares, &public_key_package)
            .map_err(|e| FrostError::Crypto(e.to_string()))?;

    let bytes = group_signature
        .serialize()
        .map_err(|e| FrostError::Crypto(e.to_string()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frost_2_of_3_threshold_lifecycle() {
        // 1. Setup 2-of-3 threshold treasury
        let setup = setup_threshold_treasury(2, 3).expect("Setup 2-of-3 FROST treasury");
        assert_eq!(setup.min_signers, 2);
        assert_eq!(setup.max_signers, 3);
        assert_eq!(setup.key_packages_json.len(), 3);

        // 2. Select 2 out of 3 signers (e.g. signers 0 and 2)
        let active_signers = vec![
            setup.key_packages_json[0].clone(),
            setup.key_packages_json[2].clone(),
        ];
        let message = b"zsplit:authorize-batch-release:room-1234:50-tickets";

        // 3. Generate threshold signature
        let signature =
            generate_threshold_signature(&active_signers, &setup.public_key_package_json, message)
                .expect("Aggregate FROST threshold signature");

        // 4. Verify the threshold signature against the group public key
        let valid = verify_frost_authorization(&setup.group_public_key, message, &signature)
            .expect("Verify FROST signature");
        assert!(valid, "FROST 2-of-3 threshold signature must verify");

        // 5. Tampered message fails verification
        let invalid =
            verify_frost_authorization(&setup.group_public_key, b"tampered message", &signature)
                .unwrap_or(false);
        assert!(!invalid, "Tampered message must not verify");
    }
}
