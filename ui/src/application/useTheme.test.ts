import { describe, expect, it, vi } from 'vitest'
import { THEME_STORAGE_KEY, readTheme, useTheme } from './useTheme'

function memoryStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial))
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
  }
}

describe('useTheme', () => {
  it('follows the system by default and ignores an unknown saved value', () => {
    expect(readTheme(memoryStorage())).toBe('system')
    expect(readTheme(memoryStorage({ [THEME_STORAGE_KEY]: 'violet' }))).toBe('system')
  })

  it('applies, saves and tells the native window', () => {
    const storage = memoryStorage()
    const root = { dataset: {} } as unknown as HTMLElement
    const native = vi.fn()
    const theme = useTheme({ storage, root, native })
    theme.choose('dark')
    expect(root.dataset.theme).toBe('dark')
    expect(storage.getItem(THEME_STORAGE_KEY)).toBe('dark')
    expect(native).toHaveBeenLastCalledWith('dark')
    theme.choose('system')
    expect(root.dataset.theme).toBeUndefined()
    expect(native).toHaveBeenLastCalledWith(null)
  })

  it('follows a choice made in another window', () => {
    const storage = memoryStorage()
    const root = { dataset: {} } as unknown as HTMLElement
    const theme = useTheme({ storage, root })
    const target = new EventTarget()
    const stop = theme.start(target as unknown as Window)
    storage.setItem(THEME_STORAGE_KEY, 'light')
    target.dispatchEvent(Object.assign(new Event('storage'), { key: THEME_STORAGE_KEY }))
    expect(theme.theme.value).toBe('light')
    expect(root.dataset.theme).toBe('light')
    stop()
  })
})
