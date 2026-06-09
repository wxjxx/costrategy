import type { UserRole } from '../types/auth';

export type NavKey = 'workbench' | 'projects' | 'users' | 'settings';

export interface NavItem {
  key: NavKey;
  label: string;
  path: string;
  icon: 'LayoutDashboard' | 'FolderKanban' | 'Users' | 'Settings';
  roles: UserRole[];
}

export const NAV_ITEMS: NavItem[] = [
  {
    key: 'workbench',
    label: '工作台',
    path: '/workbench',
    icon: 'LayoutDashboard',
    roles: ['employee', 'manager', 'admin'],
  },
  {
    key: 'projects',
    label: '项目管理',
    path: '/projects',
    icon: 'FolderKanban',
    roles: ['manager', 'admin'],
  },
  {
    key: 'users',
    label: '用户管理',
    path: '/users',
    icon: 'Users',
    roles: ['admin'],
  },
  {
    key: 'settings',
    label: '系统设置',
    path: '/settings',
    icon: 'Settings',
    roles: ['admin'],
  },
];

export function getVisibleNavItems(role: UserRole): NavItem[] {
  return NAV_ITEMS.filter((item) => item.roles.includes(role));
}
