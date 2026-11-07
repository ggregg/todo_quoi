import { describe, expect, it } from 'vitest'
import { isOpenableLink, normalizeLinkInput } from './linkRules'

describe('linkRules', () => {
  it('completes what people usually type', () => {
    expect(normalizeLinkInput(' www.exemple.fr ')).toBe('https://www.exemple.fr')
    expect(normalizeLinkInput('exemple.fr/page')).toBe('https://exemple.fr/page')
    expect(normalizeLinkInput('greg@exemple.fr')).toBe('mailto:greg@exemple.fr')
    expect(normalizeLinkInput('http://a.b')).toBe('http://a.b')
    expect(normalizeLinkInput('  ')).toBe('')
  })

  it('only web and mail links are openable', () => {
    expect(isOpenableLink('https://a.b')).toBe(true)
    expect(isOpenableLink('mailto:x@y.z')).toBe(true)
    expect(isOpenableLink('javascript:alert(1)')).toBe(false)
    expect(isOpenableLink('file:///etc')).toBe(false)
  })
})
