// ACCOUNT RECORD
// --------------------------------------------------------------------------------------------

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

    pub fn locked(&self) -> bool {
        self.status.locked()
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
// --------------------------------------------------------------------------------------------

pub enum AccountStatus {
    New { seed: Word },
    Tracked,
    Locked,
}

impl AccountStatus {
    pub fn locked(&self) -> bool {
        matches!(self, AccountStatus::Locked { .. })
    }

    pub fn seed(&self) -> Option<&Word> {
        match self {
            AccountStatus::New { seed } => Some(seed),
            _ => None,
        }
    }
}
