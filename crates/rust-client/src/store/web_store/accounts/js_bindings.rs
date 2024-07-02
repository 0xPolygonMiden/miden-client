use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Account IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/accounts.js")]
extern "C" {
    // GETS
    // ================================================================================================
    #[wasm_bindgen(js_name = getAccountIds)]
    pub fn idxdb_get_account_ids() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAllAccountStubs)]
    pub fn idxdb_get_account_stubs() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountStub)]
    pub fn idxdb_get_account_stub(account_id: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountCode)]
    pub fn idxdb_get_account_code(code_root: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountStorage)]
    pub fn idxdb_get_account_storage(storage_root: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountAssetVault)]
    pub fn idxdb_get_account_asset_vault(vault_root: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountAuth)]
    pub fn idxdb_get_account_auth(account_id: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getAccountAuthByPubKey)]
    pub fn idxdb_get_account_auth_by_pub_key(pub_key: Vec<u8>) -> JsValue;

    #[wasm_bindgen(js_name = fetchAndCacheAccountAuthByPubKey)]
    pub fn idxdb_fetch_and_cache_account_auth_by_pub_key(account_id: String) -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = insertAccountCode)]
    pub fn idxdb_insert_account_code(
        code_root: String,
        code: String,
        module: Vec<u8>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountStorage)]
    pub fn idxdb_insert_account_storage(
        storage_root: String,
        storage_slots: Vec<u8>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAssetVault)]
    pub fn idxdb_insert_account_asset_vault(vault_root: String, assets: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountRecord)]
    pub fn idxdb_insert_account_record(
        id: String,
        code_root: String,
        storage_root: String,
        vault_root: String,
        nonce: String,
        committed: bool,
        account_seed: Option<Vec<u8>>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAuth)]
    pub fn idxdb_insert_account_auth(
        id: String,
        auth_info: Vec<u8>,
        pub_key: Vec<u8>,
    ) -> js_sys::Promise;
}
