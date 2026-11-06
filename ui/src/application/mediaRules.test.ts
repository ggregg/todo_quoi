import { describe, expect, it } from 'vitest'
import { extensionFor, formatDuration, formatSize, handlingFor, MAX_MEDIA_BYTES, scaledSize } from './mediaRules'

describe('préparation des médias (POC I0-2)', () => {
  it('classe les fichiers collés ou glissés', () => {
    expect(handlingFor({ type: 'image/jpeg', size: 1000 })).toBe('compress-image')
    expect(handlingFor({ type: 'image/png', size: 1000 })).toBe('compress-image')
    expect(handlingFor({ type: 'image/gif', size: 1000 })).toBe('keep-image')
    expect(handlingFor({ type: 'image/svg+xml', size: 1000 })).toBe('keep-image')
    expect(handlingFor({ type: 'application/pdf', size: 1000 })).toBe('attachment')
    expect(handlingFor({ type: 'image/jpeg', size: MAX_MEDIA_BYTES + 1 })).toBe('too-large')
  })

  it('réduit le côté long à 2560 px en gardant les proportions', () => {
    expect(scaledSize(6000, 4000)).toEqual({ width: 2560, height: 1707 })
    expect(scaledSize(1000, 3000)).toEqual({ width: 853, height: 2560 })
    expect(scaledSize(800, 600)).toEqual({ width: 800, height: 600 })
  })

  it('choisit l’extension par le nom puis par le type', () => {
    expect(extensionFor({ name: 'Devis Final.PDF', type: 'application/pdf' })).toBe('pdf')
    expect(extensionFor({ type: 'audio/webm;codecs=opus' })).toBe('webm')
    expect(extensionFor({ name: 'sans-extension', type: 'application/x-inconnu' })).toBe('bin')
  })

  it('formate durées et tailles', () => {
    expect(formatDuration(12.4)).toBe('0:12')
    expect(formatDuration(75)).toBe('1:15')
    expect(formatSize(318_000)).toBe('311 Ko')
    expect(formatSize(5 * 1024 * 1024)).toBe('5,0 Mo')
  })
})
