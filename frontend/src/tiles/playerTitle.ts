import type { TFunction } from 'i18next'
import { relativeKind, type RelativeKind } from '@/lib/format'
import type { TileId } from '@/tiles/defaults'

// Positional title per relative seat — mirrors BoardTile's KIND_EDGE
// (shimocha sits on our right, kamicha on our left, toimen across).
const KIND_TITLE_KEY: Record<RelativeKind, string> = {
  self: 'tile.player_self',
  shimocha: 'tile.player_right',
  toimen: 'tile.player_across',
  kamicha: 'tile.player_left',
}

/** Localized title of a player tile. With no hero seat known, falls back to
 *  the neutral "Player N" — relativeKind() collapses every seat to 'self'
 *  there, which would label all four panels "Self". */
export function playerTileTitle(
  t: TFunction,
  seat: number,
  ourSeat: number | null,
  numPlayers: number,
): string {
  if (ourSeat == null) return t('tile.player_n', { n: seat + 1 })
  return t(KIND_TITLE_KEY[relativeKind(seat, ourSeat, numPlayers)])
}

/** Seat index carried by a `player-N` tile id, or null for other tiles. */
export function seatOfPlayerTile(id: TileId): number | null {
  return id.startsWith('player-') ? Number(id.slice('player-'.length)) : null
}
