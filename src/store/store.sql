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

-- Create account_auth table
CREATE TABLE account_auth (
    account_id UNSIGNED BIG INT NOT NULL,  -- ID of the account
    auth_info BLOB NOT NULL,               -- Serialized representation of information needed for authentication
    PRIMARY KEY (account_id),
    FOREIGN KEY (account_id) REFERENCES accounts(id)
);

-- Create accounts table
CREATE TABLE accounts (
    id UNSIGNED BIG INT NOT NULL,  -- account ID.
    code_root BLOB NOT NULL,       -- root of the account_code
    storage_root BLOB NOT NULL,    -- root of the account_storage Merkle tree.
    vault_root BLOB NOT NULL,      -- root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,         -- account nonce.
    committed BOOLEAN NOT NULL,    -- true if recorded, false if not.
    PRIMARY KEY (id),
    FOREIGN KEY (code_root) REFERENCES account_code(root),
    FOREIGN KEY (storage_root) REFERENCES account_storage(root),
    FOREIGN KEY (vault_root) REFERENCES account_vaults(root)
);

-- Create input notes table
CREATE TABLE input_notes (
    hash BLOB NOT NULL,                                     -- the note hash
    nullifier BLOB NOT NULL,                                -- the nullifier of the note
    script BLOB NOT NULL,                                   -- the serialized NoteScript, including script hash and ProgramAst
    vault BLOB NOT NULL,                                    -- the serialized NoteVault, including vault hash and list of assets
    inputs BLOB NOT NULL,                                   -- the serialized NoteInputs, including inputs hash and list of inputs
    serial_num BLOB NOT NULL,                               -- the note serial number
    sender_id UNSIGNED BIG INT NOT NULL,                    -- the account ID of the sender
    tag UNSIGNED BIG INT NOT NULL,                          -- the note tag
    num_assets UNSIGNED BIG INT NOT NULL,                   -- the number of assets in the note
    inclusion_proof BLOB NOT NULL,                          -- the inclusion proof of the note against a block number
    recipients BLOB NOT NULL,                               -- a list of account IDs of accounts which can consume this note
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed or consumed
        'pending', 'committed', 'consumed'
        )),
    commit_height UNSIGNED BIG INT NOT NULL,                -- the block number at which the note was included into the chain
    PRIMARY KEY (hash)
);

-- Create state sync table
CREATE TABLE state_sync (
    block_number UNSIGNED BIG INT NOT NULL, -- the block number of the most recent state sync
    tags BLOB NOT NULL,                     -- the serialized list of tags
    PRIMARY KEY (block_number)
);

-- insert initial row into state_sync table
INSERT OR IGNORE INTO state_sync (block_number, tags)
SELECT 0, '[]'
WHERE (
    SELECT COUNT(*) FROM state_sync
) = 0;
