/**
 * Typed client for the ZoneFlow loopback API.
 *
 * Wire types mirror `src/api/dto.rs` and `src/config.rs` exactly — field names
 * are snake_case because that is what serde emits. The bootstrap exchanges the
 * one-time token from `location.hash` for an HttpOnly `zf_session` cookie; every
 * later request is same-origin and relies on that cookie (EventSource cannot
 * set a bearer header).
 */

/* ------------------------------- wire types ------------------------------- */

export interface RectDto {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** `config::LaunchMode` — `#[serde(rename_all = "lowercase")]`. */
export type LaunchMode = 'launch' | 'attach';

/** `config::TabBehavior` — `#[serde(rename_all = "snake_case")]`. */
export type TabBehavior = 'focus_or_new' | 'reuse_blank' | 'ignore';

/** `config::CloseAction` — `#[serde(rename_all = "lowercase")]`. */
export type CloseAction = 'ask' | 'minimize' | 'exit';

export interface ZoneDto {
  id: number;
  label: string;
  exe: string;
  path: string;
  launch_mode: LaunchMode;
  title_match: string;
  url: string;
  tab_behavior: TabBehavior;
  macro_enabled: boolean;
  trigger_title: string;
  macro_text: string;
  notes: string;
  key_delay: number;
  /** Normalized percent (0..100) of the work area. */
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Profile {
  name: string;
  hotkey: string;
  zones: ZoneDto[];
}

export interface ResolutionPreset {
  label: string;
  width: number;
  work_height: number;
}

/** Mirrors `dto::SettingsDto` / `config::Settings`. */
export interface SettingsDto {
  snap_points: number[];
  grid_cols: number;
  grid_rows: number;
  min_zone_pct: number;
  resize_step_pct: number;
  boundary_tolerance_pct: number;
  undo_depth: number;
  display_debounce_ms: number;
  overlay_inactive_alpha: number;
  overlay_active_alpha: number;
  dwm_snap_retries: number;
  min_window_px: number;
  zone_tints: string[];
  auto_launch_on_startup: boolean;
  resolutions: ResolutionPreset[];
  app_list_cap: number;
  running_apps_ttl_secs: number;
  taskbar_h: number;
  stage_lower_h: number;
}

/** Mirrors `config::AppConfig` (the `fallback` field is `#[serde(skip)]`). */
export interface AppConfig {
  schema_version: number;
  profiles: Record<string, Profile>;
  active_profile: string;
  gap: number;
  close_action: CloseAction;
  auto_start: boolean;
  hotkeys: Record<string, string>;
  settings: SettingsDto;
}

export interface ProfileSummary {
  name: string;
  zone_count: number;
  applied: boolean;
  selected: boolean;
}

export interface AppEntryDto {
  label: string;
  exe: string;
  path: string;
}

export interface HotkeyDto {
  name: string;
  binding: string;
}

export interface LogLineDto {
  seq: number;
  level: string;
  message: string;
}

export interface EngineStatusDto {
  work_area: RectDto;
  zone_count: number;
  applied: string;
  enabled: boolean;
}

/** Mirrors `dto::StateSnapshot` — the payload of `GET /api/state`. */
export interface StateSnapshot {
  draft: AppConfig;
  dirty: boolean;
  applied: string;
  selected: string;
  enabled: boolean;
  selected_zone: number;
  profiles: ProfileSummary[];
  zones: ZoneDto[];
  settings: SettingsDto;
  hotkeys: HotkeyDto[];
  logs: LogLineDto[];
  status: EngineStatusDto;
  can_undo: boolean;
  can_redo: boolean;
}

/** One geometry-only update in a batch. Mirrors `dto::ZoneGeometryUpdateDto`. */
export interface ZoneGeometryUpdateDto {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SplitDto {
  axis: string;
  ratio: number;
}

export interface MergeDto {
  source_id: number;
  target_id: number;
}

export interface AssignDto {
  label: string;
  exe: string;
  path: string;
}

export interface PreviewDto {
  profile: string;
  steps: string[];
  executed: boolean;
}

export interface ApplyResultDto {
  profile: string;
  message: string;
  launched: boolean;
}

/** Returned by `/api/apply` and `/api/launch-arrange` when layout is disabled. */
export interface InactiveResult {
  ok: boolean;
  applied: false;
  inactive: true;
  reason: string;
  profile: string;
}

export interface SplitResult {
  first: ZoneDto;
  second: ZoneDto;
}

export interface OkResult {
  ok: boolean;
}

/* ------------------------------- error shape ------------------------------ */

export interface ApiErrorDetail {
  code: string;
  message: string;
}

/** `{ "error": { "code": ..., "message": ... } }` from `api::ApiError`. */
export interface ApiErrorBody {
  error: ApiErrorDetail;
}

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

/* ------------------------------- bootstrap -------------------------------- */

export type BootstrapState =
  | 'idle'
  | 'exchanging'
  | 'ready'
  | 'missing-token'
  | 'rejected'
  | 'failed';

let bootstrapState: BootstrapState = 'idle';
let bootstrapMessage = '';
let exchangeAttempts = 0;

export function getBootstrapState(): BootstrapState {
  return bootstrapState;
}

export function getBootstrapMessage(): string {
  return bootstrapMessage;
}

/** A retry is offered only after the first failed exchange (plan §1.1). */
export function canRetryBootstrap(): boolean {
  return bootstrapState === 'failed' && exchangeAttempts < 2;
}

function readTokenFromHash(): string | null {
  const hash = window.location.hash.replace(/^#/, '');
  if (!hash) return null;
  return new URLSearchParams(hash).get('token');
}

/**
 * Exchange the fragment token for the session cookie. The fragment is cleared
 * only on success so a retry can re-read it. Returns true when ready.
 */
export async function bootstrap(): Promise<boolean> {
  const token = readTokenFromHash();
  if (!token) {
    bootstrapState = 'missing-token';
    bootstrapMessage = 'Open the editor from the ZoneFlow tray menu.';
    return false;
  }

  exchangeAttempts += 1;
  bootstrapState = 'exchanging';
  bootstrapMessage = '';

  try {
    const response = await fetch('/api/session', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { Authorization: `Bearer ${token}` },
    });

    if (response.status === 403) {
      bootstrapState = 'rejected';
      bootstrapMessage = 'Session rejected — reopen from the tray.';
      return false;
    }
    if (!response.ok) {
      bootstrapState = 'failed';
      bootstrapMessage =
        exchangeAttempts < 2
          ? 'Could not open a session. Retry, or reopen from the tray.'
          : 'Could not open a session. Reopen the editor from the ZoneFlow tray menu.';
      return false;
    }

    // Success: drop the token from the address bar/fragment.
    window.history.replaceState(null, '', window.location.pathname);
    bootstrapState = 'ready';
    bootstrapMessage = '';
    return true;
  } catch (error) {
    bootstrapState = 'failed';
    const detail = error instanceof Error ? error.message : String(error);
    bootstrapMessage =
      exchangeAttempts < 2
        ? `Session exchange failed (${detail}). Retry, or reopen from the tray.`
        : `Session exchange failed (${detail}). Reopen the editor from the ZoneFlow tray menu.`;
    return false;
  }
}

/** Retry the exchange once, then instruct the user to reopen from the tray. */
export async function retryBootstrap(): Promise<boolean> {
  if (!canRetryBootstrap()) return false;
  return bootstrap();
}

/* ------------------------------ session expiry ---------------------------- */

let sessionExpiredHandler: (() => void) | null = null;

/** Register the store callback invoked on any 401 after bootstrap. */
export function setSessionExpiredHandler(handler: () => void): void {
  sessionExpiredHandler = handler;
}

/* ------------------------------ fetch wrapper ----------------------------- */

export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const hasBody = init.body !== undefined && init.body !== null;
  const response = await fetch(path, {
    ...init,
    credentials: 'same-origin',
    headers: {
      ...(hasBody ? { 'Content-Type': 'application/json' } : {}),
      ...(init.headers ?? {}),
    },
  });

  if (!response.ok) {
    let detail: ApiErrorDetail | null = null;
    try {
      const body = (await response.json()) as ApiErrorBody;
      if (body?.error && typeof body.error.code === 'string') {
        detail = body.error;
      }
    } catch {
      // Non-JSON error body — fall back to the status text.
    }
    const code = detail?.code ?? 'http_error';
    const message = detail?.message ?? `Request failed (${response.status})`;

    // 401 after bootstrap means the process restarted (per-launch token).
    // Keep local state/draft; surface a banner rather than clearing anything.
    if (response.status === 401) {
      sessionExpiredHandler?.();
    }

    throw new ApiError(response.status, code, message);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

/* ------------------------------- endpoints -------------------------------- */

function withDiscard(path: string, discardDirty: boolean): string {
  return `${path}?discard_dirty=${discardDirty ? 'true' : 'false'}`;
}

function encoded(name: string): string {
  return encodeURIComponent(name);
}

export const api = {
  getState: () => apiFetch<StateSnapshot>('/api/state'),
  getStatus: () => apiFetch<EngineStatusDto>('/api/status'),
  shutdown: () => apiFetch<OkResult>('/api/shutdown', { method: 'POST' }),

  listProfiles: () => apiFetch<ProfileSummary[]>('/api/profiles'),
  createProfile: (name: string) =>
    apiFetch<{ name: string }>('/api/profiles', {
      method: 'POST',
      body: JSON.stringify({ name }),
    }),
  renameProfile: (name: string, newName: string) =>
    apiFetch<OkResult>(`/api/profiles/${encoded(name)}`, {
      method: 'PUT',
      body: JSON.stringify({ new_name: newName }),
    }),
  deleteProfile: (name: string) =>
    apiFetch<OkResult>(`/api/profiles/${encoded(name)}`, { method: 'DELETE' }),
  duplicateProfile: (name: string, newName: string) =>
    apiFetch<OkResult>(`/api/profiles/${encoded(name)}/duplicate`, {
      method: 'POST',
      body: JSON.stringify({ new_name: newName }),
    }),
  loadProfile: (name: string, discardDirty = false) =>
    apiFetch<OkResult>(withDiscard(`/api/profiles/${encoded(name)}/load`, discardDirty), {
      method: 'POST',
    }),
  selectProfile: (name: string, discardDirty = false) =>
    apiFetch<OkResult>(withDiscard(`/api/profiles/${encoded(name)}/select`, discardDirty), {
      method: 'POST',
    }),
  applyProfile: (name: string, discardDirty = false) =>
    apiFetch<OkResult | ApplyResultDto | InactiveResult>(
      withDiscard(`/api/profiles/${encoded(name)}/apply`, discardDirty),
      { method: 'POST' },
    ),

  listZones: () => apiFetch<ZoneDto[]>('/api/zones'),
  createZone: (zone: ZoneDto) =>
    apiFetch<ZoneDto>('/api/zones', { method: 'POST', body: JSON.stringify(zone) }),
  updateZone: (id: number, zone: ZoneDto) =>
    apiFetch<ZoneDto>(`/api/zones/${id}`, { method: 'PUT', body: JSON.stringify(zone) }),
  /** Apply several geometry updates as one undo step (e.g. a divider drag). */
  updateZonesBatch: (updates: ZoneGeometryUpdateDto[]) =>
    apiFetch<ZoneDto[]>('/api/zones/batch', { method: 'PUT', body: JSON.stringify(updates) }),
  deleteZone: (id: number) => apiFetch<OkResult>(`/api/zones/${id}`, { method: 'DELETE' }),
  splitZone: (id: number, spec: SplitDto) =>
    apiFetch<SplitResult>(`/api/zones/${id}/split`, {
      method: 'POST',
      body: JSON.stringify(spec),
    }),
  mergeZones: (spec: MergeDto) =>
    apiFetch<ZoneDto>('/api/zones/merge', { method: 'POST', body: JSON.stringify(spec) }),
  assignZone: (id: number, spec: AssignDto) =>
    apiFetch<ZoneDto>(`/api/zones/${id}/assign`, {
      method: 'POST',
      body: JSON.stringify(spec),
    }),
  unassignZone: (id: number) =>
    apiFetch<OkResult>(`/api/zones/${id}/assignment`, { method: 'DELETE' }),
  undo: () => apiFetch<OkResult>('/api/undo', { method: 'POST' }),
  redo: () => apiFetch<OkResult>('/api/redo', { method: 'POST' }),

  listApps: (source: 'installed' | 'running' = 'installed') =>
    apiFetch<AppEntryDto[]>(`/api/apps?source=${source}`),
  browseApps: () => apiFetch<AppEntryDto[]>('/api/apps/browse', { method: 'POST' }),

  save: () => apiFetch<OkResult>('/api/save', { method: 'POST' }),
  apply: () => apiFetch<ApplyResultDto | InactiveResult>('/api/apply', { method: 'POST' }),
  launchArrange: () =>
    apiFetch<ApplyResultDto | InactiveResult>('/api/launch-arrange', { method: 'POST' }),
  preview: () => apiFetch<PreviewDto>('/api/preview', { method: 'POST' }),
  revert: () => apiFetch<OkResult>('/api/revert', { method: 'POST' }),
  disable: () => apiFetch<OkResult>('/api/disable', { method: 'POST' }),
  enable: () => apiFetch<OkResult>('/api/enable', { method: 'POST' }),

  getSettings: () => apiFetch<SettingsDto>('/api/settings'),
  setSettings: (settings: SettingsDto) =>
    apiFetch<OkResult>('/api/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  getHotkeys: () => apiFetch<HotkeyDto[]>('/api/hotkeys'),
  setHotkeys: (hotkeys: Record<string, string>) =>
    apiFetch<OkResult>('/api/hotkeys', { method: 'PUT', body: JSON.stringify(hotkeys) }),
  getLogs: (since = 0) => apiFetch<LogLineDto[]>(`/api/logs?since=${since}`),
};
