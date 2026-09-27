// Convert an array of mjai tile strings to a mahgen DSL string.
// Used by the overlay's bot-show list to render tile sequences.
//
// Tile-index layout (0..33) mirrors mahgen's: 0..8 1m..9m, 9..17 1p..9p,
// 18..26 1s..9s, 27..33 E S W N P F C. Red fives (5mr/5pr/5sr) collapse
// onto the plain 5.

const Z_INDEX: Record<string, number> = { E: 1, S: 2, W: 3, N: 4, P: 5, F: 6, C: 7 }

export function mjaiToMahgen(tiles: string[] | null | undefined): string {
  if (!tiles || !tiles.length) return ''
  let backs = 0
  const m: number[] = []
  const p: number[] = []
  const s: number[] = []
  const z: number[] = []
  for (const tile of tiles) {
    if (!tile || tile === '?') {
      backs++
      continue
    }
    if (Z_INDEX[tile]) {
      z.push(Z_INDEX[tile])
      continue
    }
    const isRed = tile.endsWith('r')
    const suit = isRed ? tile[tile.length - 2] : tile[tile.length - 1]
    const num = isRed ? 0 : parseInt(tile[0], 10)
    if (suit === 'm') m.push(num)
    else if (suit === 'p') p.push(num)
    else if (suit === 's') s.push(num)
  }
  const sortKey = (a: number, b: number) => (a === 0 ? 5.5 : a) - (b === 0 ? 5.5 : b)
  m.sort(sortKey); p.sort(sortKey); s.sort(sortKey); z.sort((a, b) => a - b)
  let out = ''
  if (m.length) out += m.join('') + 'm'
  if (p.length) out += p.join('') + 'p'
  if (s.length) out += s.join('') + 's'
  if (z.length) out += z.join('') + 'z'
  if (backs > 0) out += '0'.repeat(backs) + 'z'
  return out
}
