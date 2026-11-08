import { describe, expect, it } from 'vitest'
import { InMemorySettingsGateway } from '../infrastructure/in-memory/InMemorySettingsGateway'
import { useDataLocation } from './useDataLocation'

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
})
