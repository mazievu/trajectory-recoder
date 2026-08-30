/**
 * @typedef {Object} DashboardConfig
 * @property {string} apiUrl
 * @property {string} apiToken
 */

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
 * @param {DashboardConfig} config
 * @param {typeof fetch} fetchImpl
 * @returns {Promise<MachinePresence[]>}
 */
export async function fetchMachines(config, fetchImpl = fetch) {
  const apiUrl = asNonEmptyString(config?.apiUrl, 'dashboard API URL').replace(/\/+$/, '');
  const apiToken = asNonEmptyString(config?.apiToken, 'dashboard API token');
  const response = await fetchImpl(`${apiUrl}${MACHINES_PATH}`, {
    headers: { 'X-Server-Token': apiToken },
  });
  if (!response.ok) {
    throw new Error(`Machine dashboard request failed (${response.status})`);
  }
  return normalizeMachinesResponse(await response.json());
}

/**
 * Runtime configuration is injected by the server-only dashboard host. It is
 * deliberately not read from Vite variables: `VITE_*` values are embedded in
 * the browser bundle and would expose the dashboard credential.
 *
 * @returns {DashboardConfig | null}
 */
export function readDashboardConfig() {
  const config = globalThis.__TRAJECTORY_DASHBOARD_CONFIG__;
  if (!config || typeof config !== 'object') return null;
  if (typeof config.apiUrl !== 'string' || typeof config.apiToken !== 'string') return null;
  return { apiUrl: config.apiUrl, apiToken: config.apiToken };
}
