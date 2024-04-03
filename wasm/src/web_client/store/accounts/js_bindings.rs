// Account IndexedDB Operations
#[wasm_bindgen(module = "/js/db/accounts.js")]
extern "C" {
    // GETS
    #[wasm_bindgen(js_name = getAccountStub)]
    fn idxdb_get_account_stub(
        account_id: String
    ) -> JsValue;

    #[wasm_bindgen(js_name = getAccountCode)]
    fn idxdb_get_account_code(
        root: String
    ) -> JsValue;

    #[wasm_bindgen(js_name = getAccountAuth)]
    fn idxdb_get_account_auth(
        account_id: String
    ) -> JsValue;

    #[wasm_bindgen(js_name = getAllAccountStubs)]
    fn idxdb_get_account_stubs() -> JsValue;

    #[wasm_bindgen(js_name = getAccountIds)]
    fn idxdb_get_account_ids() -> JsValue;

    // INSERTS
    #[wasm_bindgen(js_name = insertAccountCode)]
    fn idxdb_insert_account_code(
        code_root: String, 
        code: String, 
        module: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountStorage)]
    fn idxdb_insert_account_storage(
        storage_root: String, 
        storage_slots: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAssetVault)]
    fn idxdb_insert_account_asset_vault(
        vault_root: String, 
        assets: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAuth)]
    fn idxdb_insert_account_auth(
        id: String,
        auth_info: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountRecord)]
    fn idxdb_insert_account_record(
        id: String, 
        code_root: String, 
        storage_root: String, 
        vault_root: String, 
        nonce: String, 
        committed: bool, 
        account_seed: Option<Vec<u8>>
    ) -> js_sys::Promise;
}