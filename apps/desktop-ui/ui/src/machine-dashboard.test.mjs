import assert from 'node:assert/strict';
import test from 'node:test';

import {
  DashboardAuthenticationError,
  fetchMachines,
  formatOnlineDuration,
  loginDashboard,
  normalizeMachinesResponse,
} from './machine-dashboard.mjs';

test('normalizes the protected machines endpoint response for the dashboard', () => {
  const machines = normalizeMachinesResponse({
    machines: [
      {
        machine_id: 'WS-01',
        hostname: 'finance-laptop',
        os_version: 'Windows 11',
        last_seen_at: '2026-08-30T10:00:00Z',
        online_duration_secs: 7265,
        status: 'ACTIVE',
        is_online: true,
      },
    ],
  });

  assert.deepEqual(machines, [
    {
      machineId: 'WS-01',
      hostname: 'finance-laptop',
      osVersion: 'Windows 11',
      lastSeenAt: '2026-08-30T10:00:00Z',
      onlineSeconds: 7265,
      status: 'online',
    },
  ]);
});

test('formats online duration without leaking implementation timestamps into the UI', () => {
  assert.equal(formatOnlineDuration(0), '0m');
  assert.equal(formatOnlineDuration(65), '1m');
  assert.equal(formatOnlineDuration(7265), '2h 1m');
});

test('uses the cookie-authenticated same-origin endpoint when loading machine presence', async () => {
  let requestedUrl;
  let requestedOptions;
  const machines = await fetchMachines(async (url, init) => {
      requestedUrl = url;
      requestedOptions = init;
      return new Response(
        JSON.stringify([
          {
            machine_id: 'WS-01',
            hostname: 'finance-laptop',
            os_version: 'Windows 11',
            last_seen_at: '2026-08-30T10:00:00Z',
            online_duration_secs: 1,
            status: 'ACTIVE',
            is_online: true,
          },
        ]),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });

  assert.equal(requestedUrl, '/api/v1/machines');
  assert.deepEqual(requestedOptions, { credentials: 'include' });
  assert.equal(machines[0].machineId, 'WS-01');
});

test('signals an unauthenticated dashboard session instead of showing fixture data', async () => {
  await assert.rejects(
    () => fetchMachines(async () => new Response('', { status: 401 })),
    DashboardAuthenticationError,
  );
});

test('submits the operator password only to the same-origin login endpoint', async () => {
  let requestedUrl;
  let requestedOptions;
  await loginDashboard('correct horse battery staple', async (url, init) => {
    requestedUrl = url;
    requestedOptions = init;
    return new Response(null, { status: 204 });
  });

  assert.equal(requestedUrl, '/api/v1/dashboard/login');
  assert.deepEqual(requestedOptions, {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password: 'correct horse battery staple' }),
  });
});
