/**
 * Liens dans la note :
 * - **créer / modifier** (bouton lien ou `Mod+K`) : un petit formulaire « texte + adresse » ;
 *   sans sélection, le texte saisi (ou l'adresse) est inséré, déjà lié ;
 * - **coller** une adresse : elle devient un lien (extension Link, `linkOnPaste` / `autolink`) ;
 * - **ouvrir** : `Mod+clic` sur le lien, ou « Ouvrir » dans la barre qui apparaît quand le curseur
 *   est sur un lien. L'ouverture passe par le port `LinkGateway` (navigateur du système).
 */
import { computed, ref, type ShallowRef } from 'vue'
import type { Editor } from '@tiptap/core'
import type { LinkGateway } from '../../ports/LinkGateway'
import { normalizeLinkInput } from '../../application/linkRules'
import type { Notify } from './useNoteMedia'
import { t } from '../../i18n'

export function useNoteLinks(editor: ShallowRef<Editor | undefined>, links: LinkGateway, notify: Notify) {
  const editing = ref(false)
  const text = ref('')
  const href = ref('')

  /** Adresse du lien sous le curseur (`null` : pas de lien). */
  const currentHref = computed<string | null>(() => {
    const value = editor.value?.isActive('link') ? editor.value.getAttributes('link').href : null
    return typeof value === 'string' ? value : null
  })

  function start(): void {
    const current = editor.value
    if (!current) return
    if (current.isActive('link')) current.chain().extendMarkRange('link').run()
    const { from, to } = current.state.selection
    text.value = current.state.doc.textBetween(from, to, ' ')
    href.value = currentHref.value ?? ''
    editing.value = true
  }

  function apply(): void {
    const current = editor.value
    editing.value = false
    if (!current) return
    const address = normalizeLinkInput(href.value)
    const chain = current.chain().focus()
    if (address === '') return void chain.extendMarkRange('link').unsetLink().run()
    const label = text.value.trim() || address
    const { from, to } = current.state.selection
    const selected = current.state.doc.textBetween(from, to, ' ')
    if (from !== to && selected === label) return void chain.setLink({ href: address }).run()
    chain
      .insertContent({ type: 'text', text: label, marks: [{ type: 'link', attrs: { href: address } }] })
      .unsetMark('link')
      .insertContent(from === to ? ' ' : '')
      .run()
  }

  function cancel(): void {
    editing.value = false
    editor.value?.commands.focus()
  }

  function remove(): void {
    editor.value?.chain().focus().extendMarkRange('link').unsetLink().run()
  }

  async function open(address: string | null = currentHref.value): Promise<void> {
    if (!address) return
    try {
      await links.open(address)
    } catch (error) {
      const message = typeof error === 'object' && error && 'message' in error ? String(error.message) : String(error)
      notify('error', t('editor.linkOpenError', { message }))
    }
  }

  /** `Mod+clic` sur un lien de la note : l'ouvrir (à brancher sur `editorProps.handleClick`). */
  function handleClick(event: MouseEvent): boolean {
    const anchor = (event.target as HTMLElement | null)?.closest('a')
    if (!anchor || !(event.metaKey || event.ctrlKey)) return false
    event.preventDefault()
    void open(anchor.getAttribute('href'))
    return true
  }

  return { editing, text, href, currentHref, start, apply, cancel, remove, open, handleClick }
}
