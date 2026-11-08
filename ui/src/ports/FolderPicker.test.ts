import { describe, expect, it } from 'vitest'
import { suggestedBaseName } from './FolderPicker'

describe('suggestedBaseName', () => {
  it('names a base after its folder, or the parent of a TodoQuoi folder', () => {
    expect(suggestedBaseName('/Users/greg/Documents/Pro')).toBe('Pro')
    expect(suggestedBaseName('/Users/greg/Dropbox/Asso/TodoQuoi/')).toBe('Asso')
    expect(suggestedBaseName('C:\\Users\\greg\\OneDrive\\Perso')).toBe('Perso')
    expect(suggestedBaseName('E:\\')).toBe('')
  })
})
