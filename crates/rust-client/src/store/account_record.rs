// ACCOUNT RECORD
// ================================================================================================
use core::fmt::Display;

use miden_objects::{accounts::Account, Digest, Word};

/// Represents a stored account state along with its status.
///
/// The account should be stored in the database with its parts normalized. Meaning that the
/// account header, vault, storage and code are stored separately. This is done to avoid data
/// duplication as the header can reference the same elements if they have equal roots.
pub struct AccountRecord {
    /// Full account object.
    account: Account,
    /// Status of the tracked account.
    status: AccountStatus,
}

impl AccountRecord {
    pub fn new(account: Account, status: AccountStatus) -> Self {
        Self { account, status }
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub fn status(&self) -> &AccountStatus {
        &self.status
    }

    pub fn is_locked(&self) -> bool {
        self.status.is_locked()
    }

    pub fn seed(&self) -> Option<&Word> {
        self.status.seed()
    }
}

impl From<AccountRecord> for Account {
    fn from(record: AccountRecord) -> Self {
        record.account
    }
}

// ACCOUNT STATUS
// ================================================================================================

/// Represents the status of an account tracked by the client.
///
/// The status of an account may change by local or external factors.
pub enum AccountStatus {
    /// The account is new and has not been used yet. The seed used to create the account is
    /// stored in this state.
    New { seed: Word },
    /// The account is tracked by the node and was used at least once.
    Tracked,
    /// The local account state doesn't match the node's state, rendering it unusable. Only used
    /// for private accounts.
    Locked { mismatched_node_hash: Digest },
}

impl AccountStatus {
    pub fn is_locked(&self) -> bool {
        matches!(self, AccountStatus::Locked { .. })
    }

    pub fn seed(&self) -> Option<&Word> {
        match self {
            AccountStatus::New { seed } => Some(seed),
            _ => None,
        }
    }
}

impl Display for AccountStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            AccountStatus::New { .. } => write!(f, "New"),
            AccountStatus::Tracked => write!(f, "Tracked"),
            AccountStatus::Locked { .. } => write!(f, "Locked"),
        }
    }
}
