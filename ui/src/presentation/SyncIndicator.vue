<script setup lang="ts">
/**
 * Indicateur de synchronisation dans la barre haute (spec §7) ; un clic ouvre le détail et le
 * journal des conflits (versions écartées, gardées 30 jours dans `.conflicts/`).
 */
import { computed, ref } from 'vue'
import type { SyncState } from '../application/useSyncStatus'
import { formatDateTime, t } from '../i18n'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ sync: SyncState }>()
const open = ref(false)
const status = computed(() => props.sync.status.value)

const detail = computed(() => {
  const s = status.value
  if (s.kind === 'offline') return s.message ?? ''
  const parts: string[] = []
  if (s.pending > 0) parts.push(t('sync.detail.pending', { count: s.pending }))
  if (s.unreadable > 0) parts.push(t('sync.detail.unreadable', { count: s.unreadable }))
  if (s.conflicts > 0) parts.push(t('sync.detail.conflicts', { count: s.conflicts }))
  return parts.join(' · ')
})

function toggle(): void {
  open.value = !open.value
  if (open.value) void props.sync.syncNow()
}

function setAsideOn(iso: string): string {
  return formatDateTime(iso)
}
</script>

<template>
  <div class="sync">
    <button
      type="button"
      class="indicator"
      :class="status.kind"
      :title="`${t(sync.labelKey.value)}${detail ? ' — ' + detail : ''}`"
      :aria-expanded="open"
      @click="toggle"
    >
      <span class="dot" aria-hidden="true" />
      <span class="label">{{ t(sync.labelKey.value) }}</span>
    </button>
    <div v-if="open" class="panel" role="dialog" :aria-label="t('sync.title')" @keydown.esc.stop="open = false">
      <p class="state">
        <strong>{{ t(sync.labelKey.value) }}</strong>
        <span v-if="detail">{{ detail }}</span>
      </p>
      <p class="hint">{{ t('sync.howItWorks') }}</p>
      <button v-if="!sync.journalOpen.value" type="button" @click="sync.openJournal()">
        <AppIcon name="file" :size="14" /> {{ t('sync.journal') }}
      </button>
      <template v-else>
        <h3>{{ t('sync.journal') }}</h3>
        <p v-if="sync.conflicts.value.length === 0" class="hint">{{ t('sync.noConflict') }}</p>
        <ul v-else>
          <li v-for="copy in sync.conflicts.value" :key="copy.fileName">
            <strong>{{ copy.title ?? copy.fileName }}</strong>
            <span class="hint">{{ t('sync.setAsideOn', { date: setAsideOn(copy.setAsideAt) }) }}</span>
            <code>.conflicts/{{ copy.fileName }}</code>
          </li>
        </ul>
      </template>
    </div>
  </div>
</template>

<style scoped>
.sync {
  position: relative;
}
.indicator {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  font-size: 12px;
  color: var(--muted);
  background: none;
  border: 1px solid transparent;
  border-radius: 999px;
}
.indicator:hover {
  border-color: var(--border);
}
.dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--muted);
}
.upToDate .dot {
  background: var(--ok);
}
.downloading .dot,
.starting .dot {
  background: var(--accent);
  animation: pulse 1.2s infinite;
}
.attention .dot {
  background: #e0a100;
}
.offline .dot {
  background: var(--danger);
}
.offline .label {
  color: var(--danger);
}
@keyframes pulse {
  50% {
    opacity: 0.3;
  }
}
.panel {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  z-index: 40;
  width: 340px;
  max-height: 60vh;
  overflow-y: auto;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  font-size: 13px;
}
.state {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0;
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--muted);
}
h3 {
  margin: 4px 0 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--muted);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
}
code {
  font-size: 11px;
  word-break: break-all;
  color: var(--muted);
}
button {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
</style>
