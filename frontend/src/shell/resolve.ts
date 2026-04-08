import type {
  BootstrapStatusResponse,
  InstallStatusResponse,
} from '../api/types';

export type ShellMode =
  | 'booting'
  | 'bootstrap'
  | 'install'
  | 'login'
  | 'dashboard'
  | 'init-error';

export interface ResolveShellModeInput {
  isBootReady: boolean;
  isAuthenticated: boolean;
  bootstrapStatus: BootstrapStatusResponse | null;
  installStatus: InstallStatusResponse | null;
  initError?: string | null;
}

const DASHBOARD_PATHS = new Set(['/', '/upload', '/history', '/api', '/settings']);
const DEFAULT_SITE_NAME = 'Avenrixa';

export function resolveShellMode(input: ResolveShellModeInput): ShellMode {
  if (!input.isBootReady) {
    return 'booting';
  }

  if (input.initError) {
    return 'init-error';
  }

  if (input.bootstrapStatus?.mode === 'bootstrap') {
    return 'bootstrap';
  }

  if (input.installStatus && !input.installStatus.installed) {
    return 'install';
  }

  if (input.installStatus?.installed) {
    return input.isAuthenticated ? 'dashboard' : 'login';
  }

  return 'init-error';
}

export function resolvePreferredRoute(
  mode: ShellMode,
  currentPath: string,
): string | null {
  switch (mode) {
    case 'bootstrap':
      return currentPath === '/bootstrap' ? null : '/bootstrap';
    case 'install':
      return currentPath === '/install' ? null : '/install';
    case 'login':
      return currentPath === '/login' ? null : '/login';
    case 'dashboard':
      return DASHBOARD_PATHS.has(currentPath) ? null : '/';
    default:
      return null;
  }
}

export function displaySiteName(value: string | null | undefined): string {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : DEFAULT_SITE_NAME;
}
