-- Create account_code table
CREATE TABLE account_code (
    root BLOB NOT NULL,         -- root of the Merkle tree for all exported procedures in account module.
    procedures BLOB NOT NULL,   -- serialized procedure digests for the account code.
    module BLOB NOT NULL,       -- serialized ModuleAst for the account code.
    PRIMARY KEY (root)
);

-- Create account_storage table
CREATE TABLE account_storage (
    root BLOB NOT NULL,         -- root of the account storage Merkle tree.
    slots BLOB NOT NULL,        -- serialized key-value pair of non-empty account slots.
    PRIMARY KEY (root)
);

-- Create account_vaults table
CREATE TABLE account_vaults (
    root BLOB NOT NULL,         -- root of the Merkle tree for the account vault.
    assets BLOB NOT NULL,       -- serialized account vault assets.
    PRIMARY KEY (root)
);

-- Create account_keys table
CREATE TABLE account_keys (
    account_id UNSIGNED BIG INT NOT NULL, -- ID of the account
    key_pair BLOB NOT NULL,               -- key pair 
    PRIMARY KEY (account_id),
    FOREIGN KEY (account_id) REFERENCES accounts(id)
);

-- Create accounts table
CREATE TABLE accounts (
    id UNSIGNED BIG INT NOT NULL,  -- account ID.
    code_root BLOB NOT NULL,       -- root of the account_code Merkle tree.
    storage_root BLOB NOT NULL,    -- root of the account_storage Merkle tree.
    vault_root BLOB NOT NULL,      -- root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,         -- account nonce.
    committed BOOLEAN NOT NULL,    -- true if recorded, false if not.
    PRIMARY KEY (id),
    FOREIGN KEY (code_root) REFERENCES account_code(root),
    FOREIGN KEY (storage_root) REFERENCES account_storage(root),
    FOREIGN KEY (vault_root) REFERENCES account_vaults(root)
);
