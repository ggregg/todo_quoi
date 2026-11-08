import { describe, expect, it } from 'vitest'
import { SILENCE_WARNING_MS, SilenceWatch, levelOf } from './audioLevel'

describe('audioLevel', () => {
  it('silence gives 0, a loud signal fills the gauge', () => {
    expect(levelOf(new Uint8Array(256).fill(128))).toBe(0)
    const loud = Uint8Array.from({ length: 256 }, (_, i) => (i % 2 === 0 ? 255 : 0))
    expect(levelOf(loud)).toBe(1)
    expect(levelOf([])).toBe(0)
  })

  it('a quiet voice is visible but not saturated', () => {
    const quiet = Uint8Array.from({ length: 256 }, (_, i) => (i % 2 === 0 ? 140 : 116))
    const level = levelOf(quiet)
    expect(level).toBeGreaterThan(0.2)
    expect(level).toBeLessThan(1)
  })

  it('warns only after a long continuous silence', () => {
    const watch = new SilenceWatch(0)
    expect(watch.update(0, SILENCE_WARNING_MS - 1)).toBe(false)
    expect(watch.update(0, SILENCE_WARNING_MS)).toBe(true)
    expect(watch.update(0.5, SILENCE_WARNING_MS + 10)).toBe(false)
    expect(watch.update(0, SILENCE_WARNING_MS + 100)).toBe(false)
  })
})
