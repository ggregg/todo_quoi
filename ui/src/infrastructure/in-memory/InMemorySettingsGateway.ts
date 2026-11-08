/** `SettingsGateway` pour le navigateur (`pnpm ui:dev`) et les tests : rien n'est redémarré. */
import type { DataLocation, SettingsGateway } from '../../ports/SettingsGateway'

export class InMemorySettingsGateway implements SettingsGateway {
  location: DataLocation = {
    kind: 'settings',
    deviceOnly: false,
    bases: [
      {
        id: 'perso',
        name: 'Perso',
        color: '#4f7cff',
        path: '/Users/demo/Documents/TodoQuoi',
        storage: 'local',
        problem: null,
      },
      {
        id: 'pro',
        name: 'Pro',
        color: '#e8833a',
        path: '/Users/demo/OneDrive/TodoQuoi',
        storage: 'oneDrive',
        problem: null,
      },
    ],
    drives: [{ provider: 'icloud', path: '/Users/demo/Library/Mobile Documents/com~apple~CloudDocs' }],
  }

  async getDataLocation(): Promise<DataLocation> {
    return structuredClone(this.location)
  }
  async addBase(name: string, color: string, path: string): Promise<void> {
    if (!path.startsWith('/') && !/^[a-z]:\\/i.test(path))
      throw { code: 'invalid-input', message: 'chemin complet attendu' }
    const id =
      name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '') || 'base'
    this.location.bases.push({
      id,
      name,
      color,
      path: `${path}/TodoQuoi`,
      storage: path.startsWith('/Volumes/') ? 'external' : 'local',
      problem: null,
    })
    this.location.deviceOnly = false
  }
  async updateBase(id: string, name: string, color: string): Promise<void> {
    const base = this.location.bases.find((candidate) => candidate.id === id)
    if (base) Object.assign(base, { name, color })
  }
  async removeBase(id: string): Promise<void> {
    this.location.bases = this.location.bases.filter((base) => base.id !== id)
  }
}
