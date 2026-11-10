/**
 * Alignement et couleur du texte des notes (ADR-0012) : deux petites extensions TipTap écrites
 * pour TodoQuoi (pas de dépendance, architecture §16), au plus près de ce que le Markdown
 * étendu sait enregistrer (crate `note_markdown`, module `formatting`) :
 * - `textAlign` : attribut des paragraphes, titres et cellules de tableau ;
 * - `textColor` : marque `<span data-color="…">`, couleurs de la palette seulement.
 */
import { Extension, Mark } from '@tiptap/core'

/** Alignements proposés (`null` = à gauche, par défaut). */
export const TEXT_ALIGNMENTS = ['left', 'center', 'right'] as const
export type TextAlignment = (typeof TEXT_ALIGNMENTS)[number]

/** Palette (miroir de `TEXT_COLORS` en Rust) ; teintes : variables `--text-<nom>` du thème. */
export const TEXT_COLORS = ['red', 'orange', 'green', 'blue', 'purple', 'gray'] as const
export type TextColor = (typeof TEXT_COLORS)[number]

/** Blocs qui peuvent être alignés. */
export const ALIGNABLE = ['paragraph', 'heading', 'tableCell', 'tableHeader'] as const

export const TextAlign = Extension.create({
  name: 'textAlign',
  addGlobalAttributes() {
    return [
      {
        types: [...ALIGNABLE],
        attributes: {
          textAlign: {
            default: null,
            parseHTML: (element) =>
              (TEXT_ALIGNMENTS as readonly string[]).find((a) => a === element.style.textAlign) ?? null,
            renderHTML: (attributes) => (attributes.textAlign ? { style: `text-align: ${attributes.textAlign}` } : {}),
          },
        },
      },
    ]
  },
})

export const TextColorMark = Mark.create({
  name: 'textColor',
  addAttributes() {
    return {
      color: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-color'),
        renderHTML: (attributes) => ({ 'data-color': attributes.color }),
      },
    }
  },
  parseHTML() {
    return [{ tag: 'span[data-color]' }]
  },
  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0]
  },
})
