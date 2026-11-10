import { describe, expect, it } from 'vitest'
import { InMemorySettingsGateway } from '../infrastructure/in-memory/InMemorySettingsGateway'
import { usePreferences } from './usePreferences'

describe('usePreferences', () => {
  it('loads, saves a change and reads back what the core kept', async () => {
    const gateway = new InMemorySettingsGateway()
    const preferences = usePreferences(gateway)
    expect(preferences.images().handling).toBe('compress')
    await preferences.load()
    await preferences.update({ deviceName: 'Mac de Greg', trashRetentionDays: 7 })
    expect(preferences.preferences.value?.deviceId).toBe('mac-de-greg')
    expect(preferences.preferences.value?.trashRetentionDays).toBe(7)
  })

  it('explains a refused change and keeps the previous values', async () => {
    const preferences = usePreferences(new InMemorySettingsGateway())
    await preferences.load()
    await preferences.update({ deviceName: '---' })
    expect(preferences.error.value).toMatch(/au moins une lettre/)
    expect(preferences.preferences.value?.deviceName).toBe('navigateur')
  })
})
