import { describe, expect, it } from 'vitest'
import { InMemorySettingsGateway } from '../infrastructure/in-memory/InMemorySettingsGateway'
import { useDataLocation } from './useDataLocation'
import { useConfirm } from './useConfirm'

describe('useDataLocation', () => {
  it('adds, renames and removes bases, and shows refusals', async () => {
    const gateway = new InMemorySettingsGateway()
    const state = useDataLocation(gateway)
    await state.openDialog()
    expect(state.location.value?.bases.map((base) => base.id)).toEqual(['perso', 'pro'])

    await state.addBase('Asso', '#2fa36b', 'relatif')
    expect(state.error.value).toContain('chemin complet')

    await state.addBase(' Asso ', '#2fa36b', '/Users/demo/Dropbox')
    expect(state.infoOf('asso').name).toBe('Asso')

    await state.updateBase('pro', 'Travail', '#000000')
    expect(state.infoOf('pro')).toMatchObject({ name: 'Travail', color: '#000000' })

    await state.removeBase('asso')
    expect(state.location.value?.bases).toHaveLength(2)
    expect(state.infoOf('inconnue').name).toBe('inconnue')
  })
  it('detaches after the app confirmation, never the device-only base', async () => {
    const gateway = new InMemorySettingsGateway()
    const confirm = useConfirm()
    const state = useDataLocation(gateway, confirm)
    await state.load()

    const cancelled = state.askToRemove('pro')
    await Promise.resolve()
    expect(confirm.current.value?.question.title).toContain('Pro')
    confirm.answer('cancel')
    await cancelled
    expect(state.location.value?.bases).toHaveLength(2)

    const accepted = state.askToRemove('pro')
    await Promise.resolve()
    confirm.answer('detach')
    await accepted
    expect(state.location.value?.bases.map((base) => base.id)).toEqual(['perso'])

    // Plus qu'une base, et c'est le dossier local de l'appareil : rien à détacher.
    gateway.location.deviceOnly = true
    await state.load()
    expect(state.canDetach('perso')).toBe(false)
    await state.askToRemove('perso')
    expect(confirm.current.value).toBeNull()
  })
})
