/**
 * Schéma de l'éditeur TipTap — **contrat** avec la conversion Rust `note_markdown` (ADR-0007) :
 * noms de nœuds et d'attributs identiques. Validé au POC I0-1 (`poc/i0-1-markdown`).
 *
 * I2 : texte, titres, listes, cases à cocher, liens, code, citations, tableaux.
 * I3 : images (affichées via `tq-media://`, redimensionnables), messages audio, pièces jointes,
 *      groupes radio éditables (un seul choix, sélection au clic).
 */
import { Node, Extension, mergeAttributes, type AnyExtension } from '@tiptap/core'
import { Plugin } from '@tiptap/pm/state'
import StarterKit from '@tiptap/starter-kit'
import Image, { type ImageOptions } from '@tiptap/extension-image'
import { Table, TableRow, TableHeader, TableCell } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { VueNodeViewRenderer } from '@tiptap/vue-3'
import type { MediaGateway } from '../../ports/MediaGateway'
import { t } from '../../i18n'
import AudioView from './nodes/AudioView.vue'
import AttachmentView from './nodes/AttachmentView.vue'
import ImageView from './nodes/ImageView.vue'
import RadioGroupView from './nodes/RadioGroupView.vue'

/** Groupe radio (`:::radio{label="…"}`), affiché par `RadioGroupView.vue`. */
const RadioGroup = Node.create({
  name: 'radioGroup',
  group: 'block',
  content: 'radioOption+',
  addAttributes: () => ({ label: { default: null } }),
  parseHTML: () => [{ tag: 'fieldset[data-radio-group]' }],
  renderHTML: ({ HTMLAttributes }) => ['fieldset', mergeAttributes(HTMLAttributes, { 'data-radio-group': '' }), 0],
  addNodeView: () => VueNodeViewRenderer(RadioGroupView),
})

/** Option d'un groupe radio : texte simple ; `checked` n'est pas recopié quand on fait `Entrée`. */
const RadioOption = Node.create({
  name: 'radioOption',
  content: 'text*',
  marks: '',
  defining: true,
  addAttributes: () => ({ checked: { default: false, keepOnSplit: false } }),
  parseHTML: () => [{ tag: 'label[data-radio-option]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'label',
    mergeAttributes(HTMLAttributes, { 'data-radio-option': '', class: 'radio-option' }),
    ['span', { contenteditable: 'false', class: node.attrs.checked ? 'radio on' : 'radio', 'data-radio-dot': '' }],
    ['span', 0],
  ],
})

/**
 * Clic sur le rond d'une option : elle devient la seule cochée de son groupe.
 * (Règle d'interface : un groupe radio est un choix unique, par définition.)
 */
const RadioSelection = Extension.create({
  name: 'radioSelection',
  addProseMirrorPlugins: () => [
    new Plugin({
      props: {
        handleClickOn(view, _pos, node, nodePos, event) {
          const isDot = (event.target as HTMLElement | null)?.hasAttribute('data-radio-dot')
          if (!isDot || node.type.name !== 'radioOption') return false
          const group = view.state.doc.resolve(nodePos)
          const groupStart = group.start()
          const transaction = view.state.tr
          group.parent.forEach((option, offset) => {
            const checked = groupStart + offset === nodePos
            if (option.attrs.checked !== checked) {
              transaction.setNodeMarkup(groupStart + offset, undefined, { ...option.attrs, checked })
            }
          })
          view.dispatch(transaction)
          return true
        },
      },
    }),
  ],
})

/**
 * Les clics sur les commandes d'un bloc (lecteur audio, boutons) vont à la commande, pas à
 * l'éditeur. Sans cela, ProseMirror prend le premier clic pour sélectionner (ou glisser) le bloc
 * et le lecteur ne démarre qu'au second clic.
 */
const leaveControlsToTheBrowser = ({ event }: { event: Event }): boolean =>
  (event.target as HTMLElement | null)?.closest('audio, video, button, input, select, a[href]') != null

/** Options partagées par les blocs médias : la passerelle qui sait afficher / ouvrir un média. */
interface MediaOptions {
  media: MediaGateway | null
}

/** Message audio (`::audio{src duration mime}`), affiché par `AudioView.vue`. */
const AudioBlock = Node.create<MediaOptions>({
  name: 'audio',
  group: 'block',
  atom: true,
  draggable: true,
  addOptions: () => ({ media: null }),
  addAttributes: () => ({ src: { default: null }, duration: { default: null }, mime: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-audio]' }],
  renderHTML: ({ HTMLAttributes }) => ['div', mergeAttributes(HTMLAttributes, { 'data-audio': '' })],
  addNodeView: () => VueNodeViewRenderer(AudioView, { stopEvent: leaveControlsToTheBrowser }),
})

/** Pièce jointe (`::file{src name size}`), affichée par `AttachmentView.vue`. */
const Attachment = Node.create<MediaOptions>({
  name: 'attachment',
  group: 'block',
  atom: true,
  draggable: true,
  addOptions: () => ({ media: null }),
  addAttributes: () => ({ src: { default: null }, name: { default: null }, size: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-attachment]' }],
  renderHTML: ({ HTMLAttributes }) => ['div', mergeAttributes(HTMLAttributes, { 'data-attachment': '' })],
  addNodeView: () => VueNodeViewRenderer(AttachmentView, { stopEvent: leaveControlsToTheBrowser }),
})

/**
 * Image en bloc, redimensionnable (`ImageView.vue`), avec largeur / hauteur facultatives.
 * Dans la note, `src` reste le lien relatif `../media/<nom>` ; seul l'affichage le traduit en
 * URL `tq-media://`.
 */
function blockImage(media: MediaGateway) {
  return Image.extend<ImageOptions & MediaOptions>({
    addOptions() {
      return { ...(this.parent?.() as ImageOptions), media }
    },
    addAttributes() {
      return { ...this.parent?.(), width: { default: null }, height: { default: null } }
    },
    renderHTML({ HTMLAttributes }) {
      const src = typeof HTMLAttributes.src === 'string' ? media.url(HTMLAttributes.src) : HTMLAttributes.src
      return ['img', mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, { src })]
    },
    addNodeView: () => VueNodeViewRenderer(ImageView),
  }).configure({ inline: false })
}

/** Toutes les extensions de l'éditeur de note, branchées sur la passerelle des médias. */
export function createNoteExtensions(media: MediaGateway): AnyExtension[] {
  return [
    // Le soulignement n'a pas d'équivalent Markdown : désactivé pour ne rien perdre (POC I0-1).
    // Liens : ouverts par `Mod+clic` ou la barre de lien (useNoteLinks), jamais par la webview.
    // Une adresse collée ou tapée devient un lien ; `www.x.fr` est complété en `https://`.
    StarterKit.configure({
      underline: false,
      link: {
        openOnClick: false,
        autolink: true,
        linkOnPaste: true,
        defaultProtocol: 'https',
        HTMLAttributes: { title: t('editor.linkHint'), rel: 'noopener noreferrer', target: null },
      },
    }),
    blockImage(media),
    Table.configure({ resizable: false }),
    TableRow,
    TableHeader,
    TableCell,
    TaskList,
    TaskItem.configure({ nested: true }),
    RadioGroup,
    RadioOption,
    RadioSelection,
    AudioBlock.configure({ media }),
    Attachment.configure({ media }),
  ]
}
