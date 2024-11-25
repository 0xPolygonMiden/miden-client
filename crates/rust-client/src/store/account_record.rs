// ACCOUNT RECORD
// ================================================================================================
use core::fmt::Display;

use miden_objects::{accounts::Account, Word};

pub struct AccountRecord {
    account: Account,
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
pub enum AccountStatus {
    New { seed: Word },
    Tracked,
    Locked,
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
            AccountStatus::Locked => write!(f, "Locked"),
        }
    }
}
