<script setup lang="ts">
/**
 * Éditeur de note TipTap (WYSIWYG, raccourcis Markdown à la frappe : `# `, `- `, `[] `, ``` …).
 * Il émet `update` à chaque modification ; la sauvegarde (débounce) est faite par `useTaskDetail`.
 * Quand `version` change, le contenu est remplacé par `content` (rechargement depuis le cœur)
 * sans émettre de modification.
 *
 * Médias (I3) : coller / glisser une image ou un fichier, 📎 joindre, 🎙 enregistrer, ⦿ radio.
 */
import { inject, onBeforeUnmount, ref, watch } from 'vue'
import type { JSONContent } from '@tiptap/core'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import type { NoteDocument } from '../../domain/task'
import { t } from '../../i18n'
import { mediaGatewayKey } from '../../ports/MediaGateway'
import { formatDuration } from '../../application/mediaRules'
import { createNoteExtensions } from './extensions'
import { useNoteMedia } from './useNoteMedia'

const props = defineProps<{ content: NoteDocument; version: number }>()
const emit = defineEmits<{
  update: [note: NoteDocument]
  blur: []
  notify: [kind: 'info' | 'error', message: string]
}>()

const media = inject(mediaGatewayKey)
if (!media) throw new Error('MediaGateway non fourni (voir main.ts)')

/** Le domaine ne dépend pas de TipTap : la conversion de type se fait ici, à sa frontière. */
const asTipTap = (note: NoteDocument) => note as JSONContent

const editor = useEditor({
  extensions: createNoteExtensions(media),
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

const noteMedia = useNoteMedia(editor, media, (kind, message) => emit('notify', kind, message))
const fileInput = ref<HTMLInputElement | null>(null)

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

// ----- Lien ----------------------------------------------------------------------------------

const linkInput = ref<HTMLInputElement | null>(null)
const editingLink = ref(false)
const linkValue = ref('')

function startLink(): void {
  linkValue.value = (editor.value?.getAttributes('link').href as string | undefined) ?? ''
  editingLink.value = true
  setTimeout(() => linkInput.value?.focus())
}

function applyLink(): void {
  const chain = editor.value?.chain().focus().extendMarkRange('link')
  if (linkValue.value.trim()) chain?.setLink({ href: linkValue.value.trim() }).run()
  else chain?.unsetLink().run()
  editingLink.value = false
}

// ----- Barre d'outils ------------------------------------------------------------------------

type Command = { label: string; title: string; active?: () => boolean; run: () => void }

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
    label: '</>',
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
    label: '•',
    title: t('editor.bulletList'),
    active: () => !!editor.value?.isActive('bulletList'),
    run: () => chain().toggleBulletList().run(),
  },
  {
    label: '1.',
    title: t('editor.orderedList'),
    active: () => !!editor.value?.isActive('orderedList'),
    run: () => chain().toggleOrderedList().run(),
  },
  {
    label: '☑',
    title: t('editor.taskList'),
    active: () => !!editor.value?.isActive('taskList'),
    run: () => chain().toggleTaskList().run(),
  },
  {
    label: '❝',
    title: t('editor.quote'),
    active: () => !!editor.value?.isActive('blockquote'),
    run: () => chain().toggleBlockquote().run(),
  },
  {
    label: '{ }',
    title: t('editor.codeBlock'),
    active: () => !!editor.value?.isActive('codeBlock'),
    run: () => chain().toggleCodeBlock().run(),
  },
  {
    label: '⊞',
    title: t('editor.table'),
    run: () => chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  { label: '🔗', title: t('editor.link'), active: () => !!editor.value?.isActive('link'), run: startLink },
  {
    label: '⦿',
    title: t('editor.radio'),
    active: () => !!editor.value?.isActive('radioGroup'),
    run: noteMedia.insertRadioGroup,
  },
  { label: '📎', title: t('editor.attach'), run: () => fileInput.value?.click() },
  {
    label: '🎙',
    title: t('editor.record'),
    active: () => noteMedia.recorder.state.value !== 'idle',
    run: () => void noteMedia.toggleRecording(),
  },
  { label: '↶', title: t('editor.undo'), run: () => chain().undo().run() },
  { label: '↷', title: t('editor.redo'), run: () => chain().redo().run() },
]
</script>

<template>
  <div class="note-editor">
    <div v-if="editor" class="toolbar" role="toolbar" :aria-label="t('detail.note')">
      <button
        v-for="command in commands"
        :key="command.title"
        type="button"
        :title="command.title"
        :aria-label="command.title"
        :class="{ active: command.active?.() }"
        @mousedown.prevent
        @click="command.run()"
      >
        {{ command.label }}
      </button>
    </div>
    <input
      v-if="editingLink"
      ref="linkInput"
      v-model="linkValue"
      class="link-input"
      :placeholder="t('editor.linkPlaceholder')"
      @keydown.enter.prevent="applyLink"
      @keydown.esc.stop.prevent="editingLink = false"
      @blur="editingLink = false"
    />
    <div v-if="noteMedia.recorder.state.value !== 'idle'" class="recording" role="status">
      <span class="dot" />
      <span v-if="noteMedia.recorder.state.value === 'starting'">{{ t('media.micStarting') }}</span>
      <span v-else>{{ t('media.recording') }} {{ formatDuration(noteMedia.recorder.elapsed.value) }}</span>
      <button
        type="button"
        class="primary"
        :disabled="noteMedia.recorder.state.value !== 'recording'"
        @click="noteMedia.toggleRecording()"
      >
        ■ {{ t('editor.stopRecording') }}
      </button>
      <button type="button" @click="noteMedia.cancelRecording()">{{ t('common.cancel') }}</button>
    </div>
    <input ref="fileInput" type="file" multiple hidden @change="onFilesPicked" />
    <EditorContent :editor="editor" class="content" />
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
.toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
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
.link-input {
  font: inherit;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--accent);
  border-radius: 6px;
  padding: 4px 8px;
}
.content :deep(.ProseMirror) {
  min-height: 160px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  outline: none;
  line-height: 1.55;
}
.content :deep(.ProseMirror:focus) {
  border-color: var(--accent);
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
