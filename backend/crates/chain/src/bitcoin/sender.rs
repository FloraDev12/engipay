//! Bitcoin transaction building and broadcasting for withdrawals.
//!
//! # Design Notes
//!
//! This is a foundational implementation that establishes the core structure
//! for Bitcoin withdrawals. Full implementation requires agreement on:
//! - Coin selection strategy (FIFO, largest-first, or BDK's default)
//! - Change address handling and key derivation
//! - Fee estimation policy (fixed rate vs dynamic from mempool)
//! - Confirmation tracking threshold (currently: 2 confirmations)
//!
//! # Safety
//!
//! The local signer (for testing) refuses mainnet transactions to prevent
//! accidental loss of real funds during development.

use std::fmt;

use engipay_core::{Asset, Money};
use thiserror::Error;

use super::Txid;

/// Error type for Bitcoin transaction operations.
#[derive(Debug, Error)]
pub enum BitcoinSenderError {
    #[error("mainnet operations are not supported by the local test signer")]
    MainnetRefused,
    
    #[error("insufficient funds: need {needed} sats, have {available} sats")]
    InsufficientFunds { needed: u64, available: u64 },
    
    #[error("invalid destination address: {0}")]
    InvalidAddress(String),
    
    #[error("transaction building failed: {0}")]
    BuildFailed(String),
    
    #[error("broadcast failed: {0}")]
    BroadcastFailed(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
}

/// A Bitcoin withdrawal request from the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalRequest {
    /// Destination address (P2PKH, P2WPKH, P2SH, or bech32).
    pub destination: String,
    /// Amount to send in satoshis.
    pub amount_sats: u64,
    /// Optional memo for record-keeping (not included in transaction).
    pub memo: Option<String>,
}

impl WithdrawalRequest {
    /// Creates a new withdrawal request.
    pub fn new(destination: String, amount: Money) -> Result<Self, BitcoinSenderError> {
        if amount.asset != Asset::Btc {
            return Err(BitcoinSenderError::BuildFailed(
                "withdrawal asset must be BTC".to_owned(),
            ));
        }
        
        // Money stores BTC in satoshis (minor units).
        let amount_sats = u64::try_from(amount.minor)
            .map_err(|_| BitcoinSenderError::BuildFailed("amount out of range".to_owned()))?;
        
        Ok(Self {
            destination,
            amount_sats,
            memo: None,
        })
    }
    
    /// Attaches a memo to the withdrawal request.
    pub fn with_memo(mut self, memo: String) -> Self {
        self.memo = Some(memo);
        self
    }
}

/// A prepared (signed) Bitcoin transaction ready for broadcast.
#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    /// The transaction hash (TXID).
    pub txid: Txid,
    /// Raw transaction bytes (serialized, signed).
    pub raw_tx: Vec<u8>,
    /// Fee paid in satoshis.
    pub fee_sats: u64,
}

/// Bitcoin network type for signer validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Regtest,
}

/// Trait for Bitcoin transaction signing.
///
/// Implementations must refuse mainnet operations when using test keys.
pub trait BitcoinSigner: Send + Sync {
    /// Returns the network this signer is configured for.
    fn network(&self) -> BitcoinNetwork;
    
    /// Signs a withdrawal transaction.
    ///
    /// # Errors
    ///
    /// Returns [`BitcoinSenderError::MainnetRefused`] if the signer is a test
    /// signer and `network()` returns [`BitcoinNetwork::Mainnet`].
    fn sign_withdrawal(
        &self,
        request: &WithdrawalRequest,
    ) -> impl std::future::Future<Output = Result<PreparedTransaction, BitcoinSenderError>> + Send;
}

/// Local test signer that refuses mainnet.
///
/// This is a placeholder implementation for development. Production signers
/// should use BDK or a similar library with proper key management.
#[derive(Debug)]
pub struct LocalTestSigner {
    network: BitcoinNetwork,
}

impl LocalTestSigner {
    /// Creates a new test signer for the given network.
    ///
    /// # Safety
    ///
    /// Only accepts testnet or regtest. Mainnet is rejected at construction
    /// to prevent accidental misconfiguration.
    pub fn new(network: BitcoinNetwork) -> Result<Self, BitcoinSenderError> {
        if network == BitcoinNetwork::Mainnet {
            return Err(BitcoinSenderError::MainnetRefused);
        }
        Ok(Self { network })
    }
}

impl BitcoinSigner for LocalTestSigner {
    fn network(&self) -> BitcoinNetwork {
        self.network
    }
    
    async fn sign_withdrawal(
        &self,
        request: &WithdrawalRequest,
    ) -> Result<PreparedTransaction, BitcoinSenderError> {
        // Placeholder: Real implementation would:
        // 1. Select UTXOs (coin selection)
        // 2. Build transaction with change output
        // 3. Sign inputs
        // 4. Return serialized transaction
        
        if self.network == BitcoinNetwork::Mainnet {
            return Err(BitcoinSenderError::MainnetRefused);
        }
        
        // Stub response for development
        Err(BitcoinSenderError::BuildFailed(
            "Bitcoin sender not fully implemented; needs coin selection and signing".to_owned(),
        ))
    }
}

/// Bitcoin withdrawal sender for broadcasting and tracking transactions.
pub struct BitcoinWithdrawalSender<S: BitcoinSigner> {
    signer: S,
    _network: BitcoinNetwork,
}

impl<S: BitcoinSigner> BitcoinWithdrawalSender<S> {
    /// Creates a new Bitcoin withdrawal sender with the given signer.
    pub fn new(signer: S) -> Self {
        let network = signer.network();
        Self {
            signer,
            _network: network,
        }
    }
    
    /// Builds, signs, and broadcasts a withdrawal transaction.
    ///
    /// # Returns
    ///
    /// The transaction ID (TXID) of the broadcast transaction.
    ///
    /// # Errors
    ///
    /// - [`BitcoinSenderError::MainnetRefused`] if the signer refuses mainnet
    /// - [`BitcoinSenderError::InsufficientFunds`] if UTXOs cannot cover amount + fee
    /// - [`BitcoinSenderError::BroadcastFailed`] if the network rejects the transaction
    pub async fn send_withdrawal(
        &self,
        request: &WithdrawalRequest,
    ) -> Result<Txid, BitcoinSenderError> {
        // 1. Build and sign the transaction
        let prepared = self.signer.sign_withdrawal(request).await?;
        
        // 2. Broadcast (placeholder - needs RPC client)
        // Real implementation would POST to esplora or call bitcoind RPC
        
        // 3. Return TXID
        Ok(prepared.txid)
    }
    
    /// Checks if a transaction has reached the required confirmations (2).
    ///
    /// # Returns
    ///
    /// `true` if the transaction has 2+ confirmations, `false` otherwise.
    pub async fn is_confirmed(&self, _txid: &Txid) -> Result<bool, BitcoinSenderError> {
        // Placeholder: Real implementation would query block height and
        // transaction confirmation count via RPC or block explorer API
        Err(BitcoinSenderError::RpcError(
            "confirmation tracking not implemented".to_owned(),
        ))
    }
}

impl fmt::Debug for BitcoinWithdrawalSender<LocalTestSigner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BitcoinWithdrawalSender")
            .field("network", &self._network)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_signer_refuses_mainnet() {
        let result = LocalTestSigner::new(BitcoinNetwork::Mainnet);
        assert!(matches!(result, Err(BitcoinSenderError::MainnetRefused)));
    }

    #[test]
    fn local_signer_accepts_testnet() {
        let signer = LocalTestSigner::new(BitcoinNetwork::Testnet).unwrap();
        assert_eq!(signer.network(), BitcoinNetwork::Testnet);
    }

    #[test]
    fn withdrawal_request_validates_asset() {
        let xlm = Money::from_minor(Asset::Xlm, 1000);
        let result = WithdrawalRequest::new("addr".to_owned(), xlm);
        assert!(result.is_err());
    }

    #[test]
    fn withdrawal_request_converts_sats() {
        let btc = Money::from_minor(Asset::Btc, 100_000);
        let request = WithdrawalRequest::new("tb1q...".to_owned(), btc).unwrap();
        assert_eq!(request.amount_sats, 100_000);
    }

    #[test]
    fn withdrawal_request_with_memo() {
        let btc = Money::from_minor(Asset::Btc, 50_000);
        let request = WithdrawalRequest::new("addr".to_owned(), btc)
            .unwrap()
            .with_memo("test withdrawal".to_owned());
        assert_eq!(request.memo, Some("test withdrawal".to_owned()));
    }

    #[tokio::test]
    async fn mainnet_signer_refuses_signing() {
        // Even if we bypass construction, signing must refuse
        let signer = LocalTestSigner {
            network: BitcoinNetwork::Mainnet,
        };
        let request = WithdrawalRequest {
            destination: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_owned(),
            amount_sats: 10_000,
            memo: None,
        };
        let result = signer.sign_withdrawal(&request).await;
        assert!(matches!(result, Err(BitcoinSenderError::MainnetRefused)));
    }
}
