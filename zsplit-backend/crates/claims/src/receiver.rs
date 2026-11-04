use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand::{RngCore, rngs::OsRng};
use uuid::Uuid;
use zcash_address::unified::{Address as EncodedUnifiedAddress, Encoding};
use zcash_keys::address::UnifiedAddress;
use zcash_protocol::consensus::NetworkType;
use zeroize::Zeroizing;
use zsplit_domain::{DomainError, Network};

const MAX_RECEIVER_BYTES: usize = 2048;
const VERSION: u8 = 1;
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const CIPHER_CONTEXT: &[u8] = b"zsplit:ephemeral-receiver:v1\0";

/// Validates the complete native address, including its Orchard point and network.
/// Accepting a unified address never implies a transparent fallback is allowed.
pub fn validate_receiver(receiver: &str, network: Network) -> Result<(), DomainError> {
    if receiver.is_empty() || receiver.len() > MAX_RECEIVER_BYTES {
        return Err(DomainError::InvalidInput);
    }
    let (actual_network, _, container) =
        EncodedUnifiedAddress::decode(receiver).map_err(|_| DomainError::InvalidInput)?;
    let expected_network = match network {
        Network::Mainnet => NetworkType::Main,
        Network::Testnet => NetworkType::Test,
        Network::Regtest => NetworkType::Regtest,
    };
    if actual_network != expected_network {
        return Err(DomainError::NetworkMismatch);
    }
    let address = UnifiedAddress::try_from(container).map_err(|_| DomainError::InvalidInput)?;
    // The reservation may wait in a queue; this API has no chain/time expiry context.
    // Never strip or silently override sender-visible address expiration policy.
    if address.expiry_height().is_some() || address.expiry_time().is_some() {
        return Err(DomainError::InvalidInput);
    }
    if address.orchard().is_none() {
        return Err(DomainError::InvalidInput);
    }
    Ok(())
}

/// AEAD protects pending receiver bytes; native sender history may retain receivers.
/// The versioned envelope is bound to its claim ID to prevent row substitution.
pub struct ReceiverCipher {
    key: Zeroizing<[u8; 32]>,
}

impl ReceiverCipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key: Zeroizing::new(key),
        }
    }

    pub fn encrypt(&self, claim_id: Uuid, receiver: &str) -> Result<Vec<u8>, DomainError> {
        if receiver.is_empty() || receiver.len() > MAX_RECEIVER_BYTES {
            return Err(DomainError::InvalidInput);
        }
        let mut nonce = [0u8; NONCE_BYTES];
        OsRng
            .try_fill_bytes(&mut nonce)
            .map_err(|_| DomainError::Unavailable)?;
        let aad = associated_data(claim_id);
        let encrypted = ChaCha20Poly1305::new(self.key.as_ref().into())
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: receiver.as_bytes(),
                    aad: &aad,
                },
            )
            .map_err(|_| DomainError::Unavailable)?;
        let mut envelope = Vec::with_capacity(1 + NONCE_BYTES + encrypted.len());
        envelope.push(VERSION);
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&encrypted);
        Ok(envelope)
    }

    pub fn decrypt(
        &self,
        claim_id: Uuid,
        envelope: &[u8],
    ) -> Result<Zeroizing<String>, DomainError> {
        if envelope.len() < 1 + NONCE_BYTES + TAG_BYTES + 1
            || envelope.len() > 1 + NONCE_BYTES + TAG_BYTES + MAX_RECEIVER_BYTES
            || envelope[0] != VERSION
        {
            return Err(DomainError::InvalidInput);
        }
        let aad = associated_data(claim_id);
        let plaintext = Zeroizing::new(
            ChaCha20Poly1305::new(self.key.as_ref().into())
                .decrypt(
                    Nonce::from_slice(&envelope[1..1 + NONCE_BYTES]),
                    Payload {
                        msg: &envelope[1 + NONCE_BYTES..],
                        aad: &aad,
                    },
                )
                .map_err(|_| DomainError::InvalidInput)?,
        );
        let receiver = std::str::from_utf8(&plaintext).map_err(|_| DomainError::InvalidInput)?;
        Ok(Zeroizing::new(receiver.to_owned()))
    }
}

fn associated_data(claim_id: Uuid) -> Vec<u8> {
    let mut aad = Vec::with_capacity(CIPHER_CONTEXT.len() + 16);
    aad.extend_from_slice(CIPHER_CONTEXT);
    aad.extend_from_slice(claim_id.as_bytes());
    aad
}

#[cfg(test)]
mod tests {
    use super::*;
    use zcash_keys::keys::{UnifiedAddressRequest, UnifiedSpendingKey};
    use zcash_protocol::consensus::TEST_NETWORK;

    fn orchard_address(network: NetworkType) -> String {
        let key = UnifiedSpendingKey::from_seed(&TEST_NETWORK, &[7; 32], 0u32.try_into().unwrap())
            .unwrap();
        let (ua, _) = key
            .to_unified_full_viewing_key()
            .default_address(UnifiedAddressRequest::ORCHARD)
            .unwrap();
        ua.to_zcash_address(network).to_string()
    }

    #[test]
    fn accepts_only_matching_network_orchard_addresses() {
        for (network, native) in [
            (Network::Mainnet, NetworkType::Main),
            (Network::Testnet, NetworkType::Test),
            (Network::Regtest, NetworkType::Regtest),
        ] {
            let address = orchard_address(native);
            assert!(validate_receiver(&address, network).is_ok());
            for other in [Network::Mainnet, Network::Testnet, Network::Regtest] {
                if network != other {
                    assert!(matches!(
                        validate_receiver(&address, other),
                        Err(DomainError::NetworkMismatch)
                    ));
                }
            }
        }
        for address in [
            "",
            "not-a-wallet",
            "tmEZhbWHTpdKMw5it8YDspUXSMGQyFwovpU",
            "zcash:anything?amount=1",
            "ztestsapling10yy2ex5dcqkclhc7z7yrnjq2z6feyjad56ptwlfgmy77dmaqqrl9gyhprdx59qgmsnyfska2kez",
        ] {
            assert!(validate_receiver(address, Network::Testnet).is_err());
        }
        assert!(validate_receiver(&"a".repeat(MAX_RECEIVER_BYTES + 1), Network::Testnet).is_err());
    }

    #[test]
    fn rejects_expiring_receivers_instead_of_stripping_policy() {
        let encoded = orchard_address(NetworkType::Test);
        let (_, _, container) = EncodedUnifiedAddress::decode(&encoded).unwrap();
        let address = UnifiedAddress::try_from(container).unwrap();
        for (height, time) in [
            (
                Some(zcash_protocol::consensus::BlockHeight::from_u32(1)),
                None,
            ),
            (None, Some(1)),
        ] {
            let expiring = UnifiedAddress::from_receivers(
                address.orchard().copied(),
                None,
                None,
                height,
                time,
            )
            .unwrap()
            .to_zcash_address(NetworkType::Test)
            .to_string();
            assert!(validate_receiver(&expiring, Network::Testnet).is_err());
        }
    }

    #[test]
    fn rejects_an_encoded_but_invalid_orchard_point() {
        use zcash_address::unified::{Receiver, Revision, Uitem};
        let encoded = EncodedUnifiedAddress::try_from_items(
            Revision::R0,
            vec![Uitem::Data(Receiver::Orchard([0xff; 43]))],
        )
        .unwrap()
        .encode(&NetworkType::Test);
        assert!(matches!(
            validate_receiver(&encoded, Network::Testnet),
            Err(DomainError::InvalidInput)
        ));
    }

    #[test]
    fn rejects_a_unified_address_without_orchard() {
        let key = UnifiedSpendingKey::from_seed(&TEST_NETWORK, &[8; 32], 0u32.try_into().unwrap())
            .unwrap();
        use zcash_keys::keys::ReceiverRequirement::{Omit, Require};
        let request = UnifiedAddressRequest::custom(Omit, Require, Omit).unwrap();
        let (ua, _) = key
            .to_unified_full_viewing_key()
            .default_address(request)
            .unwrap();
        assert!(
            validate_receiver(
                &ua.to_zcash_address(NetworkType::Test).to_string(),
                Network::Testnet
            )
            .is_err()
        );
    }

    #[test]
    fn encryption_is_randomized_and_bound_to_claim_and_key() {
        let cipher = ReceiverCipher::new([3; 32]);
        let claim = Uuid::new_v4();
        let address = orchard_address(NetworkType::Test);
        let envelope = cipher.encrypt(claim, &address).unwrap();
        assert_ne!(envelope, cipher.encrypt(claim, &address).unwrap());
        assert!(
            !envelope
                .windows(address.len())
                .any(|w| w == address.as_bytes())
        );
        assert_eq!(cipher.decrypt(claim, &envelope).unwrap().as_str(), address);
        assert!(cipher.decrypt(Uuid::new_v4(), &envelope).is_err());
        assert!(
            ReceiverCipher::new([4; 32])
                .decrypt(claim, &envelope)
                .is_err()
        );
        for index in [0, 1, envelope.len() - 1] {
            let mut damaged = envelope.clone();
            damaged[index] ^= 1;
            assert!(cipher.decrypt(claim, &damaged).is_err());
        }
        for truncated in [&[][..], &envelope[..12], &envelope[..envelope.len() - 1]] {
            assert!(cipher.decrypt(claim, truncated).is_err());
        }
    }
}
