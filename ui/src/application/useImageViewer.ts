/**
 * Visionneuse d'images plein écran (dans la fenêtre) : quelles images, laquelle est affichée.
 *
 * État **partagé** au niveau du module : une image de la note (vue de nœud TipTap) l'ouvre, et
 * le composant `ImageViewer.vue`, monté une fois par fenêtre, l'affiche.
 */
import { computed, ref, shallowRef } from 'vue'

export interface ViewerImage {
  /** Adresse affichable (`tq-media://…`). */
  url: string
  /** Texte alternatif / légende. */
  alt: string
}

const images = shallowRef<ViewerImage[]>([])
const index = ref(0)

/** Ouvre la visionneuse sur `images[start]` ; les autres sont accessibles par ← →. */
export function openImageViewer(list: ViewerImage[], start: number): void {
  if (list.length === 0) return
  images.value = list
  index.value = Math.min(Math.max(start, 0), list.length - 1)
}

export function useImageViewer() {
  const current = computed(() => images.value[index.value] ?? null)
  const count = computed(() => images.value.length)

  function close(): void {
    images.value = []
    index.value = 0
  }

  /** Image suivante (`step` = 1) ou précédente (`-1`), en boucle. */
  function go(step: number): void {
    if (count.value > 0) index.value = (index.value + step + count.value) % count.value
  }

  return { current, count, index, close, go }
}
