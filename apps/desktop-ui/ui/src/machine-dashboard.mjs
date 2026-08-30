/**
 * @typedef {Object} MachinePresence
 * @property {string} machineId
 * @property {string} hostname
 * @property {string} osVersion
 * @property {string | null} lastSeenAt
 * @property {number} onlineSeconds
 * @property {'online' | 'offline'} status
 */

const MACHINES_PATH = '/api/v1/machines';

function asNonEmptyString(value, fieldName) {
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`Invalid machine response: ${fieldName} is required`);
  }
  return value;
}

export class DashboardAuthenticationError extends Error {
  constructor() {
    super('Sign in is required to view connected machines.');
    this.name = 'DashboardAuthenticationError';
  }
}

/**
 * Convert the server's machine DTO into a stable renderer model. The server
 * owns presence calculation; the UI must not infer it from the browser clock.
 *
 * @param {unknown} payload
 * @returns {MachinePresence[]}
 */
export function normalizeMachinesResponse(payload) {
  const records = Array.isArray(payload)
    ? payload
    : payload && typeof payload === 'object' && Array.isArray(payload.machines)
      ? payload.machines
      : null;

  if (records === null) {
    throw new Error('Invalid machine response: expected an array of machines');
  }

  return records.map((record) => {
    if (!record || typeof record !== 'object') {
      throw new Error('Invalid machine response: machine must be an object');
    }
    if (typeof record.is_online !== 'boolean') {
      throw new Error('Invalid machine response: is_online must be boolean');
    }
    const onlineSeconds = Number(record.online_duration_secs);
    if (!Number.isFinite(onlineSeconds) || onlineSeconds < 0) {
      throw new Error('Invalid machine response: online_seconds must be non-negative');
    }

    return {
      machineId: asNonEmptyString(record.machine_id, 'machine_id'),
      hostname: asNonEmptyString(record.hostname, 'hostname'),
      osVersion: asNonEmptyString(record.os_version, 'os_version'),
      lastSeenAt: typeof record.last_seen_at === 'string' ? record.last_seen_at : null,
      onlineSeconds: Math.floor(onlineSeconds),
      status: record.is_online ? 'online' : 'offline',
    };
  });
}

/** @param {number} seconds */
export function formatOnlineDuration(seconds) {
  const safeSeconds = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
}

/**
 * @param {typeof fetch} fetchImpl
 * @returns {Promise<MachinePresence[]>}
 */
export async function fetchMachines(fetchImpl = fetch) {
  const response = await fetchImpl(MACHINES_PATH, { credentials: 'include' });
  if (response.status === 401 || response.status === 403) {
    throw new DashboardAuthenticationError();
  }
  if (!response.ok) {
    throw new Error(`Machine dashboard request failed (${response.status})`);
  }
  return normalizeMachinesResponse(await response.json());
}

/**
 * Establishes an HttpOnly dashboard session. No bearer token is persisted or
 * exposed to the browser bundle.
 *
 * @param {string} password
 * @param {typeof fetch} fetchImpl
 */
export async function loginDashboard(password, fetchImpl = fetch) {
  const response = await fetchImpl('/api/v1/dashboard/login', {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password: asNonEmptyString(password, 'password') }),
  });
  if (!response.ok) {
    throw new Error(`Dashboard login failed (${response.status})`);
  }
}
