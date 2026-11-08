import { describe, expect, it } from 'vitest'
import { InMemorySyncGateway } from '../infrastructure/in-memory/InMemorySyncGateway'
import { useSyncStatus } from './useSyncStatus'

describe('useSyncStatus', () => {
  it('follows the status and opening the journal acknowledges conflicts', async () => {
    const gateway = new InMemorySyncGateway()
    gateway.conflicts = [
      { taskId: 'x', title: 'Note', fileName: 'x.20261107T100000Z.pc.md', setAsideAt: '2026-11-07T10:00:00.000Z' },
    ]
    const sync = useSyncStatus(gateway)
    const stop = await sync.start()
    expect(sync.status.value.kind).toBe('upToDate')

    gateway.setStatus({ kind: 'attention', pending: 0, unreadable: 0, conflicts: 1, message: null })
    expect(sync.labelKey.value).toBe('sync.attention')

    await sync.openJournal()
    expect(sync.conflicts.value).toHaveLength(1)
    expect(sync.status.value.conflicts).toBe(0)
    stop()
  })
})
