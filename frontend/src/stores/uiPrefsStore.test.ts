import { beforeEach, describe, expect, it, vi } from 'vitest'

// Node 22+ ships an experimental `localStorage` global that is undefined
// without `--localstorage-file` and shadows jsdom's, so install an explicit
// in-memory stand-in the store and the assertions both see.
function stubLocalStorage() {
  const backing = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => backing.get(k) ?? null,
    setItem: (k: string, v: string) => void backing.set(k, String(v)),
    removeItem: (k: string) => void backing.delete(k),
    clear: () => backing.clear(),
  })
  return backing
}

// The store reads localStorage at module-init, so each test re-imports a
// fresh copy after seeding storage to exercise the load path.
async function freshStore() {
  vi.resetModules()
  const mod = await import('./uiPrefsStore')
  return mod
}

describe('uiPrefsStore UI scale', () => {
  beforeEach(() => {
    stubLocalStorage()
  })

  it('defaults to 1.0 on first launch', async () => {
    const { useUiPrefsStore } = await freshStore()
    expect(useUiPrefsStore.getState().scale).toBe(1.0)
  })

  it('persists the scale across restarts', async () => {
    const { useUiPrefsStore } = await freshStore()
    useUiPrefsStore.getState().setScale(1.2)
    expect(localStorage.getItem('akagi.ui.scale')).toBe('1.2')

    const restarted = await freshStore()
    expect(restarted.useUiPrefsStore.getState().scale).toBe(1.2)
  })

  it('clamps out-of-range values', async () => {
    const { useUiPrefsStore, SCALE_MIN, SCALE_MAX } = await freshStore()
    useUiPrefsStore.getState().setScale(9)
    expect(useUiPrefsStore.getState().scale).toBe(SCALE_MAX)
    useUiPrefsStore.getState().setScale(0.1)
    expect(useUiPrefsStore.getState().scale).toBe(SCALE_MIN)
  })
})
