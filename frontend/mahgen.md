## Mahgen tile sizing — what works and what doesn't

The `<mah-gen>` web component has a frustrating sizing model. We learned this
the hard way; everything below is required reading before touching tile size
code in `src/lib/mahgenRegistry.ts`.

### What `mah-gen` actually does

Source: [`eric200203/mahgen`](https://github.com/eric200203/mahgen) `src/MahgenElement.ts`. The element constructor:

```ts
constructor() {
  super();
  const root = this.attachShadow({ mode: 'open' });
  this.img = document.createElement('img');
  root.appendChild(this.img);
}
```

A single `<img>` is appended to an open shadow root. `attributeChangedCallback`
on `data-seq` calls `Mahgen.render(seq, river)` which produces a **single
base64 PNG** (composited by `JimpWorker.ts`) and assigns it to `this.img.src`.
The PNG load is async.

### Why every "obvious" approach fails

1. **`zoom: 0.x` on the host.** Doesn't apply reliably in WebKitGTK (the Tauri
   webview on Linux). Even when it works in Chromium, it's not portable.

4. **`max-width: 100%` on the inner `<img>` (set via shadow injection).**
   Circular dependency:
   - `<img>`'s containing block resolves to the shadow root.
   - Shadow root inherits the host's content box.
   - Host (`mah-gen`, `display: inline-block`) sizes to its content (= the img).
   - So `100%` resolves to the img's own width — no constraint.

5. **`img.style.width = "100%"` (set via shadow injection).** Same circular
   problem as above.

6. **`transform: scale()` on the host.** Scales visually but doesn't change
   the layout box, so siblings don't reflow and tiles overlap.

### The approach that works

Set explicit **pixel** dimensions on **both** the host element AND the inner
`<img>`. Setting the host's box to a fixed size breaks the circularity, and
explicit pixel dims on the img bypass the percentage-resolution issue entirely.

```js
// Open shadow → reachable from JS even though CSS can't penetrate.
const img = el.shadowRoot.querySelector('img');
el.style.width = w + 'px';
el.style.height = h + 'px';
img.style.width = w + 'px';
img.style.height = h + 'px';
img.style.objectFit = 'contain';
img.style.display = 'block';
```

### Native tile dimensions (mahgen `res/` assets)

```
Normal portrait tile:    70 × 100   (e.g. 1m.png)
Horizontal (sideways):   92 ×  77   (e.g. _1m.png — used for chi/pon/riichi-cut)
Stacked (called):        92 × 146   (e.g. =1m.png — used for ankan back tile)
Space:                  ~70 × ...   (used as filler in melds)
```

### Sizing modes (current `SIZE_CTX` in `mahgenRegistry.ts`)

| Mode | Formula | When to use |
|---|---|---|
| `linear` | `h = base * cw / ref` (clamped); `w = h * nw / nh` | Rows in the bot-show list: multiple mahgens in the same row, container much wider than the image |
| `fill-height` | `h = clamp(ch - pad, [min, max])`; `w = h * nw / nh` | The overlay's rows, where the container's *height* is the scarce axis and the tile grows with the window |

### Async timing

Three things are async:

1. **Custom element upgrade.** `customElements.define('mah-gen', ...)` runs
   when the mahgen UMD script executes. If you create `<mah-gen>` before that
   completes, `el.shadowRoot` is `null`. → RAF retry until ready.

2. **DOM connect.** `registerMahgen` may be called before the element is
   appended to the DOM, so `el.isConnected` is `false` on the first sizing
   pass. → Retry with a counter (we use up to 6 RAF), then drop the entry.
   Don't delete on first detach — that was the bug that kept tiles at native
   size for several iterations.

3. **Image load.** `img.src = base64` triggers an async decode. `naturalWidth`
   / `naturalHeight` read 0 until the `load` event fires. → Attach a single
   `load` listener that recomputes size on every src swap (mahgen swaps src
   each `data-seq` change).

```js
if (!img._akagiOnLoad) {
  img._akagiOnLoad = true;
  img.addEventListener('load', () => applyMahgenSize(el));
}
```

### Container resize

A `ResizeObserver` watches the size container of every registered mahgen so
that a window resize (or any container size change) reflows the tiles
automatically.

Each container is observed once (RO calls are idempotent). The callback finds
all registry entries whose `container` matches the resized element and
re-applies size for each.

### Empty sequences

When `data-seq` is unset or `""`, the host has explicit pixel dims but the
inner `<img>` has no `src` (mahgen returns early in `genImage` for `seq===null`,
or sets `img.src = ''` on parse error). The browser renders this as an empty
sized box that looks like a gray placeholder rectangle.

Fix: `el.style.display = 'none'` when the sequence is empty. `setMahgenSeq`
unsets it on the next non-empty sequence:

```js
const seq = el.getAttribute('data-seq');
if (!seq) { el.style.display = 'none'; return; }
el.style.display = '';   // restore CSS rule's `display: inline-block`
```

### Cleanup

The retry counter handles "element will be attached momentarily" but it does
**not** handle "element was deliberately removed via `innerHTML = ''`". For
those code paths, call `unregisterMahgen(m)` for each `<mah-gen>` before the
wipe:

```js
list.querySelectorAll('mah-gen').forEach((m) => unregisterMahgen(m));
list.innerHTML = '';
```

Otherwise the registry leaks an entry per old row every time the list
rebuilds.

### Quick checklist for adding a new mahgen-bearing widget

1. Pick a sizing mode (`linear` / `fill-height`) and add an entry
   to `SIZE_CTX` if needed.
2. Pick a sizing **container** — an ancestor whose `clientWidth` is what
   bounds the tiles. Don't pick a content-sized element (its width depends on
   the image, which causes self-feeding loops).
3. Call `registerMahgen(el, kind, container)` right after appending the
   `<mah-gen>` to its parent.
4. If your widget rebuilds via `innerHTML = ''`, call `unregisterMahgen` on
   each old `<mah-gen>` before the wipe.
5. Use `setMahgenSeq(el, seq)` (not `setAttribute('data-seq', ...)` directly)
   to get the opacity crossfade and the post-load size re-apply.
