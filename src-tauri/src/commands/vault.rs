//! Commandes du coffre (spec §9, ADR-0010) : un coffre par base.

use super::run;
use crate::auto_lock::SharedAutoLock;
use crate::composition::LiveState;
use crate::dto::{parse_base_id, parse_task_id};
use crate::error::UiError;
use crate::events::notify_vaults_changed;
use crate::plain_copies;
use crate::settings::LocalSettings;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tq_application::use_cases::encrypt_task::EncryptScope;
use tq_application::vault::{AutoLockDelay, Password, Remember, VaultState};

/// État du coffre d'une base. JSON : `{ "base": "pro", "state": "locked" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseVaultDto {
    /// Identifiant de la base.
    pub base: String,
    /// `none`, `locked` ou `unlocked`.
    pub state: &'static str,
}

/// Que faire des sous-tâches. JSON : `"taskOnly"` ou `"withSubtasks"`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EncryptScopeDto {
    /// Seulement la tâche.
    TaskOnly,
    /// Avec ses sous-tâches.
    WithSubtasks,
}

fn state_name(state: &VaultState) -> &'static str {
    match state {
        VaultState::None => "none",
        VaultState::Locked => "locked",
        VaultState::Unlocked => "unlocked",
    }
}

fn remember(on_this_device: bool) -> Remember {
    // Case à cocher de l'interface → énumération (pas de booléen dans l'application).
    if on_this_device {
        Remember::OnThisDevice
    } else {
        Remember::NotOnThisDevice
    }
}

fn password(text: String) -> Result<Password, UiError> {
    Password::new(text).map_err(|error| UiError::from(tq_application::AppError::Vault(error)))
}

/// État des coffres. `invoke('list_vaults')`.
#[tauri::command]
pub async fn list_vaults(state: State<'_, LiveState>) -> Result<Vec<BaseVaultDto>, UiError> {
    let use_case = state.current().vault.list.clone();
    run(move || {
        Ok(use_case
            .execute()?
            .into_iter()
            .map(|vault| BaseVaultDto {
                base: vault.base.to_string(),
                state: state_name(&vault.state),
            })
            .collect())
    })
    .await
}

/// Crée le coffre d'une base. `invoke('create_vault', { base, password, remember })`.
#[tauri::command]
pub async fn create_vault(
    app: AppHandle,
    state: State<'_, LiveState>,
    base: String,
    password: String,
    remember: bool,
) -> Result<(), UiError> {
    let (base, password, remember) = (
        parse_base_id(&base)?,
        self::password(password)?,
        self::remember(remember),
    );
    let use_case = state.current().vault.create.clone();
    run(move || use_case.execute(&base, &password, remember)).await?;
    notify_vaults_changed(&app);
    Ok(())
}

/// Déverrouille le coffre d'une base. `invoke('unlock_vault', { base, password, remember })`.
#[tauri::command]
pub async fn unlock_vault(
    app: AppHandle,
    state: State<'_, LiveState>,
    base: String,
    password: String,
    remember: bool,
) -> Result<(), UiError> {
    let (base, password, remember) = (
        parse_base_id(&base)?,
        self::password(password)?,
        self::remember(remember),
    );
    let use_case = state.current().vault.unlock.clone();
    run(move || use_case.execute(&base, &password, remember)).await?;
    notify_vaults_changed(&app);
    Ok(())
}

/// Verrouille le coffre d'une base ; les copies déchiffrées des pièces jointes ouvertes sont
/// effacées. `invoke('lock_vault', { base })`.
#[tauri::command]
pub async fn lock_vault(
    app: AppHandle,
    state: State<'_, LiveState>,
    base: String,
) -> Result<(), UiError> {
    let base = parse_base_id(&base)?;
    let use_case = state.current().vault.lock.clone();
    run(move || {
        use_case.execute(&base);
        plain_copies::clear();
        Ok(())
    })
    .await?;
    notify_vaults_changed(&app);
    Ok(())
}

/// Change le mot de passe du coffre d'une base.
/// `invoke('change_vault_password', { base, current, next })`.
#[tauri::command]
pub async fn change_vault_password(
    state: State<'_, LiveState>,
    base: String,
    current: String,
    next: String,
) -> Result<(), UiError> {
    let (base, current, next) = (parse_base_id(&base)?, password(current)?, password(next)?);
    let use_case = state.current().vault.change_password.clone();
    run(move || use_case.execute(&base, &current, &next)).await
}

/// L'utilisateur agit (frappe, clic) : le délai du verrouillage automatique repart de zéro.
/// `invoke('vault_activity')`, au plus une fois toutes les 30 s par fenêtre.
#[tauri::command]
pub fn vault_activity(auto_lock: State<'_, SharedAutoLock>) {
    auto_lock.0.record_activity();
}

/// Délai du verrouillage automatique, en minutes (`0` : jamais). `invoke('get_vault_auto_lock')`.
#[tauri::command]
pub fn get_vault_auto_lock(auto_lock: State<'_, SharedAutoLock>) -> u32 {
    auto_lock.0.delay().minutes()
}

/// Change le délai et l'enregistre dans les réglages de l'appareil.
/// `invoke('set_vault_auto_lock', { minutes })` (`0` : jamais).
#[tauri::command]
pub fn set_vault_auto_lock(
    app: AppHandle,
    auto_lock: State<'_, SharedAutoLock>,
    minutes: u32,
) -> Result<(), UiError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| UiError::internal(error.to_string()))?;
    let settings = LocalSettings {
        vault_auto_lock_minutes: Some(minutes),
        ..LocalSettings::load(&config_dir)
    };
    settings
        .save(&config_dir)
        .map_err(|error| UiError::internal(format!("réglage non enregistré : {error}")))?;
    auto_lock.0.set_delay(AutoLockDelay::from_minutes(minutes));
    Ok(())
}

/// Chiffre une tâche. `invoke('encrypt_task', { taskId, scope: 'withSubtasks' })` ; renvoie les
/// tâches chiffrées.
#[tauri::command]
pub async fn encrypt_task(
    state: State<'_, LiveState>,
    task_id: String,
    scope: EncryptScopeDto,
) -> Result<Vec<String>, UiError> {
    let id = parse_task_id(&task_id)?;
    let scope = match scope {
        EncryptScopeDto::TaskOnly => EncryptScope::TaskOnly,
        EncryptScopeDto::WithSubtasks => EncryptScope::WithSubtasks,
    };
    let use_case = state.current().vault.encrypt.clone();
    run(move || {
        Ok(use_case
            .execute(&id, scope)?
            .iter()
            .map(ToString::to_string)
            .collect())
    })
    .await
}

/// Remet une tâche en clair. `invoke('decrypt_task', { taskId })`.
#[tauri::command]
pub async fn decrypt_task(state: State<'_, LiveState>, task_id: String) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.current().vault.decrypt.clone();
    run(move || use_case.execute(&id)).await
}
