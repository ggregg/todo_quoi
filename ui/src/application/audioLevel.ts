/**
 * Niveau du micro pendant un enregistrement (retour visuel : le micro capte-t-il quelque chose ?).
 * Fonctions pures ; la mesure elle-même (Web Audio `AnalyserNode`) est dans `useAudioRecorder`.
 */

/** Seuil sous lequel on considère qu'il n'y a pas de son (niveau 0 – 1). */
export const SILENCE_LEVEL = 0.02
/** Durée de silence continu avant d'avertir (ms). */
export const SILENCE_WARNING_MS = 2500

/**
 * Niveau 0 – 1 d'un échantillon temporel d'`AnalyserNode.getByteTimeDomainData` (octets centrés
 * sur 128). Moyenne quadratique, amplifiée pour qu'une voix normale remplisse bien la jauge.
 */
export function levelOf(samples: ArrayLike<number>): number {
  if (samples.length === 0) return 0
  let sum = 0
  for (let index = 0; index < samples.length; index++) {
    const centered = ((samples[index] ?? 128) - 128) / 128
    sum += centered * centered
  }
  return Math.min(1, Math.sqrt(sum / samples.length) * 4)
}

/** Surveille le silence : `true` après `SILENCE_WARNING_MS` sans dépasser `SILENCE_LEVEL`. */
export class SilenceWatch {
  private lastSound: number

  constructor(startedAt: number) {
    this.lastSound = startedAt
  }

  /** Nouveau niveau mesuré à l'instant `now` (ms) ; renvoie vrai si c'est le silence depuis trop longtemps. */
  update(level: number, now: number): boolean {
    if (level > SILENCE_LEVEL) this.lastSound = now
    return now - this.lastSound >= SILENCE_WARNING_MS
  }
}
