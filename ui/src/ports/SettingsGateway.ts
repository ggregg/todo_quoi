/**
 * Port : bases ouvertes sur cet appareil (spec §5.1bis et §7, ADR-0008 et ADR-0009). Ajouter ou
 * détacher une base recharge l'assemblage à chaud puis les fenêtres (`bases-changed`) ;
 * renommer ou changer la couleur s'applique directement.
 */
import type { InjectionKey } from 'vue'

export type DriveProvider = 'icloud' | 'googleDrive' | 'oneDrive' | 'dropbox'

/** Stockage d'une base, détecté par le cœur : une base peut être n'importe où. */
export type StorageKind = 'local' | 'external' | 'network' | DriveProvider

export interface DriveFolder {
  provider: DriveProvider
  path: string
}

/** Une base et son état. */
export interface BaseInfo {
  id: string
  name: string
  /** Couleur `#rrggbb`. */
  color: string
  path: string
  /** Stockage détecté (disque local, externe, réseau, ou drive cloud). */
  storage: StorageKind
  /** Raison si la base n'a pas pu être ouverte au démarrage (`null` : ouverte). */
  problem: string | null
}

export interface DataLocation {
  /** `environment` : variable TODOQUOI_DATA_DIR (une base imposée) ; `settings` : réglages. */
  kind: 'environment' | 'settings'
  /** Bases, dans l'ordre du bandeau. */
  bases: BaseInfo[]
  /** Vrai tant que la seule base est le dossier local de l'appareil (non synchronisé). */
  deviceOnly: boolean
  /** Drives détectés sur l'appareil. */
  drives: DriveFolder[]
}

/** Compression des images ajoutées aux notes. */
export interface ImageSettings {
  /** Qualité JPEG, en pourcentage (50 à 100). */
  quality: number
  /** Plus grand côté, en pixels. */
  maxSide: number
  /** `compress` : réduire ; `original` : garder l'image telle quelle. */
  handling: 'compress' | 'original'
}

/** Préférences de l'appareil (spec §5.7), enregistrées dans ses réglages, jamais dans une base. */
export interface Preferences {
  /** Nom de l'appareil (signe les modifications et les fichiers liés). */
  deviceName: string
  /** Identifiant qui en découle (`Mac de Greg` → `mac-de-greg`), calculé par le cœur. */
  deviceId: string
  /** Durée de la corbeille, en jours (1 à 365). */
  trashRetentionDays: number
  images: ImageSettings
}

export const DEFAULT_IMAGE_SETTINGS: ImageSettings = { quality: 85, maxSide: 2560, handling: 'compress' }

export interface SettingsGateway {
  getDataLocation(): Promise<DataLocation>
  /** Ajoute une base (n'importe quel dossier, ou un dossier `TodoQuoi` existant). */
  addBase(name: string, color: string, path: string): Promise<void>
  /** Renomme une base ou change sa couleur. */
  updateBase(id: string, name: string, color: string): Promise<void>
  /** Détache une base de l'app (le dossier n'est pas touché). */
  removeBase(id: string): Promise<void>
  getPreferences(): Promise<Preferences>
  /** Enregistre ; changer le nom de l'appareil ou la durée de la corbeille recharge les fenêtres. */
  setPreferences(preferences: Preferences): Promise<void>
}

export const settingsGatewayKey: InjectionKey<SettingsGateway> = Symbol('SettingsGateway')
