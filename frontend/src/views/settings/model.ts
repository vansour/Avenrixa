export type SettingsSection =
  | 'account'
  | 'general'
  | 'security'
  | 'system'
  | 'maintenance'
  | 'users';

export const ADMIN_SECTIONS: SettingsSection[] = [
  'account',
  'general',
  'security',
  'system',
  'maintenance',
  'users',
];

export const USER_SECTIONS: SettingsSection[] = ['account', 'security'];

export function sectionLabel(section: SettingsSection): string {
  switch (section) {
    case 'account':
      return '账户';
    case 'general':
      return '基础设置';
    case 'security':
      return '安全';
    case 'system':
      return '系统状态';
    case 'maintenance':
      return '维护';
    case 'users':
      return '用户管理';
  }
}
