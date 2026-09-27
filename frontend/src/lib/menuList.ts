import {
  Bot,
  LayoutDashboard,
  Settings as SettingsIcon,
  type LucideIcon,
} from 'lucide-react'

// `t` is i18next's TFunction — we keep the parameter typed loosely so this
// module doesn't pull in i18next types just to declare a signature.
type Translate = (key: string) => string

export type Submenu = {
  href: string
  label: string
  active?: boolean
}

export type MenuItem = {
  href: string
  label: string
  icon: LucideIcon
  active?: boolean
  submenus?: Submenu[]
}

export type MenuGroup = {
  groupLabel: string
  menus: MenuItem[]
}

export function getMenuList(t: Translate): MenuGroup[] {
  return [
    {
      groupLabel: '',
      menus: [
        { href: '/', label: t('nav.overview'), icon: LayoutDashboard },
        { href: '/bots', label: t('nav.bots'), icon: Bot },
        { href: '/settings', label: t('nav.settings'), icon: SettingsIcon },
      ],
    },
  ]
}
