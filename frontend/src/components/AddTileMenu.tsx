import { useTranslation } from 'react-i18next'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu'
import { useGameStore } from '@/stores/gameStore'
import { useLayoutStore } from '@/stores/layoutStore'
import { ALL_TILES, type Breakpoint, type TileId } from '@/tiles/defaults'
import { playerTileTitle, seatOfPlayerTile } from '@/tiles/playerTitle'

// Stable empty fallback — see OpponentsTile note on Zustand selector identity.
const EMPTY_HIDDEN: readonly TileId[] = []

// i18n keys for the non-player tiles only. Player titles are seat-relative
// (see playerTileTitle) and can't live in a static map — a hardcoded key is
// exactly how "Self" once ended up welded to seat 2. Kept here (not in
// defaults.ts) because defaults.ts is consumed in non-React contexts
// (layout calc) where the i18n hook isn't available.
const TILE_TITLE_KEYS: Partial<Record<TileId, string>> = {
  'header':          'tile.header_full_title',
  'self-hand':       'tile.self_hand',
  'board':           'tile.board',
  'recommendations': 'tile.recommendations',
  'risk-chart':      'tile.risk_chart',
  'opponents':       'tile.opponents',
  'events':          'tile.events',
  'notifications':   'tile.notifications',
  'bot-responses':   'tile.bot_responses',
  'bot-action':      'tile.bot_action',
  'bot-show':        'tile.bot_show',
  'proxy-control':   'tile.proxy_control',
}

// Canonical order as the tiebreak so equal labels (unlikely, but two tiles
// could localize to the same word) keep a stable ordering.
const CANONICAL_RANK = new Map(ALL_TILES.map((id, i) => [id, i] as const))

export function AddTileMenu({ bp }: { bp: Breakpoint }) {
  const { t } = useTranslation()
  const hidden = useLayoutStore((s) => s.hidden[bp] ?? EMPTY_HIDDEN)
  const mode = useLayoutStore((s) => s.mode)
  const show = useLayoutStore((s) => s.show)
  const ourSeat = useGameStore((s) => s.game?.our_seat ?? null)
  const numPlayers = useGameStore((s) => s.game?.num_players ?? null)

  const label = (id: TileId): string => {
    const seat = seatOfPlayerTile(id)
    if (seat != null) {
      return playerTileTitle(t, seat, ourSeat, numPlayers ?? (mode === '3p' ? 3 : 4))
    }
    return t(TILE_TITLE_KEYS[id] ?? id)
  }

  // 3p: never offer player-3 in the Add menu. Listed alphabetically by the
  // localized label (player names included), not by hide-insertion order.
  const hiddenIds = mode === '3p' ? hidden.filter((id) => id !== 'player-3') : hidden
  const offered = hiddenIds
    .map((id) => ({ id, label: label(id) }))
    .sort(
      (a, b) =>
        a.label.localeCompare(b.label) ||
        (CANONICAL_RANK.get(a.id) ?? 0) - (CANONICAL_RANK.get(b.id) ?? 0),
    )

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-1.5 text-xs">
          <Plus className="h-3.5 w-3.5" />
          {t('common.add_tile')}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel className="text-xs">{t('common.hidden_tiles')}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {offered.length === 0 ? (
          <DropdownMenuItem disabled className="text-xs text-muted-foreground">
            {t('common.all_tiles_visible')}
          </DropdownMenuItem>
        ) : offered.map(({ id, label }) => (
          <DropdownMenuItem key={id} onClick={() => show(id, bp)} className="text-xs">
            {label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
