<script setup lang="ts">
/**
 * Éditeur de note TipTap (WYSIWYG, raccourcis Markdown à la frappe : `# `, `- `, `[] `, ``` …).
 * Il émet `update` à chaque modification ; la sauvegarde (débounce) est faite par `useTaskDetail`.
 * Quand `version` change, le contenu est remplacé par `content` (rechargement depuis le cœur)
 * sans émettre de modification.
 *
 * Médias (I3) : coller / glisser une image ou un fichier (copie), joindre un fichier choisi dans
 * la fenêtre native (copier, déplacer ou lier), enregistrer un audio, radio.
 */
import { inject, onBeforeUnmount, ref, watch } from 'vue'
import type { JSONContent } from '@tiptap/core'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import type { NoteDocument } from '../../domain/task'
import { t } from '../../i18n'
import { mediaGatewayKey } from '../../ports/MediaGateway'
import { formatDuration } from '../../application/mediaRules'
import { createNoteExtensions } from './extensions'
import AppIcon, { type IconName } from '../AppIcon.vue'
import { useNoteMedia } from './useNoteMedia'
import { useNoteLinks } from './useNoteLinks'
import { useNoteTable } from './useNoteTable'
import TableControls from './TableControls.vue'
import FormatMenus from './FormatMenus.vue'
import { useNoteFormat } from './useNoteFormat'
import { linkGatewayKey } from '../../ports/LinkGateway'
import { fileAttachmentsKey } from '../../ports/FileAttachments'
import { confirmKey } from '../../application/useConfirm'
import { useFileAttach } from '../../application/useFileAttach'

/** `taskId` : la tâche dont c'est la note (ses médias vont dans sa base). */
const props = defineProps<{ content: NoteDocument; version: number; taskId: string }>()
const emit = defineEmits<{
  update: [note: NoteDocument]
  blur: []
  notify: [kind: 'info' | 'error', message: string]
}>()

const media = inject(mediaGatewayKey)
if (!media) throw new Error('MediaGateway non fourni (voir main.ts)')
const links = inject(linkGatewayKey)
if (!links) throw new Error('LinkGateway non fourni (voir main.ts)')
const files = inject(fileAttachmentsKey)
if (!files) throw new Error('FileAttachments non fourni (voir main.ts)')
const confirm = inject(confirmKey)
if (!confirm) throw new Error('Confirm non fourni (voir App.vue / TaskWindow.vue)')

/** Le domaine ne dépend pas de TipTap : la conversion de type se fait ici, à sa frontière. */
const asTipTap = (note: NoteDocument) => note as JSONContent

const editor = useEditor({
  extensions: createNoteExtensions(media, files),
  content: asTipTap(props.content),
  onUpdate: ({ editor }) => emit('update', editor.getJSON() as NoteDocument),
  onBlur: () => emit('blur'),
  editorProps: {
    handleKeyDown: (_view, event) => {
      // Mod+K : saisir un lien.
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        startLink()
        return true
      }
      return false
    },
    // Mod+clic sur un lien : l'ouvrir dans le navigateur du système.
    handleClick: (_view, _pos, event) => noteLinks.handleClick(event),
    // Coller une image ou un fichier (copie d'écran, fichier copié dans le Finder…).
    handlePaste: (_view, event) => {
      const files = [...(event.clipboardData?.files ?? [])]
      if (files.length === 0) return false
      event.preventDefault()
      void noteMedia.insertFiles(files)
      return true
    },
    // Glisser-déposer des fichiers depuis le bureau, à l'endroit où on les lâche.
    handleDrop: (view, event, _slice, moved) => {
      const files = [...(event.dataTransfer?.files ?? [])]
      if (moved || files.length === 0) return false
      event.preventDefault()
      const position = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos
      void noteMedia.insertFiles(files, position)
      return true
    },
  },
})

const notify = (kind: 'info' | 'error', message: string) => emit('notify', kind, message)
const noteMedia = useNoteMedia(editor, media, notify, () => props.taskId)
const noteLinks = useNoteLinks(editor, links, notify)
const noteTable = useNoteTable(editor)
const noteFormat = useNoteFormat(editor)
/** Zone de note : les commandes des tableaux s'y positionnent. */
const contentArea = ref<HTMLElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

const fileAttach = useFileAttach(files, confirm, () => props.taskId)

/** 📎 : fenêtre native (copier / déplacer / lier) ; sans elle (navigateur), le champ fichier. */
async function attachFiles(): Promise<void> {
  const result = await fileAttach.pickAndAttach()
  if (result === null) return fileInput.value?.click()
  const at = editor.value?.state.selection.to
  if (result.nodes.length > 0 && at !== undefined) {
    editor.value?.chain().focus().insertContentAt(at, result.nodes).run()
  }
  if (result.failures.length > 0) notify('error', t('attach.failed', { details: result.failures.join(' · ') }))
}

function onFilesPicked(event: Event): void {
  const input = event.target as HTMLInputElement
  void noteMedia.insertFiles([...(input.files ?? [])])
  input.value = ''
}

watch(
  () => props.version,
  () => editor.value?.commands.setContent(asTipTap(props.content), { emitUpdate: false }),
)

onBeforeUnmount(() => editor.value?.destroy())

/** Un clic dans la zone de note, sous le texte, place le curseur à la fin (grande zone cliquable). */
function focusAtEnd(event: MouseEvent): void {
  if ((event.target as HTMLElement).closest('.ProseMirror')) return
  event.preventDefault()
  editor.value?.commands.focus('end')
}

// ----- Liens (voir useNoteLinks) -----------------------------------------------------------------

const linkFields = ref<HTMLElement | null>(null)
function startLink(): void {
  noteLinks.start()
  setTimeout(() => linkFields.value?.querySelector<HTMLInputElement>('input[name="href"]')?.focus())
}

// ----- Barre d'outils ------------------------------------------------------------------------

/** Un bouton : une icône, ou un libellé typographique (G, I, S, H1, H2). */
type Command = { icon?: IconName; label?: string; title: string; active?: () => boolean; run: () => void }

function chain() {
  return editor.value!.chain().focus()
}

const commands: Command[] = [
  {
    label: 'G',
    title: t('editor.bold'),
    active: () => !!editor.value?.isActive('bold'),
    run: () => chain().toggleBold().run(),
  },
  {
    label: 'I',
    title: t('editor.italic'),
    active: () => !!editor.value?.isActive('italic'),
    run: () => chain().toggleItalic().run(),
  },
  {
    label: 'S',
    title: t('editor.strike'),
    active: () => !!editor.value?.isActive('strike'),
    run: () => chain().toggleStrike().run(),
  },
  {
    icon: 'code',
    title: t('editor.code'),
    active: () => !!editor.value?.isActive('code'),
    run: () => chain().toggleCode().run(),
  },
  {
    label: 'H1',
    title: t('editor.heading', { level: 1 }),
    active: () => !!editor.value?.isActive('heading', { level: 1 }),
    run: () => chain().toggleHeading({ level: 1 }).run(),
  },
  {
    label: 'H2',
    title: t('editor.heading', { level: 2 }),
    active: () => !!editor.value?.isActive('heading', { level: 2 }),
    run: () => chain().toggleHeading({ level: 2 }).run(),
  },
  {
    icon: 'bulletList',
    title: t('editor.bulletList'),
    active: () => !!editor.value?.isActive('bulletList'),
    run: () => chain().toggleBulletList().run(),
  },
  {
    icon: 'orderedList',
    title: t('editor.orderedList'),
    active: () => !!editor.value?.isActive('orderedList'),
    run: () => chain().toggleOrderedList().run(),
  },
  {
    icon: 'taskList',
    title: t('editor.taskList'),
    active: () => !!editor.value?.isActive('taskList'),
    run: () => chain().toggleTaskList().run(),
  },
  {
    icon: 'quote',
    title: t('editor.quote'),
    active: () => !!editor.value?.isActive('blockquote'),
    run: () => chain().toggleBlockquote().run(),
  },
  {
    icon: 'codeBlock',
    title: t('editor.codeBlock'),
    active: () => !!editor.value?.isActive('codeBlock'),
    run: () => chain().toggleCodeBlock().run(),
  },
  {
    icon: 'table',
    title: t('editor.table'),
    run: () => chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  { icon: 'link', title: t('editor.link'), active: () => !!editor.value?.isActive('link'), run: startLink },
  {
    icon: 'radio',
    title: t('editor.radio'),
    active: () => !!editor.value?.isActive('radioGroup'),
    run: noteMedia.insertRadioGroup,
  },
  { icon: 'paperclip', title: t('editor.attach'), run: () => void attachFiles() },
  {
    icon: 'mic',
    title: t('editor.record'),
    active: () => noteMedia.recorder.state.value !== 'idle',
    run: () => void noteMedia.toggleRecording(),
  },
  { icon: 'undo', title: t('editor.undo'), run: () => chain().undo().run() },
  { icon: 'redo', title: t('editor.redo'), run: () => chain().redo().run() },
]
</script>

<template>
  <div class="note-editor">
    <div v-if="editor" class="toolbar" role="toolbar" :aria-label="t('detail.note')">
      <template v-for="command in commands" :key="command.title">
        <button
          type="button"
          :title="command.title"
          :aria-label="command.title"
          :class="{ active: command.active?.() }"
          @mousedown.prevent
          @click="command.run()"
        >
          <AppIcon v-if="command.icon" :name="command.icon" />
          <span v-else :class="['label', command.label]">{{ command.label }}</span>
        </button>
        <!-- Aligner et colorer : juste après les titres, avec la mise en forme du texte. -->
        <FormatMenus v-if="command.label === 'H2'" :format="noteFormat" />
      </template>
    </div>
    <form
      v-if="noteLinks.editing.value"
      ref="linkFields"
      class="link-form"
      @submit.prevent="noteLinks.apply()"
      @keydown.esc.stop.prevent="noteLinks.cancel()"
    >
      <label>
        {{ t('editor.linkText') }}
        <input v-model="noteLinks.text.value" name="text" :placeholder="t('editor.linkTextPlaceholder')" />
      </label>
      <label>
        {{ t('editor.linkHref') }}
        <input v-model="noteLinks.href.value" name="href" :placeholder="t('editor.linkPlaceholder')" />
      </label>
      <div class="link-actions">
        <button type="submit" class="primary">{{ t('editor.linkApply') }}</button>
        <button type="button" @click="noteLinks.cancel()">{{ t('common.cancel') }}</button>
      </div>
    </form>
    <div v-else-if="noteLinks.currentHref.value" class="link-bar">
      <AppIcon name="link" :size="14" />
      <span class="href" :title="noteLinks.currentHref.value">{{ noteLinks.currentHref.value }}</span>
      <button type="button" @mousedown.prevent @click="noteLinks.open()">
        <AppIcon name="external" :size="14" /> {{ t('editor.linkOpen') }}
      </button>
      <button type="button" @mousedown.prevent @click="startLink()">
        <AppIcon name="edit" :size="14" /> {{ t('editor.linkEdit') }}
      </button>
      <button type="button" @mousedown.prevent @click="noteLinks.remove()">
        <AppIcon name="close" :size="14" /> {{ t('editor.linkRemove') }}
      </button>
    </div>
    <div v-if="noteMedia.recorder.state.value !== 'idle'" class="recording" role="status">
      <span class="dot" />
      <span v-if="noteMedia.recorder.state.value === 'starting'">{{ t('media.micStarting') }}</span>
      <span v-else>{{ t('media.recording') }} {{ formatDuration(noteMedia.recorder.elapsed.value) }}</span>
      <span
        class="meter"
        role="meter"
        :aria-label="t('media.micLevel')"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="Math.round(noteMedia.recorder.level.value * 100)"
      >
        <span
          v-for="segment in 12"
          :key="segment"
          class="segment"
          :class="{ lit: noteMedia.recorder.level.value * 12 >= segment - 0.5, hot: segment > 9 }"
        />
      </span>
      <button
        type="button"
        class="primary"
        :disabled="noteMedia.recorder.state.value !== 'recording'"
        @click="noteMedia.toggleRecording()"
      >
        <AppIcon name="stop" :size="12" /> {{ t('editor.stopRecording') }}
      </button>
      <button type="button" @click="noteMedia.cancelRecording()">{{ t('common.cancel') }}</button>
      <span v-if="noteMedia.recorder.microphone.value" class="mic-name">
        <AppIcon name="mic" :size="12" /> {{ noteMedia.recorder.microphone.value }}
      </span>
      <span v-if="noteMedia.recorder.silent.value" class="silent" role="alert">{{ t('media.silent') }}</span>
    </div>
    <input ref="fileInput" type="file" multiple hidden @change="onFilesPicked" />
    <div ref="contentArea" class="content" @mousedown="focusAtEnd">
      <EditorContent :editor="editor" />
      <TableControls v-if="editor && contentArea" :editor="editor" :table="noteTable" :container="contentArea" />
      <p v-if="editor?.isEmpty" class="placeholder" aria-hidden="true">{{ t('editor.placeholder') }}</p>
    </div>
  </div>
</template>

<style scoped>
.recording {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--danger);
}
.recording {
  flex-wrap: wrap;
}
.meter {
  display: inline-flex;
  align-items: flex-end;
  gap: 2px;
  height: 16px;
}
.segment {
  width: 4px;
  height: 100%;
  border-radius: 1px;
  background: var(--border);
  transition: background 0.06s;
}
.segment.lit {
  background: var(--ok);
}
.segment.lit.hot {
  background: #e0a100;
}
.mic-name {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--muted);
}
.silent {
  flex-basis: 100%;
  font-size: 12px;
  color: var(--danger);
}
.recording .dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--danger);
  animation: pulse 1s infinite;
}
@keyframes pulse {
  50% {
    opacity: 0.3;
  }
}
.note-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
/* Barre d'outils collée en haut du panneau quand on fait défiler une longue note. */
.toolbar {
  position: sticky;
  top: -10px;
  z-index: 5;
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.toolbar button {
  display: inline-grid;
  place-items: center;
  height: 28px;
}
.toolbar .label {
  font-weight: 700;
}
.toolbar .label.I {
  font-style: italic;
  font-family: Georgia, serif;
}
.toolbar .label.S {
  text-decoration: line-through;
}
.content {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  cursor: text;
}
.content > :deep(div) {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.placeholder {
  position: absolute;
  top: 14px;
  left: 18px;
  right: 18px;
  margin: 0;
  color: var(--muted);
  pointer-events: none;
  font-size: 15px;
}
.toolbar button {
  min-width: 28px;
  padding: 2px 6px;
  font-size: 12px;
  background: transparent;
  border-color: transparent;
}
.toolbar button:hover {
  background: var(--hover);
}
.toolbar button.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.link-form,
.link-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 6px 10px;
  padding: 6px 8px;
  border: 1px solid var(--accent);
  border-radius: 6px;
  background: var(--accent-soft);
  font-size: 12px;
}
.link-form label {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1 1 160px;
}
.link-form input {
  font: inherit;
  font-size: 13px;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 3px 6px;
}
.link-actions {
  display: flex;
  gap: 6px;
}
.link-bar {
  align-items: center;
}
.link-bar .href {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--accent);
}
.link-bar button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 1px 6px;
  font-size: 12px;
}
.content :deep(.ProseMirror) {
  flex: 1;
  min-height: max(260px, 45vh);
  padding: 12px 16px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  outline: none;
  font-size: 15px;
  line-height: 1.6;
  transition:
    border-color 0.15s,
    box-shadow 0.15s;
}
.content :deep(.ProseMirror:focus) {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.content :deep(h1) {
  font-size: 20px;
  margin: 10px 0 4px;
}
.content :deep(h2) {
  font-size: 17px;
  margin: 8px 0 4px;
}
.content :deep(p) {
  margin: 4px 0;
}
.content :deep(a) {
  color: var(--accent);
}
.content :deep(code) {
  background: var(--surface-2);
  border-radius: 4px;
  padding: 0 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.content :deep(pre) {
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  overflow: auto;
}
.content :deep(pre code) {
  background: none;
  padding: 0;
}
.content :deep(blockquote) {
  margin: 6px 0;
  padding-left: 12px;
  border-left: 3px solid var(--border);
  color: var(--muted);
}
.content :deep(ul[data-type='taskList']) {
  list-style: none;
  padding-left: 4px;
}
.content :deep(ul[data-type='taskList'] li) {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}
.content :deep(ul[data-type='taskList'] li > label) {
  margin-top: 2px;
}
.content :deep(ul[data-type='taskList'] li[data-checked='true'] > div) {
  color: var(--done);
  text-decoration: line-through;
}
.content :deep(table) {
  border-collapse: collapse;
  margin: 6px 0;
}
.content :deep(th),
.content :deep(td) {
  border: 1px solid var(--border);
  padding: 4px 10px;
  min-width: 60px;
}
.content :deep(th) {
  background: var(--surface-2);
}
/* Couleur du texte (ADR-0012) : le nom vient du fichier, la teinte du thème. */
.content :deep(span[data-color='red']) {
  color: var(--text-red);
}
.content :deep(span[data-color='orange']) {
  color: var(--text-orange);
}
.content :deep(span[data-color='green']) {
  color: var(--text-green);
}
.content :deep(span[data-color='blue']) {
  color: var(--text-blue);
}
.content :deep(span[data-color='purple']) {
  color: var(--text-purple);
}
.content :deep(span[data-color='gray']) {
  color: var(--text-gray);
}
/* Cellules sélectionnées (glisser, Maj+clic, poignée de ligne / colonne). */
.content :deep(.selectedCell) {
  background: var(--selected);
}
.content :deep(.ProseMirror-hideselection *::selection) {
  background: transparent;
}
.content :deep(.radio-group) {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 10px;
}
.content :deep(.radio-option) {
  display: flex;
  align-items: center;
  gap: 8px;
}
/* Texte de l'option : occupe toute la ligne, même vide, pour que le curseur y soit visible
   (retour de test I3 : curseur « perdu » dans une option vide). */
.content :deep(.radio-option > span:last-child) {
  flex: 1;
  min-width: 2ch;
  min-height: 1.4em;
}
.content :deep(.radio) {
  width: 13px;
  height: 13px;
  border: 1.5px solid var(--muted);
  border-radius: 50%;
  display: inline-block;
}
.content :deep(.radio) {
  cursor: pointer;
}
.content :deep(.radio.on) {
  background: radial-gradient(var(--accent) 45%, transparent 50%);
}
.content :deep(.media-placeholder) {
  display: inline-block;
  padding: 4px 10px;
  border: 1px dashed var(--border);
  border-radius: 6px;
  color: var(--muted);
  font-size: 13px;
}
.content :deep(img) {
  max-width: 100%;
  border-radius: 6px;
}
</style>
