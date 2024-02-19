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
    PRIMARY KEY (account_id)
);

-- Create accounts table
CREATE TABLE accounts (
    id UNSIGNED BIG INT NOT NULL,  -- account ID.
    code_root BLOB NOT NULL,       -- root of the account_code
    storage_root BLOB NOT NULL,    -- root of the account_storage Merkle tree.
    vault_root BLOB NOT NULL,      -- root of the account_vault Merkle tree.
    nonce BIGINT NOT NULL,         -- account nonce.
    committed BOOLEAN NOT NULL,    -- true if recorded, false if not.
    account_seed BLOB NULL,        -- account seed used to generate the ID. Expected to be NULL for non-new accounts
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
    nullifier BLOB NOT NULL,                                -- the nullifier of the note
    recipient BLOB NOT NULL,                                -- the note recipient
    script BLOB NOT NULL,                                   -- the serialized NoteScript, including script hash and ProgramAst
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault hash and list of assets
    inputs BLOB NOT NULL,                                   -- the serialized NoteInputs, including inputs hash and list of inputs
    serial_num BLOB NOT NULL,                               -- the note serial number.
    sender_id UNSIGNED BIG INT NULL,                        -- the account ID of the sender. Known once the note is recorded on chain
    tag UNSIGNED BIG INT NULL,                              -- the note tag. Known once the note is recorded on-chain
    inclusion_proof BLOB NULL,                              -- the inclusion proof of the note against a block number. Known once the note is recorded on-chain
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed or consumed
        'pending', 'committed', 'consumed'
        )),
    PRIMARY KEY (note_id)
);

-- Create output notes table
CREATE TABLE output_notes (
    note_id BLOB NOT NULL,                                  -- the note id
    nullifier BLOB NULL,                                    -- the nullifier of the note, only known if we know script, inputs, serial_num
    recipient BLOB NOT NULL,                                -- the note recipient
    script BLOB NULL,                                       -- the serialized NoteScript, including script hash and ProgramAst. May not be known
    assets BLOB NOT NULL,                                   -- the serialized NoteAssets, including vault hash and list of assets
    inputs BLOB NULL,                                       -- the serialized NoteInputs, including inputs hash and list of inputs. May not be known
    serial_num BLOB NULL,                                   -- the note serial number. May not be known
    sender_id UNSIGNED BIG INT NOT NULL,                    -- the account ID of the sender
    tag UNSIGNED BIG INT NOT NULL,                          -- the note tag
    inclusion_proof BLOB NULL,                              -- the inclusion proof of the note against a block number
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed or consumed
        'pending', 'committed', 'consumed'
        )),
    PRIMARY KEY (note_id)
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
    notes_root BLOB NOT NULL,             -- root of the notes Merkle tree in this block
    sub_hash BLOB NOT NULL,               -- hash of all other header fields in the block
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
