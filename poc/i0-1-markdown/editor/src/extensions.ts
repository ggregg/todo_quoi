/**
 * Extensions TipTap du POC : celles de TipTap + nos blocs maison.
 *
 * Le schéma défini ici doit accepter tel quel le JSON produit par le convertisseur Rust
 * (`mdconv to-json`). Les noms de nœuds et d'attributs sont donc le contrat entre les deux.
 */
import { Node, mergeAttributes, type AnyExtension } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Image from '@tiptap/extension-image'
import { Table, TableRow, TableHeader, TableCell } from '@tiptap/extension-table'
import { TaskList, TaskItem } from '@tiptap/extension-list'

/** Groupe radio : un seul choix parmi N (`:::radio{label="…"}`). */
export const RadioGroup = Node.create({
  name: 'radioGroup',
  group: 'block',
  content: 'radioOption+',
  addAttributes: () => ({ label: { default: null } }),
  parseHTML: () => [{ tag: 'fieldset[data-radio-group]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'fieldset',
    mergeAttributes(HTMLAttributes, { 'data-radio-group': '' }),
    ['legend', { contenteditable: 'false' }, node.attrs.label ?? ''],
    ['div', 0],
  ],
})

/** Une option d'un groupe radio : texte simple, sans mise en forme. */
export const RadioOption = Node.create({
  name: 'radioOption',
  content: 'text*',
  marks: '',
  defining: true,
  addAttributes: () => ({ checked: { default: false } }),
  parseHTML: () => [{ tag: 'label[data-radio-option]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'label',
    mergeAttributes(HTMLAttributes, { 'data-radio-option': '' }),
    ['input', { type: 'radio', disabled: 'true', ...(node.attrs.checked ? { checked: 'true' } : {}) }],
    ['span', 0],
  ],
})

/** Message audio (`::audio{src=… duration=… mime=…}`) : bloc atomique, non éditable. */
export const AudioBlock = Node.create({
  name: 'audio',
  group: 'block',
  atom: true,
  addAttributes: () => ({ src: { default: null }, duration: { default: null }, mime: { default: null } }),
  parseHTML: () => [{ tag: 'audio[data-audio]' }],
  renderHTML: ({ HTMLAttributes }) => ['audio', mergeAttributes(HTMLAttributes, { 'data-audio': '', controls: 'true' })],
})

/** Pièce jointe (`::file{src=… name=… size=…}`). */
export const Attachment = Node.create({
  name: 'attachment',
  group: 'block',
  atom: true,
  addAttributes: () => ({ src: { default: null }, name: { default: null }, size: { default: null } }),
  parseHTML: () => [{ tag: 'a[data-attachment]' }],
  renderHTML: ({ node, HTMLAttributes }) => [
    'a',
    mergeAttributes(HTMLAttributes, { 'data-attachment': '', href: node.attrs.src }),
    `📎 ${node.attrs.name ?? ''}`,
  ],
})

/** Image en bloc, avec largeur / hauteur facultatives (`{width=480}`). */
const BlockImage = Image.extend({
  addAttributes() {
    return { ...this.parent?.(), width: { default: null }, height: { default: null } }
  },
}).configure({ inline: false })

export const extensions: AnyExtension[] = [
  // Le soulignement n'a pas d'équivalent Markdown : désactivé pour ne rien perdre.
  StarterKit.configure({ underline: false, link: { openOnClick: false } }),
  BlockImage,
  Table,
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
