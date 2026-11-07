import { describe, expect, it } from 'vitest'
import { openImageViewer, useImageViewer } from './useImageViewer'

describe('useImageViewer', () => {
  it('opens on the chosen image, loops with arrows and closes', () => {
    const viewer = useImageViewer()
    openImageViewer(
      [
        { url: 'a', alt: 'A' },
        { url: 'b', alt: 'B' },
        { url: 'c', alt: 'C' },
      ],
      2,
    )
    expect(viewer.current.value?.alt).toBe('C')
    viewer.go(1)
    expect(viewer.current.value?.alt).toBe('A')
    viewer.go(-1)
    expect(viewer.index.value).toBe(2)
    viewer.close()
    expect(viewer.current.value).toBeNull()
  })

  it('ignores an empty list', () => {
    const viewer = useImageViewer()
    openImageViewer([], 0)
    expect(viewer.current.value).toBeNull()
  })
})
