/**
 * Faux `VaultGateway` (navigateur, tests) : rien n'est chiffré. Il simule seulement les états —
 * coffre absent / verrouillé / ouvert par base, mot de passe vérifié, tâches « scellées » quand
 * leur coffre est verrouillé — pour travailler l'interface. Le vrai coffre est en Rust.
 */
import { type BaseId, type Task, type TaskId, type UiError } from '../../domain/task'
import { MIN_PASSWORD_CHARS, type BaseVault, type EncryptScope, type VaultGateway } from '../../ports/VaultGateway'
import type { InMemoryTaskGateway } from './InMemoryTaskGateway'

export class InMemoryVaultGateway implements VaultGateway {
  private readonly passwords = new Map<BaseId, string>()
  private readonly open = new Set<BaseId>()

  constructor(
    private readonly tasks: InMemoryTaskGateway,
    private readonly bases: () => Promise<BaseId[]>,
  ) {}

  async listVaults(): Promise<BaseVault[]> {
    return (await this.bases()).map((base) => ({ base, state: this.stateOf(base) }))
  }
  async createVault(base: BaseId, password: string): Promise<void> {
    checkLength(password)
    if (this.passwords.has(base)) throw error('vault', 'cette base a déjà un coffre')
    this.passwords.set(base, password)
    this.open.add(base)
  }
  async unlockVault(base: BaseId, password: string): Promise<void> {
    const expected = this.passwords.get(base)
    if (expected === undefined) throw error('vault-none', 'cette base n’a pas encore de coffre')
    if (expected !== password) throw error('vault-wrong-password', 'mot de passe incorrect')
    this.open.add(base)
    this.tasks.setConfidentiality((task) =>
      task.base === base && task.confidentiality === 'sealed' ? 'encrypted' : null,
    )
  }
  async lockVault(base: BaseId): Promise<void> {
    this.open.delete(base)
    this.tasks.setConfidentiality((task) =>
      task.base === base && task.confidentiality === 'encrypted' ? 'sealed' : null,
    )
  }
  async encryptTask(taskId: TaskId, scope: EncryptScope): Promise<TaskId[]> {
    const all = await this.allTasks()
    const task = all.find((candidate) => candidate.id === taskId)
    if (!task) throw error('rule', 'tâche introuvable')
    if (!this.open.has(task.base))
      throw error(this.passwords.has(task.base) ? 'vault-locked' : 'vault-none', 'coffre fermé')
    const targets = new Set([taskId, ...(scope === 'withSubtasks' ? descendants(all, taskId) : [])])
    return this.tasks.setConfidentiality((candidate) =>
      targets.has(candidate.id) && candidate.confidentiality === 'plain' ? 'encrypted' : null,
    )
  }
  async decryptTask(taskId: TaskId): Promise<void> {
    this.tasks.setConfidentiality((task) =>
      task.id === taskId && task.confidentiality === 'encrypted' ? 'plain' : null,
    )
  }

  private stateOf(base: BaseId): BaseVault['state'] {
    if (this.open.has(base)) return 'unlocked'
    return this.passwords.has(base) ? 'locked' : 'none'
  }
  private async allTasks(): Promise<Task[]> {
    return (await this.tasks.listTasks()).tasks
  }
}

function descendants(tasks: Task[], id: TaskId): TaskId[] {
  const children = tasks.filter((task) => task.parentId === id).map((task) => task.id)
  return children.flatMap((child) => [child, ...descendants(tasks, child)])
}

function checkLength(password: string): void {
  if ([...password].length < MIN_PASSWORD_CHARS) {
    throw error('vault-password-too-short', `le mot de passe doit contenir au moins ${MIN_PASSWORD_CHARS} caractères`)
  }
}

function error(code: UiError['code'], message: string): UiError {
  return { code, message }
}
