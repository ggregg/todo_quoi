import { describe, expect, it } from 'vitest'
import { InMemoryFileAttachments } from '../infrastructure/in-memory/InMemoryFileAttachments'
import { useConfirm } from './useConfirm'
import { useFileAttach } from './useFileAttach'

function setup(picked: string[] | null) {
  const files = new InMemoryFileAttachments('mac-greg')
  files.picked = picked
  for (const path of picked ?? []) files.present.add(path)
  const confirm = useConfirm()
  const attach = useFileAttach(files, confirm, () => 'tache-1')
  /** Lance « Joindre » et répond `choice` à la question copier / déplacer / lier. */
  async function run(choice: string) {
    const pending = attach.pickAndAttach()
    await Promise.resolve()
    await Promise.resolve()
    confirm.answer(choice)
    return pending
  }
  return { files, confirm, attach, run }
}

describe('useFileAttach', () => {
  it('without a native picker, falls back to the browser file field', async () => {
    const { attach } = setup(null)
    expect(await attach.pickAndAttach()).toBeNull()
  })

  it('copy is the default choice and keeps the original', async () => {
    const { files, confirm, attach } = setup(['/docs/devis.pdf'])
    const pending = attach.pickAndAttach()
    await Promise.resolve()
    await Promise.resolve()
    expect(confirm.current.value?.question.choices.find((choice) => choice.primary)?.id).toBe('copy')
    confirm.answer('copy')
    const result = await pending
    expect(result?.nodes).toEqual([
      { type: 'attachment', attrs: { src: `../media/${'0'.repeat(64)}.bin`, name: 'devis.pdf', size: 1 } },
    ])
    expect(files.present.has('/docs/devis.pdf')).toBe(true)
  })

  it('move removes the original', async () => {
    const { files, run } = setup(['/docs/devis.pdf'])
    await run('move')
    expect(files.moved).toEqual(['/docs/devis.pdf'])
  })

  it('link keeps the path and this device, nothing is copied', async () => {
    const { run } = setup(['C:\\Users\\greg\\plan.dwg'])
    const result = await run('link')
    expect(result?.nodes).toEqual([
      { type: 'fileLink', attrs: { path: 'C:\\Users\\greg\\plan.dwg', name: 'plan.dwg', device: 'mac-greg' } },
    ])
  })

  it('cancelling inserts nothing', async () => {
    const { run } = setup(['/docs/devis.pdf'])
    expect(await run('cancel')).toEqual({ nodes: [], failures: [] })
  })

  it('a link is inactive on another device and when the file is gone', async () => {
    const files = new InMemoryFileAttachments('mac-greg')
    files.present.add('/docs/plan.dwg')
    expect(await files.linkState({ path: '/docs/plan.dwg', device: 'mac-greg' })).toBe('available')
    expect(await files.linkState({ path: '/docs/plan.dwg', device: 'pc-bureau' })).toBe('otherDevice')
    files.present.delete('/docs/plan.dwg')
    expect(await files.linkState({ path: '/docs/plan.dwg', device: 'mac-greg' })).toBe('missing')
  })
})
