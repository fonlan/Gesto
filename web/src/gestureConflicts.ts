import type { GestureBinding } from './types'

const DIRECTION_BUTTONS = new Set(['U', 'D', 'L', 'R'])

export interface GestureConflictScope {
  duplicateGestures: string[]
  byIndex: Record<number, string>
}

export const normalizeGesture = (value: string) =>
  value
    .toUpperCase()
    .split('')
    .filter((item) => DIRECTION_BUTTONS.has(item))
    .join('')

export const collectGestureConflicts = (bindings: GestureBinding[]): GestureConflictScope => {
  const indicesByGesture = new Map<string, number[]>()

  bindings.forEach((binding, index) => {
    const gesture = normalizeGesture(binding.gesture)
    if (!gesture) {
      return
    }

    const indices = indicesByGesture.get(gesture) ?? []
    indices.push(index)
    indicesByGesture.set(gesture, indices)
  })

  const duplicateGestures: string[] = []
  const byIndex: Record<number, string> = {}

  indicesByGesture.forEach((indices, gesture) => {
    if (indices.length < 2) {
      return
    }

    duplicateGestures.push(gesture)
    indices.forEach((index) => {
      byIndex[index] = gesture
    })
  })

  duplicateGestures.sort()

  return {
    duplicateGestures,
    byIndex
  }
}