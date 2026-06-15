import { describe, it, expect, vi } from 'vitest'
import { eventBus } from '../services/eventBus'

describe('eventBus', () => {
  it('on and emit work correctly', () => {
    const handler = vi.fn()
    eventBus.on('test:event', handler)
    eventBus.emit('test:event', 'arg1', 42)
    expect(handler).toHaveBeenCalledWith('arg1', 42)
    eventBus.clear('test:event')
  })

  it('once fires only once', () => {
    const handler = vi.fn()
    eventBus.once('test:once', handler)
    eventBus.emit('test:once')
    eventBus.emit('test:once')
    expect(handler).toHaveBeenCalledTimes(1)
    eventBus.clear('test:once')
  })

  it('off removes handler', () => {
    const handler = vi.fn()
    eventBus.on('test:off', handler)
    eventBus.off('test:off', handler)
    eventBus.emit('test:off')
    expect(handler).not.toHaveBeenCalled()
    eventBus.clear('test:off')
  })

  it('clear removes all handlers for event', () => {
    const handler = vi.fn()
    eventBus.on('test:clear', handler)
    eventBus.clear('test:clear')
    eventBus.emit('test:clear')
    expect(handler).not.toHaveBeenCalled()
  })

  it('clear without argument removes all events', () => {
    const handler = vi.fn()
    eventBus.on('a', handler)
    eventBus.on('b', handler)
    eventBus.clear()
    eventBus.emit('a')
    eventBus.emit('b')
    expect(handler).not.toHaveBeenCalled()
  })

  it('listeners returns handler count', () => {
    const handler = vi.fn()
    eventBus.on('test:count', handler)
    expect(eventBus.listeners('test:count')).toBe(1)
    eventBus.clear('test:count')
    expect(eventBus.listeners('test:count')).toBe(0)
  })

  it('on returns unsubscribe function', () => {
    const handler = vi.fn()
    const unsub = eventBus.on('test:unsub', handler)
    unsub()
    eventBus.emit('test:unsub')
    expect(handler).not.toHaveBeenCalled()
    eventBus.clear('test:unsub')
  })
})
