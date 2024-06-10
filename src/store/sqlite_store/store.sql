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
    root BLOB NOT NULL,         -- root of the Merkle tree for the account asset vault.
    assets BLOB NOT NULL,       -- serialized account vault assets.
    PRIMARY KEY (root)
);

-- Create account_auth table
CREATE TABLE account_auth (
    account_id UNSIGNED BIG INT NOT NULL,  -- ID of the account
    auth_info BLOB NOT NULL,               -- Serialized representation of information needed for authentication
    pub_key BLOB NOT NULL,                 -- Public key for easier authenticator use
    PRIMARY KEY (account_id)
);

-- Create accounts table
CREATE TABLE accounts (
    id UNSIGNED BIG INT NOT NULL,  -- Account ID.
    code_root BLOB NOT NULL,       -- Root of the account_code
    storage_root BLOB NOT NULL,    -- Root of the account_storage Merkle tree.
    vault_root BLOB NOT NULL,      -- Root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,         -- Account nonce.
    committed BOOLEAN NOT NULL,    -- True if recorded, false if not.
    account_seed BLOB NULL,        -- Account seed used to generate the ID. Expected to be NULL for non-new accounts
    PRIMARY KEY (id, nonce),
    FOREIGN KEY (code_root) REFERENCES account_code(root),
    FOREIGN KEY (storage_root) REFERENCES account_storage(root),
    FOREIGN KEY (vault_root) REFERENCES account_vaults(root)

    CONSTRAINT check_seed_nonzero CHECK (NOT (nonce = 0 AND account_seed IS NULL))
);

-- Create transactions table
CREATE TABLE transactions (
    id BLOB NOT NULL,                                -- Transaction ID (hash of various components)
    account_id UNSIGNED BIG INT NOT NULL,            -- ID of the account against which the transaction was executed.
    init_account_state BLOB NOT NULL,                -- Hash of the account state before the transaction was executed.
    final_account_state BLOB NOT NULL,               -- Hash of the account state after the transaction was executed.
    input_notes BLOB,                                -- Serialized list of input note hashes
    output_notes BLOB,                               -- Serialized list of output note hashes
    script_hash BLOB,                                -- Transaction script hash
    script_inputs BLOB,                              -- Transaction script inputs
    block_num UNSIGNED BIG INT,                      -- Block number for the block against which the transaction was executed.
    commit_height UNSIGNED BIG INT NULL,             -- Block number of the block at which the transaction was included in the chain.
    FOREIGN KEY (script_hash) REFERENCES transaction_scripts(script_hash),
    PRIMARY KEY (id)
);

CREATE TABLE transaction_scripts (
    script_hash BLOB NOT NULL,                       -- Transaction script Hash
    program BLOB,                                    -- Transaction script program, serialized

    PRIMARY KEY (script_hash)
);

-- Create input notes table
CREATE TABLE input_notes (
    note_id BLOB NOT NULL,                                  -- the note id
    recipient BLOB NOT NULL,                                -- the note recipient
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault hash and list of assets
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed, processing or consumed
        'Pending', 'Committed', 'Processing', 'Consumed'
        )),

    inclusion_proof JSON NULL,                              -- JSON consisting of the following fields:
    -- block_num                                              -- number of the block the note was included in
    -- note_index                                             -- the index of the note in the note Merkle tree of the block the note was created in.
    -- sub_hash                                               -- sub hash of the block the note was included in stored as a hex string
    -- note_root                                              -- the note root of the block the note was created in
    -- note_path                                              -- the Merkle path to the note in the note Merkle tree of the block the note was created in, stored as an array of digests

    metadata JSON NULL,                                     -- JSON consisting of the following fields:
    -- sender_id                                              -- the account ID of the sender
    -- tag                                                    -- the note tag

    details JSON NOT NULL,                                  -- JSON consisting of the following fields:
    -- nullifier                                              -- the nullifier of the note
    -- script_hash                                                 -- the note's script hash
    -- inputs                                                 -- the serialized NoteInputs, including inputs hash and list of inputs
    -- serial_num                                             -- the note serial number
    consumer_transaction_id BLOB NULL,                      -- the transaction ID of the transaction that consumed the note
    created_at UNSIGNED BIG INT NOT NULL,                   -- timestamp of the note creation/import
    submitted_at UNSIGNED BIG INT NULL,                      -- timestamp of the note submission to node
    nullifier_height UNSIGNED BIG INT NULL,                 -- block height when the nullifier arrived
    FOREIGN KEY (consumer_transaction_id) REFERENCES transactions(id)
    PRIMARY KEY (note_id)

    CONSTRAINT check_valid_inclusion_proof_json CHECK (
      inclusion_proof IS NULL OR
      (
        json_extract(inclusion_proof, '$.origin.block_num') IS NOT NULL AND
        json_extract(inclusion_proof, '$.origin.node_index') IS NOT NULL AND
        json_extract(inclusion_proof, '$.sub_hash') IS NOT NULL AND
        json_extract(inclusion_proof, '$.note_root') IS NOT NULL AND
        json_extract(inclusion_proof, '$.note_path') IS NOT NULL
      ))
    CONSTRAINT check_valid_metadata_json CHECK (metadata IS NULL OR (json_extract(metadata, '$.sender') IS NOT NULL AND json_extract(metadata, '$.tag') IS NOT NULL))
    CONSTRAINT check_valid_consumer_transaction_id CHECK (consumer_transaction_id IS NULL OR status != 'Pending')
);

-- Create output notes table
CREATE TABLE output_notes (
    note_id BLOB NOT NULL,                                  -- the note id
    recipient BLOB NOT NULL,                                -- the note recipient
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault hash and list of assets
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed, processing or consumed
        'Pending', 'Committed', 'Processing', 'Consumed'
        )),

    inclusion_proof JSON NULL,                              -- JSON consisting of the following fields:
    -- block_num                                              -- number of the block the note was included in
    -- note_index                                             -- the index of the note in the note Merkle tree of the block the note was created in.
    -- sub_hash                                               -- sub hash of the block the note was included in stored as a hex string
    -- note_root                                              -- the note root of the block the note was created in
    -- note_path                                              -- the Merkle path to the note in the note Merkle tree of the block the note was created in, stored as an array of digests

    metadata JSON NOT NULL,                                 -- JSON consisting of the following fields:
    -- sender_id                                              -- the account ID of the sender
    -- tag                                                    -- the note tag

    details JSON NULL,                                      -- JSON consisting of the following fields:
    -- nullifier                                              -- the nullifier of the note
    -- script                                                 -- the note's script hash
    -- inputs                                                 -- the serialized NoteInputs, including inputs hash and list of inputs
    -- serial_num                                             -- the note serial number
    consumer_transaction_id BLOB NULL,                      -- the transaction ID of the transaction that consumed the note
    created_at UNSIGNED BIG INT NOT NULL,                   -- timestamp of the note creation/import
    submitted_at UNSIGNED BIG INT NULL,                      -- timestamp of the note submission to node
    nullifier_height UNSIGNED BIG INT NULL,                 -- block height when the nullifier arrived
    FOREIGN KEY (consumer_transaction_id) REFERENCES transactions(id)
    PRIMARY KEY (note_id)

    CONSTRAINT check_valid_inclusion_proof_json CHECK (
      inclusion_proof IS NULL OR
      (
        json_extract(inclusion_proof, '$.origin.block_num') IS NOT NULL AND
        json_extract(inclusion_proof, '$.origin.node_index') IS NOT NULL AND
        json_extract(inclusion_proof, '$.sub_hash') IS NOT NULL AND
        json_extract(inclusion_proof, '$.note_root') IS NOT NULL AND
        json_extract(inclusion_proof, '$.note_path') IS NOT NULL
      ))
    CONSTRAINT check_valid_details_json CHECK (
      details IS NULL OR
      (
        json_extract(details, '$.nullifier') IS NOT NULL AND
        json_extract(details, '$.script_hash') IS NOT NULL AND
        json_extract(details, '$.inputs') IS NOT NULL AND
        json_extract(details, '$.serial_num') IS NOT NULL
      ))
    CONSTRAINT check_valid_consumer_transaction_id CHECK (consumer_transaction_id IS NULL OR status != 'Pending')
);

-- Create note's scripts table, used for both input and output notes
-- TODO: can't do FOREIGN KEY over json fields, sure we're ok?
CREATE TABLE notes_scripts (
    script_hash BLOB NOT NULL,                       -- Note script Hash
    serialized_note_script BLOB,                     -- NoteScript, serialized

    PRIMARY KEY (script_hash)
);

-- Create state sync table
CREATE TABLE state_sync (
    block_num UNSIGNED BIG INT NOT NULL,    -- the block number of the most recent state sync
    tags BLOB NOT NULL,                     -- the serialized list of tags
    PRIMARY KEY (block_num)
);

-- insert initial row into state_sync table
INSERT OR IGNORE INTO state_sync (block_num, tags)
SELECT 0, '[]'
WHERE (
    SELECT COUNT(*) FROM state_sync
) = 0;

-- Create block headers table
CREATE TABLE block_headers (
    block_num UNSIGNED BIG INT NOT NULL,  -- block number
    header BLOB NOT NULL,                 -- serialized block header
    chain_mmr_peaks BLOB NOT NULL,        -- serialized peaks of the chain MMR at this block
    has_client_notes BOOL NOT NULL,       -- whether the block has notes relevant to the client
    PRIMARY KEY (block_num)
);

-- Create chain mmr nodes
CREATE TABLE chain_mmr_nodes (
    id UNSIGNED BIG INT NOT NULL,   -- in-order index of the internal MMR node
    node BLOB NOT NULL,             -- internal node value (hash)
    PRIMARY KEY (id)
)
