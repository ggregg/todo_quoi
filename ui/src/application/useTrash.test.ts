import { describe, expect, it } from 'vitest'
import { InMemoryTaskGateway } from '../infrastructure/in-memory/InMemoryTaskGateway'
import { id, makeTask } from './testSupport'
import { useConfirm } from './useConfirm'
import { useTrash } from './useTrash'

describe('useTrash', () => {
  it('lists deleted roots and restores them with their subtasks', async () => {
    const gateway = new InMemoryTaskGateway([makeTask(1, 'Projet'), makeTask(2, 'Lot', 1), makeTask(3, 'Courses')])
    await gateway.deleteTask(id(1))
    const restoredInTree: string[] = []
    const trash = useTrash(
      gateway,
      async (taskId) => {
        restoredInTree.push(taskId)
        await gateway.restoreTask(taskId)
      },
      useConfirm(),
    )

    await trash.show()
    expect(trash.open.value).toBe(true)
    expect(trash.items.value.map((task) => task.title)).toEqual(['Projet'])

    await trash.restore(id(1))
    expect(restoredInTree).toEqual([id(1)])
    expect(trash.items.value).toEqual([])
    expect((await gateway.listTasks()).tasks).toHaveLength(3)
  })

  it('purges for good only after confirmation, subtasks included', async () => {
    const gateway = new InMemoryTaskGateway([makeTask(1, 'Projet'), makeTask(2, 'Lot', 1), makeTask(3, 'Courses')])
    await gateway.deleteTask(id(1))
    const confirm = useConfirm()
    const trash = useTrash(gateway, async () => {}, confirm)
    await trash.show()

    const cancelled = trash.purge(trash.items.value[0]!)
    confirm.answer('cancel')
    expect(await cancelled).toBe(false)
    expect(trash.items.value).toHaveLength(1)

    const confirmed = trash.purge(trash.items.value[0]!)
    expect(confirm.current.value?.question.choices.find((choice) => choice.primary)?.id).toBe('purge')
    confirm.answer('purge')
    expect(await confirmed).toBe(true)
    expect(trash.items.value).toEqual([])
    expect((await gateway.listTasks()).tasks.map((task) => task.title)).toEqual(['Courses'])
  })
})
