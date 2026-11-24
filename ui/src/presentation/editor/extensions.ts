/**
 * Schéma de l'éditeur TipTap — **contrat** avec la conversion Rust `note_markdown` (ADR-0007) :
 * noms de nœuds et d'attributs identiques. Validé au POC I0-1 (`poc/i0-1-markdown`).
 *
 * I2 : texte, titres, listes, cases à cocher, liens, code, citations, tableaux.
 * I3 : images (affichées via `tq-media://`, redimensionnables), messages audio, pièces jointes.
 * Ensuite : fichiers liés (ADR-0011), alignement et couleur du texte (ADR-0012), taille du texte
 * (ADR-0013) dans `formatting.ts`. Les groupes de choix ont été retirés (ADR-0014) : le Rust relit
 * les anciens comme une liste de cases.
 */
import { Node, mergeAttributes, type AnyExtension } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Image, { type ImageOptions } from '@tiptap/extension-image'
import { Table, TableRow, TableHeader, TableCell } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'
import { VueNodeViewRenderer } from '@tiptap/vue-3'
import type { MediaGateway } from '../../ports/MediaGateway'
import { t } from '../../i18n'
import AudioView from './nodes/AudioView.vue'
import AttachmentView from './nodes/AttachmentView.vue'
import FileLinkView from './nodes/FileLinkView.vue'
import type { FileAttachments } from '../../ports/FileAttachments'
import ImageView from './nodes/ImageView.vue'
import { TextAlign, TextColorMark, TextSizeMark } from './formatting'

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

/** Fichier lié (`::filelink{path name device}`), affiché par `FileLinkView.vue`. */
const FileLink = Node.create<{ files: FileAttachments | null }>({
  name: 'fileLink',
  group: 'block',
  atom: true,
  draggable: true,
  addOptions: () => ({ files: null }),
  addAttributes: () => ({ path: { default: null }, name: { default: null }, device: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-file-link]' }],
  renderHTML: ({ HTMLAttributes }) => ['div', mergeAttributes(HTMLAttributes, { 'data-file-link': '' })],
  addNodeView: () => VueNodeViewRenderer(FileLinkView, { stopEvent: leaveControlsToTheBrowser }),
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

/** Toutes les extensions de l'éditeur de note, branchées sur les passerelles des médias et des fichiers. */
export function createNoteExtensions(media: MediaGateway, files: FileAttachments): AnyExtension[] {
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
    TextAlign,
    TextColorMark,
    TextSizeMark,
    TaskList,
    TaskItem.configure({ nested: true }),
    AudioBlock.configure({ media }),
    Attachment.configure({ media }),
    FileLink.configure({ files }),
  ]
}
