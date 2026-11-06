/**
 * Schéma de l'éditeur TipTap — **contrat** avec la conversion Rust `note_markdown` (ADR-0007) :
 * noms de nœuds et d'attributs identiques. Validé au POC I0-1 (`poc/i0-1-markdown`).
 *
 * I2 : texte, titres, listes, cases à cocher, liens, code, citations, tableaux.
 * Les blocs maison (radio, audio, pièce jointe) et les images sont **affichés** pour qu'aucune
 * note existante ne perde de contenu ; leur insertion et leur édition arrivent en I3.
 */
import { Node, mergeAttributes, type AnyExtension } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import { Table, TableRow, TableHeader, TableCell } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'

/** Groupe radio (`:::radio{label="…"}`) — affichage seul en I2. */
const RadioGroup = Node.create({
  name: 'radioGroup',
  group: 'block',
  content: 'radioOption+',
  addAttributes: () => ({ label: { default: null } }),
  parseHTML: () => [{ tag: 'fieldset[data-radio-group]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'fieldset',
    mergeAttributes(HTMLAttributes, { 'data-radio-group': '', class: 'radio-group' }),
    ['legend', { contenteditable: 'false' }, node.attrs.label ?? ''],
    ['div', 0],
  ],
})

/** Option d'un groupe radio : texte simple. */
const RadioOption = Node.create({
  name: 'radioOption',
  content: 'text*',
  marks: '',
  defining: true,
  addAttributes: () => ({ checked: { default: false } }),
  parseHTML: () => [{ tag: 'label[data-radio-option]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'label',
    mergeAttributes(HTMLAttributes, { 'data-radio-option': '', class: 'radio-option' }),
    ['span', { contenteditable: 'false', class: node.attrs.checked ? 'radio on' : 'radio' }],
    ['span', 0],
  ],
})

/** Message audio (`::audio{…}`) — affichage seul en I2 (lecteur en I3). */
const AudioBlock = Node.create({
  name: 'audio',
  group: 'block',
  atom: true,
  addAttributes: () => ({ src: { default: null }, duration: { default: null }, mime: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-audio]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'div',
    mergeAttributes(HTMLAttributes, { 'data-audio': '', class: 'media-placeholder' }),
    `🎙 Message audio${node.attrs.duration ? ` · ${Math.round(Number(node.attrs.duration))} s` : ''}`,
  ],
})

/** Pièce jointe (`::file{…}`) — affichage seul en I2. */
const Attachment = Node.create({
  name: 'attachment',
  group: 'block',
  atom: true,
  addAttributes: () => ({ src: { default: null }, name: { default: null }, size: { default: null } }),
  parseHTML: () => [{ tag: 'div[data-attachment]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'div',
    mergeAttributes(HTMLAttributes, { 'data-attachment': '', class: 'media-placeholder' }),
    `📎 ${node.attrs.name ?? 'Pièce jointe'}`,
  ],
})

/** Image en bloc, avec largeur / hauteur facultatives. */
const BlockImage = Image.extend({
  addAttributes() {
    return { ...this.parent?.(), width: { default: null }, height: { default: null } }
  },
}).configure({ inline: false })

export const noteExtensions: AnyExtension[] = [
  // Le soulignement n'a pas d'équivalent Markdown : désactivé pour ne rien perdre (POC I0-1).
  StarterKit.configure({ underline: false, link: { openOnClick: false, autolink: true } }),
  BlockImage,
  Table.configure({ resizable: false }),
  TableRow,
  TableHeader,
  TableCell,
  TaskList,
  TaskItem.configure({ nested: true }),
  RadioGroup,
  RadioOption,
  AudioBlock,
  Attachment,
]
