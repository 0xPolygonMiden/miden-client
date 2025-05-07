-- Table for storing different settings in run-time, which need to persist over runs.
-- Note: we can store values of different types in the same `value` field.
CREATE TABLE settings (
    name  TEXT NOT NULL,
    value ANY,

    PRIMARY KEY (name),
    CONSTRAINT settings_name_is_not_empty CHECK (length(name) > 0)
) STRICT, WITHOUT ROWID;

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

-- Create foreign_account_code table
CREATE TABLE foreign_account_code(
    account_id TEXT NOT NULL,              -- ID of the account
    code_root TEXT NOT NULL,               -- Root of the account_code
    PRIMARY KEY (account_id),
    FOREIGN KEY (code_root) REFERENCES account_code(root)
);

-- Create accounts table
CREATE TABLE accounts (
    account_commitment TEXT NOT NULL UNIQUE,    -- Account state commitment
    id UNSIGNED BIG INT NOT NULL,               -- Account ID.
    code_root TEXT NOT NULL,                    -- Root of the account_code
    storage_root TEXT NOT NULL,                 -- Root of the account_storage Merkle tree.
    vault_root TEXT NOT NULL,                   -- Root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,                      -- Account nonce.
    committed BOOLEAN NOT NULL,                 -- True if recorded, false if not.
    account_seed BLOB NULL,                     -- Account seed used to generate the ID. Expected to be NULL for non-new accounts
    locked BOOLEAN NOT NULL,                    -- True if the account is locked, false if not.
    PRIMARY KEY (account_commitment),
    FOREIGN KEY (code_root) REFERENCES account_code(root),
    FOREIGN KEY (storage_root) REFERENCES account_storage(root),
    FOREIGN KEY (vault_root) REFERENCES account_vaults(root)

    CONSTRAINT check_seed_nonzero CHECK (NOT (nonce = 0 AND account_seed IS NULL))
);

CREATE UNIQUE INDEX idx_account_commitment ON accounts(account_commitment);

-- Create transactions table
CREATE TABLE transactions (
    id TEXT NOT NULL,                                -- Transaction ID (commitment of various components)
    details BLOB NOT NULL,                           -- Serialized transaction details
    script_root TEXT,                                -- Transaction script root
    block_num UNSIGNED BIG INT,                      -- Block number for the block against which the transaction was executed.
    commit_height UNSIGNED BIG INT NULL,             -- Block number of the block at which the transaction was included in the chain.
    discard_cause BLOB NULL,                         -- Serialized cause of the discarded transaction
    FOREIGN KEY (script_root) REFERENCES transaction_scripts(script_root),
    PRIMARY KEY (id)
);

CREATE TABLE transaction_scripts (
    script_root TEXT NOT NULL,                       -- Transaction script root
    script BLOB,                                     -- serialized Transaction script

    PRIMARY KEY (script_root)
);

-- Create input notes table
CREATE TABLE input_notes (
    note_id TEXT NOT NULL,                                  -- the note id
    assets BLOB NOT NULL,                                   -- the serialized list of assets
    serial_number BLOB NOT NULL,                            -- the serial number of the note
    inputs BLOB NOT NULL,                                   -- the serialized list of note inputs
    script_root TEXT NOT NULL,                              -- the script root of the note, used to join with the notes_scripts table
    nullifier TEXT NOT NULL,                                -- the nullifier of the note, used to query by nullifier
    state_discriminant UNSIGNED INT NOT NULL,               -- state discriminant of the note, used to query by state
    state BLOB NOT NULL,                                    -- serialized note state
    created_at UNSIGNED BIG INT NOT NULL,                   -- timestamp of the note creation/import

    PRIMARY KEY (note_id)
    FOREIGN KEY (script_root) REFERENCES notes_scripts(script_root)
);

-- Create output notes table
CREATE TABLE output_notes (
    note_id TEXT NOT NULL,                                  -- the note id
    recipient_digest TEXT NOT NULL,                                -- the note recipient
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault commitment and list of assets
    metadata BLOB NOT NULL,                                 -- serialized metadata
    nullifier TEXT NULL,
    expected_height UNSIGNED INT NOT NULL,                  -- the block height after which the note is expected to be created
-- TODO: normalize script data for output notes
--     script_commitment TEXT NULL,
    state_discriminant UNSIGNED INT NOT NULL,               -- state discriminant of the note, used to query by state
    state BLOB NOT NULL,                                    -- serialized note state

    PRIMARY KEY (note_id)
);

-- Create note's scripts table, used for both input and output notes
CREATE TABLE notes_scripts (
    script_root TEXT NOT NULL,                       -- Note script root
    serialized_note_script BLOB,                     -- NoteScript, serialized

    PRIMARY KEY (script_root)
);

-- Create state sync table
CREATE TABLE state_sync (
    block_num UNSIGNED BIG INT NOT NULL,    -- the block number of the most recent state sync
    PRIMARY KEY (block_num)
);

-- Create tags table
CREATE TABLE tags (
    tag BLOB NOT NULL,                  -- the serialized tag
    source BLOB NOT NULL                -- the serialized tag source
);

-- insert initial row into state_sync table
INSERT OR IGNORE INTO state_sync (block_num)
SELECT 0
WHERE (
    SELECT COUNT(*) FROM state_sync
) = 0;

-- Create block headers table
CREATE TABLE block_headers (
    block_num UNSIGNED BIG INT NOT NULL,  -- block number
    header BLOB NOT NULL,                 -- serialized block header
    partial_blockchain_peaks BLOB NOT NULL,        -- serialized peaks of the partial blockchain MMR at this block
    has_client_notes BOOL NOT NULL,       -- whether the block has notes relevant to the client
    PRIMARY KEY (block_num)
);

-- Create partial blockchain nodes
CREATE TABLE partial_blockchain_nodes (
    id UNSIGNED BIG INT NOT NULL,   -- in-order index of the internal MMR node
    node BLOB NOT NULL,             -- internal node value (commitment)
    PRIMARY KEY (id)
)
