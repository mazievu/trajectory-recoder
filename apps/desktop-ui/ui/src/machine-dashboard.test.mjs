import assert from 'node:assert/strict';
import test from 'node:test';

import {
  fetchMachines,
  formatOnlineDuration,
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

test('uses only the dashboard credential when loading machine presence', async () => {
  let requestedUrl;
  let requestedHeaders;
  const machines = await fetchMachines(
    { apiUrl: 'https://trajectory.example', apiToken: 'dashboard-token' },
    async (url, init) => {
      requestedUrl = url;
      requestedHeaders = init.headers;
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
    },
  );

  assert.equal(requestedUrl, 'https://trajectory.example/api/v1/machines');
  assert.deepEqual(requestedHeaders, { 'X-Server-Token': 'dashboard-token' });
  assert.equal(machines[0].machineId, 'WS-01');
});

test('rejects a failed protected API response instead of showing fixture data', async () => {
  await assert.rejects(
    () => fetchMachines({ apiUrl: 'https://trajectory.example', apiToken: 'dashboard-token' }, async () => new Response('', { status: 401 })),
    /401/,
  );
});
