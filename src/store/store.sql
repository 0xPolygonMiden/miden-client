-- Create account_code table
CREATE TABLE account_code (
    root BLOB NOT NULL,      -- root of the Merkle tree for all exported procedures in account module.
    procedures BLOB NOT NULL, -- serialized procedure digests for the account code.
    module BLOB NOT NULL      -- serialized ModuleAst for the account code.
);

-- Create account_storage table
CREATE TABLE account_storage (
    root BLOB NOT NULL,  -- root of the account storage Merkle tree.
    slots BLOB NOT NULL  -- serialized key-value pair of non-empty account slots.
);

-- Create account_vault table
CREATE TABLE account_vault (
    root BLOB NOT NULL,   -- root of the Merkle tree for the account vault.
    assets BLOB NOT NULL  -- serialized account vault assets.
);

-- Update accounts table
CREATE TABLE accounts (
    id UNSIGNED BIG INT NOT NULL,  -- account ID.
    code_root BLOB NOT NULL,      -- root of the account_code Merkle tree.
    storage_root BLOB NOT NULL,   -- root of the account_storage Merkle tree.
    vault_root BLOB NOT NULL,     -- root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,        -- account nonce.
    committed BOOLEAN NOT NULL    -- true if recorded, false if not.
);
