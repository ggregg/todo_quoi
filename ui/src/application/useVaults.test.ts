import { describe, expect, it, vi } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import { InMemoryVaultGateway } from '../infrastructure/in-memory/InMemoryVaultGateway'
import { MAIN_BASE } from '../domain/task'
import { id, makeTask } from './testSupport'
import { useConfirm } from './useConfirm'
import { useVaults } from './useVaults'

function setup() {
  const tasks = new InMemoryTaskGateway([makeTask(1, 'Projet'), makeTask(2, 'Étape', 1)])
  const gateway = new InMemoryVaultGateway(tasks, async () => [MAIN_BASE])
  const confirm = useConfirm()
  const changed = vi.fn(async () => {})
  const vaults = useVaults(gateway, confirm, changed)
  const confidentiality = async (n: number) =>
    (await tasks.listTasks()).tasks.find((task) => task.id === id(n))?.confidentiality
  return { tasks, vaults, confirm, changed, confidentiality }
}

describe('useVaults', () => {
  it('the first encryption of a base asks for a password, creates the vault, then encrypts', async () => {
    const { tasks, vaults, confidentiality, changed } = setup()
    await vaults.load()
    expect(vaults.stateOf(MAIN_BASE)).toBe('none')

    const task = (await tasks.listTasks()).tasks.find((t) => t.id === id(2))!
    await vaults.encrypt(task, 0)
    expect(vaults.prompt.value).toEqual({ kind: 'create', base: MAIN_BASE })

    await vaults.submit({ password: 'mot de passe', confirmation: 'mot de passe' })
    expect(vaults.prompt.value).toBeNull()
    expect(vaults.stateOf(MAIN_BASE)).toBe('unlocked')
    expect(await confidentiality(2)).toBe('encrypted')
    expect(changed).toHaveBeenCalled()
  })

  it('refuses two different passwords at creation and keeps the dialog open', async () => {
    const { vaults } = setup()
    vaults.ask('create', MAIN_BASE)
    await vaults.submit({ password: 'mot de passe', confirmation: 'autre chose' })
    expect(vaults.error.value).toMatch(/identiques/)
    expect(vaults.prompt.value).not.toBeNull()
  })

  it('locking seals the encrypted tasks; a wrong password is explained, the right one reopens', async () => {
    const { tasks, vaults, confidentiality } = setup()
    vaults.ask('create', MAIN_BASE)
    await vaults.submit({ password: 'mot de passe', confirmation: 'mot de passe' })
    const task = (await tasks.listTasks()).tasks.find((t) => t.id === id(2))!
    await vaults.encrypt(task, 0)

    await vaults.toggle(MAIN_BASE) // ouvert → verrouillé
    expect(vaults.stateOf(MAIN_BASE)).toBe('locked')
    expect(await confidentiality(2)).toBe('sealed')

    await vaults.toggle(MAIN_BASE) // verrouillé → dialogue
    expect(vaults.prompt.value).toEqual({ kind: 'unlock', base: MAIN_BASE })
    await vaults.submit({ password: 'pas le bon' })
    expect(vaults.error.value).toBe('Mot de passe incorrect.')
    await vaults.submit({ password: 'mot de passe' })
    expect(vaults.stateOf(MAIN_BASE)).toBe('unlocked')
    expect(await confidentiality(2)).toBe('encrypted')
  })

  it('a task with subtasks asks whether to encrypt them too (yes by default)', async () => {
    const { tasks, vaults, confirm, confidentiality } = setup()
    vaults.ask('create', MAIN_BASE)
    await vaults.submit({ password: 'mot de passe', confirmation: 'mot de passe' })
    const parent = (await tasks.listTasks()).tasks.find((t) => t.id === id(1))!

    const pending = vaults.encrypt(parent, 1)
    expect(confirm.current.value?.question.choices[0]).toMatchObject({ id: 'withSubtasks', primary: true })
    confirm.answer('withSubtasks')
    await pending
    expect(await confidentiality(1)).toBe('encrypted')
    expect(await confidentiality(2)).toBe('encrypted')
  })

  it('cancelling the subtask question encrypts nothing', async () => {
    const { tasks, vaults, confirm, confidentiality } = setup()
    const parent = (await tasks.listTasks()).tasks.find((t) => t.id === id(1))!
    const pending = vaults.encrypt(parent, 1)
    confirm.answer('cancel')
    await pending
    expect(vaults.prompt.value).toBeNull()
    expect(await confidentiality(1)).toBe('plain')
  })
})

describe('useVaults — mot de passe et activité', () => {
  it('changes the password after checking the current one', async () => {
    const { vaults } = setup()
    vaults.ask('create', MAIN_BASE)
    await vaults.submit({ password: 'mot de passe', confirmation: 'mot de passe' })

    vaults.ask('change', MAIN_BASE)
    await vaults.submit({ password: 'nouveau mdp', confirmation: 'autre', currentPassword: 'mot de passe' })
    expect(vaults.error.value).toMatch(/identiques/)
    await vaults.submit({ password: 'nouveau mdp', confirmation: 'nouveau mdp', currentPassword: 'pas le bon' })
    expect(vaults.error.value).toBe('Mot de passe incorrect.')
    await vaults.submit({ password: 'nouveau mdp', confirmation: 'nouveau mdp', currentPassword: 'mot de passe' })
    expect(vaults.prompt.value).toBeNull()

    await vaults.toggle(MAIN_BASE) // verrouiller
    vaults.ask('unlock', MAIN_BASE)
    await vaults.submit({ password: 'nouveau mdp' })
    expect(vaults.stateOf(MAIN_BASE)).toBe('unlocked')
  })

  it('signals activity at most once per interval and stops listening on cleanup', async () => {
    const tasks = new InMemoryTaskGateway([])
    const gateway = new InMemoryVaultGateway(tasks, async () => [MAIN_BASE])
    const vaults = useVaults(gateway, useConfirm(), async () => {})
    const target = new EventTarget()
    const stop = await vaults.start(target as unknown as Window)
    target.dispatchEvent(new Event('keydown'))
    target.dispatchEvent(new Event('pointerdown'))
    expect(gateway.activity).toBe(1)
    stop()
    vi.useFakeTimers()
    vi.setSystemTime(Date.now() + 60_000)
    target.dispatchEvent(new Event('keydown'))
    vi.useRealTimers()
    expect(gateway.activity).toBe(1)
  })

  it('reads and saves the auto-lock delay', async () => {
    const tasks = new InMemoryTaskGateway([])
    const gateway = new InMemoryVaultGateway(tasks, async () => [MAIN_BASE])
    const vaults = useVaults(gateway, useConfirm(), async () => {})
    await vaults.loadAutoLock()
    expect(vaults.autoLockMinutes.value).toBe(15)
    await vaults.setAutoLock(0)
    expect(gateway.autoLockMinutes).toBe(0)
  })
})
