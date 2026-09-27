// Mahgen tile sizing for the suggestion overlay's bot-show rows.
// See frontend/mahgen.md for the full background on why every "obvious"
// CSS-only approach fails. The TL;DR: mahgen renders into an open shadow
// root; setting host + inner <img> dimensions in pixels is the only path
// that survives the WebKitGTK + circular-containing-block constraints.

export type MahgenKind = 'bot-show' | 'overlay-show'

type SizeCtx =
  | { mode: 'linear'; base: number; ref: number; min: number; max: number }
  // The only height-driven mode. Every other one reads the container's *width*,
  // which is right inside a scrolling list but wrong for a small free-floating
  // window: there, the height is the scarce axis, and a tile that ignores it
  // leaves a dead band under the last row no matter how tall the user drags the
  // window. `pad` is the breathing room left inside the row.
  | { mode: 'fill-height'; pad: number; min: number; max: number }

const SIZE_CTX: Record<MahgenKind, SizeCtx> = {
  // bot-show: per-row tile group inside the BotShowList. `ref` is the row
  // width; the cap keeps chi/pon melds legible without dwarfing the label.
  'bot-show': { mode: 'linear', base: 30, ref: 260, min: 22, max: 64 },
  // overlay-show: the same rows in the always-on-top overlay window. The
  // container here is the row itself (not the list), and the rows split the
  // window's height between them — so the tile grows with the window and the
  // list always reaches the bottom edge. `max` is generous on purpose: someone
  // who drags the overlay large wants big, legible tiles.
  // `min` is low enough that 5 rows in a window shrunk to `MIN_HEIGHT` still fit
  // rather than getting cropped by the row's overflow.
  'overlay-show': { mode: 'fill-height', pad: 8, min: 20, max: 120 },
}

type MahgenImg = HTMLImageElement & { _akagiOnLoad?: boolean }
type MahgenEl = HTMLElement & { shadowRoot: ShadowRoot | null }

type Entry = {
  kind: MahgenKind
  container: HTMLElement | null
  retries: number
}

const registry = new Map<MahgenEl, Entry>()

export function applyMahgenSize(el: MahgenEl): void {
  const entry = registry.get(el)
  if (!entry) return

  if (!el.isConnected) {
    entry.retries += 1
    if (entry.retries > 6) {
      registry.delete(el)
      return
    }
    requestAnimationFrame(() => applyMahgenSize(el))
    return
  }
  entry.retries = 0

  const root = el.shadowRoot
  if (!root) {
    requestAnimationFrame(() => applyMahgenSize(el))
    return
  }

  const img = root.querySelector('img') as MahgenImg | null
  if (!img) {
    requestAnimationFrame(() => applyMahgenSize(el))
    return
  }

  if (!img._akagiOnLoad) {
    img._akagiOnLoad = true
    img.addEventListener('load', () => applyMahgenSize(el))
  }

  const seq = el.getAttribute('data-seq')
  if (!seq) {
    el.style.display = 'none'
    return
  }
  el.style.display = ''

  const cfg = SIZE_CTX[entry.kind]
  const cw = entry.container?.clientWidth ?? 200
  const nw = img.naturalWidth
  const nh = img.naturalHeight
  const aspectKnown = nw > 0 && nh > 0

  let w: number
  let h: number

  if (cfg.mode === 'fill-height') {
    // The row's height is set by the flex layout, not by this tile, so growing
    // the tile can't feed back into the measurement.
    const ch = entry.container?.clientHeight ?? cfg.min + cfg.pad
    h = Math.max(cfg.min, Math.min(cfg.max, ch - cfg.pad))
    w = aspectKnown ? (h * nw) / nh : h * 0.75
  } else {
    h = cfg.base * (cw / cfg.ref)
    h = Math.max(cfg.min, Math.min(cfg.max, h))
    w = aspectKnown ? (h * nw) / nh : cfg.base
  }

  el.style.width = `${w}px`
  el.style.height = `${h}px`
  el.style.flex = '0 0 auto'
  img.style.width = `${w}px`
  img.style.height = `${h}px`
  img.style.objectFit = 'contain'
  img.style.display = 'block'
}

const ro = typeof ResizeObserver !== 'undefined'
  ? new ResizeObserver((entries) => {
      const containers = new Set(entries.map((e) => e.target))
      for (const [el, ent] of registry) {
        if (ent.container && containers.has(ent.container)) applyMahgenSize(el)
      }
    })
  : null

const observedContainers = new WeakSet<Element>()

export function registerMahgen(el: MahgenEl, kind: MahgenKind, container: HTMLElement | null): void {
  registry.set(el, { kind, container, retries: 0 })
  if (container && ro && !observedContainers.has(container)) {
    ro.observe(container)
    observedContainers.add(container)
  }
  applyMahgenSize(el)
}

export function unregisterMahgen(el: MahgenEl): void {
  registry.delete(el)
}

let resizeRaf = 0
function scheduleResize(): void {
  if (resizeRaf) return
  resizeRaf = requestAnimationFrame(() => {
    resizeRaf = 0
    for (const el of registry.keys()) applyMahgenSize(el)
  })
}

if (typeof window !== 'undefined') {
  window.addEventListener('resize', scheduleResize)
}

export function setMahgenSeq(el: MahgenEl, seq: string): void {
  const next = seq ?? ''
  if ((el.getAttribute('data-seq') ?? '') === next) return
  el.style.opacity = '0'
  requestAnimationFrame(() => {
    el.setAttribute('data-seq', next)
    requestAnimationFrame(() => {
      el.style.opacity = '1'
      applyMahgenSize(el)
    })
  })
}
