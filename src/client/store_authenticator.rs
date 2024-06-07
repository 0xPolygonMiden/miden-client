use alloc::rc::Rc;
use core::cell::RefCell;

use miden_objects::{
    accounts::{AccountDelta, AuthSecretKey},
    crypto::dsa::rpo_falcon512::{self, Polynomial},
    Digest, Felt, Word,
};
use miden_tx::{AuthenticationError, TransactionAuthenticator};
use rand::Rng;

use crate::store::Store;

/// Represents an authenticator based on a [Store]
pub struct StoreAuthenticator<R, S> {
    store: Rc<S>,
    rng: RefCell<R>
}

impl<R: Rng, S: Store> StoreAuthenticator<R, S> {
    pub fn new_with_rng(store: Rc<S>, rng: R) -> Self {
        StoreAuthenticator { store, rng: RefCell::new(rng) }
    }
}

impl<R: Rng, S: Store> TransactionAuthenticator for StoreAuthenticator<R, S> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The pub key should correspond to one of the keys tracked by the authenticator's store.
    ///
    /// # Errors
    /// If the public key is not found in the store, [AuthenticationError::UnknownKey] is
    /// returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let mut rng = self.rng.borrow_mut();

        let secret_key = self
            .store
            .get_account_auth_by_pub_key(pub_key)
            .map_err(|_| AuthenticationError::UnknownKey(format!("{}", Digest::from(pub_key))))?;
        

        let AuthSecretKey::RpoFalcon512(k) = secret_key;
        get_falcon_signature(&k, message, &mut *rng)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

// TODO: Remove the falcon signature function once it's available on base and made public

/// Retrieves a falcon signature over a message.
/// Gets as input a [Word] containing a secret key, and a [Word] representing a message and
/// outputs a vector of values to be pushed onto the advice stack.
/// The values are the ones required for a Falcon signature verification inside the VM and they are:
///
/// 1. The nonce represented as 8 field elements.
/// 2. The expanded public key represented as the coefficients of a polynomial of degree < 512.
/// 3. The signature represented as the coefficients of a polynomial of degree < 512.
/// 4. The product of the above two polynomials in the ring of polynomials with coefficients
///    in the Miden field.
///
/// # Errors
/// Will return an error if either:
/// - The secret key is malformed due to either incorrect length or failed decoding.
/// - The signature generation failed.
///
/// TODO: once this gets made public in miden base, remve this implementation and use the one from
/// base
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
