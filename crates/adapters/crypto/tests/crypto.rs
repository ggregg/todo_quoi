//! Tests de l'adaptateur de chiffrement, à travers les ports `VaultKeys` et `Cipher`.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use tq_adapter_crypto::{Argon2VaultKeys, XChaChaCipher};
use tq_application::ports::{Cipher, KdfParams, VaultKeys};
use tq_application::vault::{Password, VaultError};

/// Dérivation volontairement faible : les tests restent rapides (la production utilise
/// `DEFAULT_KDF`, testé à part).
fn fast_keys() -> Argon2VaultKeys {
    Argon2VaultKeys::with_params(KdfParams {
        memory_kib: 64,
        iterations: 1,
        parallelism: 1,
    })
}

fn password(text: &str) -> Password {
    Password::new(text.to_string()).unwrap()
}

#[test]
fn a_vault_opens_with_its_password_and_gives_back_the_same_key() {
    let keys = fast_keys();
    let (record, key) = keys.create(&password("correct horse")).unwrap();
    let reopened = keys.unlock(&record, &password("correct horse")).unwrap();
    assert_eq!(reopened.expose(), key.expose());
}

#[test]
fn a_wrong_password_is_refused() {
    let keys = fast_keys();
    let (record, _) = keys.create(&password("correct horse")).unwrap();
    assert_eq!(
        keys.unlock(&record, &password("wrong horse")).unwrap_err(),
        VaultError::WrongPassword
    );
}

#[test]
fn two_vaults_never_share_key_salt_or_identifier() {
    let keys = fast_keys();
    let (first, first_key) = keys.create(&password("même mot de passe")).unwrap();
    let (second, second_key) = keys.create(&password("même mot de passe")).unwrap();
    assert_ne!(first.key_id, second.key_id);
    assert_ne!(first.salt, second.salt);
    assert_ne!(first_key.expose(), second_key.expose());
}

#[test]
fn an_envelope_moved_to_another_vault_does_not_open() {
    let keys = fast_keys();
    let (first, _) = keys.create(&password("correct horse")).unwrap();
    let (mut second, _) = keys.create(&password("correct horse")).unwrap();
    second.wrapped_key = first.wrapped_key.clone();
    second.salt = first.salt.clone();
    assert!(keys.unlock(&second, &password("correct horse")).is_err());
}

#[test]
fn production_parameters_still_open_a_vault() {
    let keys = Argon2VaultKeys::new();
    let (record, key) = keys.create(&password("correct horse")).unwrap();
    assert_eq!(record.kdf.memory_kib, 64 * 1024);
    // Un coffre s'ouvre avec les paramètres de **son** manifeste, quels que soient ceux du jour.
    let reopened = fast_keys()
        .unlock(&record, &password("correct horse"))
        .unwrap();
    assert_eq!(reopened.expose(), key.expose());
}

#[test]
fn sealing_round_trips_and_detects_any_change() {
    let (_, key) = fast_keys().create(&password("correct horse")).unwrap();
    let cipher = XChaChaCipher::new();
    let sealed = cipher.seal(&key, b"Note secrete", b"tache-1").unwrap();
    assert!(sealed.starts_with("xc20p1:"));
    assert_eq!(
        cipher.open(&key, &sealed, b"tache-1").unwrap(),
        b"Note secrete"
    );
    // Autre contexte : refusé (un chiffré recopié dans une autre tâche ne s'ouvre pas).
    assert!(cipher.open(&key, &sealed, b"tache-2").is_err());
    // Un seul caractère modifié : refusé.
    let mut tampered = sealed.clone();
    let last = tampered.pop().unwrap();
    tampered.push(if last == '0' { '1' } else { '0' });
    assert!(cipher.open(&key, &tampered, b"tache-1").is_err());
}

#[test]
fn the_same_text_sealed_twice_gives_two_different_results() {
    let (_, key) = fast_keys().create(&password("correct horse")).unwrap();
    let cipher = XChaChaCipher::new();
    let first = cipher.seal(&key, b"identique", b"ctx").unwrap();
    let second = cipher.seal(&key, b"identique", b"ctx").unwrap();
    assert_ne!(first, second, "nonce aléatoire à chaque chiffrement");
}
