// use miden_objects::{
//     accounts::Account,
//     crypto::dsa::rpo_falcon512::KeyPair,
//     Word,
// };
// use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use async_trait::async_trait;

// Hoping that eventually, we can use the generic store type defined in client/store/mod.rs.
// Might need to add conditional rendering to add async_trait to the trait definitions? 
// Or maybe we can just add it to the trait definitions and it will work for both the CLI and the browser?
// Basically, can we keep the generic Client definition and the Store trait definitions the same
// and add what we need for the browser-specific implementation
#[async_trait(?Send)]
pub trait Store {
    async fn insert_string(
        &mut self,
        data: String
    ) -> Result<(), ()>;

    // async fn insert_account(
    //     &mut self,
    //     account: &Account,
    //     account_seed: Option<Word>,
    //     auth_info: &AuthInfo,
    // ) -> Result<(), ()>;
}

// DATABASE AUTH INFO
// ================================================================================================

// #[derive(Debug)]
// pub enum AuthInfo {
//     RpoFalcon512(KeyPair),
// }

// const RPO_FALCON512_AUTH: u8 = 0;

// impl AuthInfo {
//     /// Returns byte identifier of specific AuthInfo
//     const fn type_byte(&self) -> u8 {
//         match self {
//             AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
//         }
//     }
// }

// impl Serializable for AuthInfo {
//     fn write_into<W: ByteWriter>(
//         &self,
//         target: &mut W,
//     ) {
//         let mut bytes = vec![self.type_byte()];
//         match self {
//             AuthInfo::RpoFalcon512(key_pair) => {
//                 bytes.append(&mut key_pair.to_bytes());
//                 target.write_bytes(&bytes);
//             },
//         }
//     }
// }

// impl Deserializable for AuthInfo {
//     fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
//         let auth_type: u8 = source.read_u8()?;
//         match auth_type {
//             RPO_FALCON512_AUTH => {
//                 let key_pair = KeyPair::read_from(source)?;
//                 Ok(AuthInfo::RpoFalcon512(key_pair))
//             },
//             val => Err(DeserializationError::InvalidValue(val.to_string())),
//         }
//     }
// }