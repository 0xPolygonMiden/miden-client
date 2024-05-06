use alloc::rc::Rc;
use core::cell::RefCell;

use miden_objects::{
    accounts::AccountDelta,
    crypto::dsa::rpo_falcon512::{self, Polynomial, PublicKey},
    Digest, Felt, Word,
};
use miden_tx::{AuthSecretKey, AuthenticationError, TransactionAuthenticator};
use rand::Rng;

use crate::store::sqlite_store::SqliteStore;

pub struct StoreAuthenticator<R> {
    store: Rc<SqliteStore>,
    rng: RefCell<R>,
}

impl<R: Rng> StoreAuthenticator<R> {
    pub fn new_with_rng(store: Rc<SqliteStore>, rng: R) -> Self {
        StoreAuthenticator { store, rng: RefCell::new(rng) }
    }
}

impl<R: Rng> TransactionAuthenticator for StoreAuthenticator<R> {
    /// Gets a signature over a message, given a public key.
    /// The key should be included in the `keys` map and should be a variant of [SecretKey].
    ///
    /// Supported signature schemes:
    /// - RpoFalcon512
    ///
    /// # Errors
    /// If the public key is not contained in the `keys` map, [AuthenticationError::UnknownKey] is
    /// returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let mut rng = self.rng.borrow_mut();
        let keys = self.store.get_account_auths().unwrap();

        let secret_key = keys
            .iter()
            .find(|k| match k {
                AuthSecretKey::RpoFalcon512(k) => k.public_key() == PublicKey::new(pub_key),
            })
            .ok_or(AuthenticationError::UnknownKey(format!("{}", Digest::from(pub_key))))?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key;
        get_falcon_signature(k, message, &mut *rng)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Retrieves a falcon signature over a message.
/// Gets as input a [Word] containing a secret key, and a [Word] representing a message and
/// outputs a vector of values to be pushed onto the advice stack.
/// The values are the ones required for a Falcon signature verification inside the VM and they are:
///
/// 1. The nonce represented as 8 field elements.
/// 2. The expanded public key represented as the coefficients of a polynomial of degree < 512.
/// 3. The signature represented as the coefficients of a polynomial of degree < 512.
/// 4. The product of the above two polynomials in the ring of polynomials with coefficients
/// in the Miden field.
///
/// # Errors
/// Will return an error if either:
/// - The secret key is malformed due to either incorrect length or failed decoding.
/// - The signature generation failed.
fn get_falcon_signature<R: Rng>(
    key: &rpo_falcon512::SecretKey,
    message: Word,
    rng: &mut R,
) -> Result<Vec<Felt>, AuthenticationError> {
    // Generate the signature
    let sig = key.sign_with_rng(message, rng);
    // The signature is composed of a nonce and a polynomial s2
    // The nonce is represented as 8 field elements.
    let nonce = sig.nonce();
    // We convert the signature to a polynomial
    let s2 = sig.sig_poly();
    // We also need in the VM the expanded key corresponding to the public key the was provided
    // via the operand stack
    let h = key.compute_pub_key_poly().0;
    // Lastly, for the probabilistic product routine that is part of the verification procedure,
    // we need to compute the product of the expanded key and the signature polynomial in
    // the ring of polynomials with coefficients in the Miden field.
    let pi = Polynomial::mul_modulo_p(&h, s2);
    // We now push the nonce, the expanded key, the signature polynomial, and the product of the
    // expanded key and the signature polynomial to the advice stack.
    let mut result: Vec<Felt> = nonce.to_elements().to_vec();

    result.extend(h.coefficients.iter().map(|a| Felt::from(a.value() as u32)));
    result.extend(s2.coefficients.iter().map(|a| Felt::from(a.value() as u32)));
    result.extend(pi.iter().map(|a| Felt::new(*a)));
    result.reverse();
    Ok(result)
}
