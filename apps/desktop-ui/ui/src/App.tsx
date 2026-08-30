import { useCallback, useEffect, useState, type CSSProperties, type FormEvent } from 'react';

import {
  DashboardAuthenticationError,
  fetchMachines,
  formatOnlineDuration,
  loginDashboard,
} from './machine-dashboard.mjs';

type MachinePresence = {
  machineId: string;
  hostname: string;
  osVersion: string;
  lastSeenAt: string | null;
  onlineSeconds: number;
  status: 'online' | 'offline';
};

const REFRESH_INTERVAL_MS = 30_000;

function displayLastSeen(timestamp: string | null): string {
  if (!timestamp) return 'Never reported';
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? 'Unknown' : date.toLocaleString();
}

export default function App() {
  const [machines, setMachines] = useState<MachinePresence[]>([]);
  const [loading, setLoading] = useState(true);
  const [authenticated, setAuthenticated] = useState(false);
  const [password, setPassword] = useState('');
  const [signingIn, setSigningIn] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const nextMachines = await fetchMachines();
      setMachines(nextMachines);
      setAuthenticated(true);
      setError(null);
    } catch (cause) {
      if (cause instanceof DashboardAuthenticationError) {
        setAuthenticated(false);
        setError(null);
        return;
      }
      setError(cause instanceof Error ? cause.message : 'Unable to load machine status.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    if (!authenticated) return undefined;
    const timer = window.setInterval(() => void refresh(), REFRESH_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [authenticated, refresh]);

  const signIn = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSigningIn(true);
    setError(null);
    try {
      await loginDashboard(password);
      setPassword('');
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Unable to sign in.');
    } finally {
      setSigningIn(false);
    }
  };

  const onlineCount = machines.filter((machine) => machine.status === 'online').length;

  if (!authenticated) {
    return (
      <main style={styles.loginPage}>
        <form onSubmit={signIn} style={styles.loginCard}>
          <h1 style={styles.title}>Trajectory Server</h1>
          <p style={styles.subtitle}>Sign in to view connected recorder machines.</p>
          <label htmlFor="dashboard-password" style={styles.loginLabel}>Dashboard password</label>
          <input
            id="dashboard-password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete="current-password"
            style={styles.password}
            required
          />
          {error && <p role="alert" style={styles.error}>{error}</p>}
          <button type="submit" style={styles.refresh} disabled={signingIn || loading}>
            {signingIn ? 'Signing in…' : 'Sign in'}
          </button>
        </form>
      </main>
    );
  }

  return (
    <main style={styles.page}>
      <header style={styles.header}>
        <div>
          <h1 style={styles.title}>Trajectory Server</h1>
          <p style={styles.subtitle}>Connected recorder machines</p>
        </div>
        <button type="button" onClick={() => void refresh()} style={styles.refresh} disabled={loading}>
          {loading ? 'Loading…' : 'Refresh'}
        </button>
      </header>

      <section aria-label="Machine summary" style={styles.summary}>
        <strong>{machines.length}</strong> registered&nbsp;·&nbsp;<strong>{onlineCount}</strong> online
      </section>

      {error && <p role="alert" style={styles.error}>{error}</p>}

      {!loading && !error && machines.length === 0 && (
        <p style={styles.empty}>No recorder machines have registered yet.</p>
      )}

      {machines.length > 0 && (
        <div style={styles.tableWrap}>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.cell}>Machine</th>
                <th style={styles.cell}>Operating system</th>
                <th style={styles.cell}>Status</th>
                <th style={styles.cell}>Last seen</th>
                <th style={styles.cell}>Online time</th>
              </tr>
            </thead>
            <tbody>
              {machines.map((machine) => (
                <tr key={machine.machineId}>
                  <td style={styles.cell}>
                    <strong>{machine.hostname}</strong><br />
                    <span style={styles.muted}>{machine.machineId}</span>
                  </td>
                  <td style={styles.cell}>{machine.osVersion}</td>
                  <td style={styles.cell}>
                    <span style={machine.status === 'online' ? styles.online : styles.offline}>
                      {machine.status === 'online' ? 'Online' : 'Offline'}
                    </span>
                  </td>
                  <td style={styles.cell}>{displayLastSeen(machine.lastSeenAt)}</td>
                  <td style={styles.cell}>{formatOnlineDuration(machine.onlineSeconds)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </main>
  );
}

const styles: Record<string, CSSProperties> = {
  page: { minHeight: '100vh', boxSizing: 'border-box', padding: '32px', background: '#0f172a', color: '#e2e8f0' },
  header: { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '16px', marginBottom: '16px' },
  title: { margin: 0, fontSize: '24px' },
  subtitle: { margin: '4px 0 0', color: '#94a3b8' },
  refresh: { border: 0, borderRadius: '6px', padding: '8px 14px', cursor: 'pointer', background: '#38bdf8', color: '#082f49', fontWeight: 700 },
  summary: { marginBottom: '16px', color: '#cbd5e1' },
  tableWrap: { overflowX: 'auto', border: '1px solid #334155', borderRadius: '8px' },
  table: { borderCollapse: 'collapse', width: '100%', minWidth: '720px', background: '#1e293b' },
  cell: { textAlign: 'left', padding: '14px 16px', borderBottom: '1px solid #334155' },
  muted: { color: '#94a3b8', fontSize: '13px' },
  online: { color: '#4ade80', fontWeight: 700 },
  offline: { color: '#f87171', fontWeight: 700 },
  error: { border: '1px solid #ef4444', borderRadius: '6px', padding: '12px', color: '#fecaca' },
  empty: { color: '#94a3b8' },
  loginPage: { minHeight: '100vh', display: 'grid', placeItems: 'center', padding: '24px', background: '#0f172a', color: '#e2e8f0' },
  loginCard: { width: 'min(100%, 380px)', display: 'grid', gap: '12px', padding: '28px', border: '1px solid #334155', borderRadius: '10px', background: '#1e293b' },
  loginLabel: { fontWeight: 700 },
  password: { padding: '10px', borderRadius: '6px', border: '1px solid #475569', background: '#0f172a', color: '#f8fafc' },
};
