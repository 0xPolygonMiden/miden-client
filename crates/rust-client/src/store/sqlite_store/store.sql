-- Create account_code table
CREATE TABLE account_code (
    root TEXT NOT NULL,         -- root of the Merkle tree for all exported procedures in account module.
    code BLOB NOT NULL,         -- serialized account code.
    PRIMARY KEY (root)
);

-- Create account_storage table
CREATE TABLE account_storage (
    root TEXT NOT NULL,         -- root of the account storage Merkle tree.
    slots BLOB NOT NULL,        -- serialized key-value pair of non-empty account slots.
    PRIMARY KEY (root)
);

-- Create account_vaults table
CREATE TABLE account_vaults (
    root TEXT NOT NULL,         -- root of the Merkle tree for the account asset vault.
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
    code_root TEXT NOT NULL,       -- Root of the account_code
    storage_root TEXT NOT NULL,    -- Root of the account_storage Merkle tree.
    vault_root TEXT NOT NULL,      -- Root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,         -- Account nonce.
    committed BOOLEAN NOT NULL,    -- True if recorded, false if not.
    account_seed BLOB NULL,        -- Account seed used to generate the ID. Expected to be NULL for non-new accounts
    account_hash TEXT NOT NULL UNIQUE,    -- Account state hash
    PRIMARY KEY (id, nonce),
    FOREIGN KEY (code_root) REFERENCES account_code(root),
    FOREIGN KEY (storage_root) REFERENCES account_storage(root),
    FOREIGN KEY (vault_root) REFERENCES account_vaults(root)

    CONSTRAINT check_seed_nonzero CHECK (NOT (nonce = 0 AND account_seed IS NULL))
);

CREATE UNIQUE INDEX idx_account_hash ON accounts(account_hash);

-- Create transactions table
CREATE TABLE transactions (
    id TEXT NOT NULL,                                -- Transaction ID (hash of various components)
    account_id UNSIGNED BIG INT NOT NULL,            -- ID of the account against which the transaction was executed.
    init_account_state BLOB NOT NULL,                -- Hash of the account state before the transaction was executed.
    final_account_state BLOB NOT NULL,               -- Hash of the account state after the transaction was executed.
    input_notes BLOB,                                -- Serialized list of input note hashes
    output_notes BLOB,                               -- Serialized list of output note hashes
    script_hash TEXT,                                -- Transaction script hash
    block_num UNSIGNED BIG INT,                      -- Block number for the block against which the transaction was executed.
    commit_height UNSIGNED BIG INT NULL,             -- Block number of the block at which the transaction was included in the chain.
    discarded BOOLEAN NOT NULL,                      -- Boolean indicating if the transaction is discarded
    FOREIGN KEY (script_hash) REFERENCES transaction_scripts(script_hash),
    PRIMARY KEY (id)
);

CREATE TABLE transaction_scripts (
    script_hash TEXT NOT NULL,                       -- Transaction script Hash
    script BLOB,                                     -- serialized Transaction script

    PRIMARY KEY (script_hash)
);

-- Create input notes table
CREATE TABLE input_notes (
    note_id TEXT NOT NULL,                                  -- the note id
    assets BLOB NOT NULL,                                   -- the serialized list of assets
    serial_number BLOB NOT NULL,                            -- the serial number of the note
    inputs BLOB NOT NULL,                                   -- the serialized list of note inputs
    script_hash TEXT NOT NULL,                              -- the script hash of the note, used to join with the notes_scripts table
    nullifier TEXT NOT NULL,                                -- the nullifier of the note, used to query by nullifier
    state_discriminant UNSIGNED INT NOT NULL,               -- state discriminant of the note, used to query by state
    state BLOB NOT NULL,                                    -- serialized note state
    created_at UNSIGNED BIG INT NOT NULL,                   -- timestamp of the note creation/import

    PRIMARY KEY (note_id)
    FOREIGN KEY (script_hash) REFERENCES notes_scripts(script_hash)
);

-- Create output notes table
CREATE TABLE output_notes (
    note_id TEXT NOT NULL,                                  -- the note id
    recipient BLOB NOT NULL,                                -- the note recipient
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault hash and list of assets
    status TEXT CHECK( status IN (                          -- the status of the note - either expected, committed, processing or consumed
        'Expected', 'Committed', 'Processing', 'Consumed'
        )),

    inclusion_proof BLOB NULL,                              -- serialized inclusion proof

    metadata BLOB NOT NULL,                                 -- serialized metadata

    nullifier TEXT NULL,
    script_hash TEXT NULL,
    details BLOB NULL,                                      -- serialized note record details
    consumer_transaction_id BLOB NULL,                      -- the transaction ID of the transaction that consumed the note
    created_at UNSIGNED BIG INT NOT NULL,                   -- timestamp of the note creation/import
    expected_height UNSIGNED BIG INT NULL,                  -- block height when the note is expected to be committed
    submitted_at UNSIGNED BIG INT NULL,                      -- timestamp of the note submission to node
    nullifier_height UNSIGNED BIG INT NULL,                 -- block height when the nullifier arrived
    ignored BOOLEAN NOT NULL DEFAULT 0,                     -- whether the note is ignored in sync
    imported_tag UNSIGNED INT NULL,                         -- imported tag for the note

    FOREIGN KEY (consumer_transaction_id) REFERENCES transactions(id)
    PRIMARY KEY (note_id)

    CONSTRAINT check_valid_consumer_transaction_id CHECK (consumer_transaction_id IS NULL OR status != 'Expected')
    CONSTRAINT check_valid_submitted_at CHECK (submitted_at IS NOT NULL OR status != 'Processing')
    CONSTRAINT check_valid_nullifier_height CHECK (nullifier_height IS NOT NULL OR status != 'Consumed')
    CONSTRAINT check_ignored_output_notes CHECK (NOT(ignored)) -- Output notes shouldn't be ignored. This check will be removed when we refactor the output notes table.
);

-- Create note's scripts table, used for both input and output notes
CREATE TABLE notes_scripts (
    script_hash TEXT NOT NULL,                       -- Note script Hash
    serialized_note_script BLOB,                     -- NoteScript, serialized

    PRIMARY KEY (script_hash)
);

-- Create state sync table
CREATE TABLE state_sync (
    block_num UNSIGNED BIG INT NOT NULL,    -- the block number of the most recent state sync
    tags BLOB NULL,                     -- the serialized list of tags, a NULL means an empty list
    PRIMARY KEY (block_num)
);

-- insert initial row into state_sync table
INSERT OR IGNORE INTO state_sync (block_num, tags)
SELECT 0, NULL
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
