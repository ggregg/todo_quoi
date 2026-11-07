import { describe, expect, it } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import { id, makeTask } from './testSupport'
import { useTrash } from './useTrash'

describe('useTrash', () => {
  it('lists deleted roots and restores them with their subtasks', async () => {
    const gateway = new InMemoryTaskGateway([makeTask(1, 'Projet'), makeTask(2, 'Lot', 1), makeTask(3, 'Courses')])
    await gateway.deleteTask(id(1))
    const restoredInTree: string[] = []
    const trash = useTrash(gateway, async (taskId) => {
      restoredInTree.push(taskId)
      await gateway.restoreTask(taskId)
    })

    await trash.show()
    expect(trash.open.value).toBe(true)
    expect(trash.items.value.map((task) => task.title)).toEqual(['Projet'])

    await trash.restore(id(1))
    expect(restoredInTree).toEqual([id(1)])
    expect(trash.items.value).toEqual([])
    expect((await gateway.listTasks()).tasks).toHaveLength(3)
  })
})
