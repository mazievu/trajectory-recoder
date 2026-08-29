import React, { useState } from 'react';

export interface ActionItem {
  id: number;
  time: string;
  type: string;
  app: string;
  window: string;
  target: string;
  detail: string;
  confidence: number;
}

const mockActions: ActionItem[] = [
  { id: 1, time: '04:05:12', type: 'CLICK', app: 'chrome.exe', window: 'Google Sheets - Q3 Budget', target: 'Cell C12', detail: 'Primary Left Click at (450, 310)', confidence: 1.0 },
  { id: 2, time: '04:05:14', type: 'TYPE_TEXT', app: 'chrome.exe', window: 'Google Sheets - Q3 Budget', target: 'Formula Bar', detail: 'Typed "=SUM(C2:C11)" (13 chars)', confidence: 1.0 },
  { id: 3, time: '04:05:18', type: 'KEY_PRESS', app: 'chrome.exe', window: 'Google Sheets - Q3 Budget', target: 'Formula Bar', detail: 'Key: Enter (VK_RETURN)', confidence: 1.0 },
  { id: 4, time: '04:05:22', type: 'WINDOW_SWITCH', app: 'excel.exe', window: 'Financial_Model_v4.xlsx', target: 'MainWindow', detail: 'Foreground Window Changed', confidence: 0.98 },
  { id: 5, time: '04:05:30', type: 'DRAG_DROP', app: 'excel.exe', window: 'Financial_Model_v4.xlsx', target: 'Range Selection', detail: 'Drag from (120, 200) to (300, 450) [dist 280px]', confidence: 0.95 },
  { id: 6, time: '04:05:45', type: 'COPY', app: 'excel.exe', window: 'Financial_Model_v4.xlsx', target: 'Clipboard', detail: 'Format CF_UNICODETEXT, SHA256 e3b0c442...', confidence: 1.0 },
];

export default function App() {
  const [isRecording, setIsRecording] = useState(true);
  const [selectedAction, setSelectedAction] = useState<ActionItem>(mockActions[0]);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState<'timeline' | 'diff' | 'tree'>('timeline');

  const filtered = mockActions.filter(a =>
    a.app.toLowerCase().includes(searchQuery.toLowerCase()) ||
    a.type.toLowerCase().includes(searchQuery.toLowerCase()) ||
    a.target.toLowerCase().includes(searchQuery.toLowerCase()) ||
    a.detail.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', backgroundColor: '#0f172a', color: '#e2e8f0', fontFamily: 'sans-serif' }}>
      {/* Top Header & Status Tray */}
      <header style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 20px', borderBottom: '1px solid #334155', backgroundColor: '#1e293b' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: isRecording ? '#22c55e' : '#ef4444', boxShadow: isRecording ? '0 0 8px #22c55e' : 'none' }} />
          <h1 style={{ margin: 0, fontSize: '18px', fontWeight: 600 }}>Trajectory Recorder</h1>
          <span style={{ fontSize: '12px', color: '#94a3b8', padding: '2px 8px', backgroundColor: '#0f172a', borderRadius: '4px' }}>
            Session: WS01_20260829_040000_a1b2c3d4
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <span style={{ fontSize: '13px', color: '#cbd5e1' }}>Disk: <strong>42.5%</strong> (Normal)</span>
          <span style={{ fontSize: '13px', color: '#cbd5e1' }}>Actions: <strong>{mockActions.length}</strong></span>
          <button
            onClick={() => setIsRecording(!isRecording)}
            style={{
              padding: '6px 14px',
              backgroundColor: isRecording ? '#dc2626' : '#16a34a',
              color: '#fff',
              border: 'none',
              borderRadius: '6px',
              cursor: 'pointer',
              fontWeight: 600
            }}
          >
            {isRecording ? 'Pause Recording' : 'Resume Recording'}
          </button>
        </div>
      </header>

      {/* Main Workspace */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* Left Timeline Panel */}
        <aside style={{ width: '420px', borderRight: '1px solid #334155', display: 'flex', flexDirection: 'column', backgroundColor: '#1e293b' }}>
          <div style={{ padding: '12px', borderBottom: '1px solid #334155' }}>
            <input
              type="text"
              placeholder="Search actions, apps, elements..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                padding: '8px 12px',
                backgroundColor: '#0f172a',
                border: '1px solid #475569',
                borderRadius: '6px',
                color: '#f8fafc',
                boxSizing: 'border-box'
              }}
            />
          </div>

          <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
            {filtered.map((item) => (
              <div
                key={item.id}
                onClick={() => setSelectedAction(item)}
                style={{
                  padding: '10px 12px',
                  marginBottom: '6px',
                  borderRadius: '6px',
                  backgroundColor: selectedAction.id === item.id ? '#334155' : '#0f172a',
                  borderLeft: selectedAction.id === item.id ? '4px solid #38bdf8' : '4px solid transparent',
                  cursor: 'pointer',
                  transition: 'all 0.15s'
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
                  <span style={{ fontWeight: 600, fontSize: '13px', color: '#38bdf8' }}>{item.type}</span>
                  <span style={{ fontSize: '11px', color: '#94a3b8' }}>{item.time}</span>
                </div>
                <div style={{ fontSize: '12px', color: '#e2e8f0', marginBottom: '2px' }}>{item.detail}</div>
                <div style={{ fontSize: '11px', color: '#64748b' }}>{item.app} • {item.target}</div>
              </div>
            ))}
          </div>
        </aside>

        {/* Right Inspection & Diff Panel */}
        <main style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '16px', overflowY: 'auto' }}>
          {/* Sub Navigation */}
          <div style={{ display: 'flex', gap: '8px', marginBottom: '16px', borderBottom: '1px solid #334155', paddingBottom: '8px' }}>
            <button
              onClick={() => setActiveTab('timeline')}
              style={{
                padding: '6px 12px',
                backgroundColor: activeTab === 'timeline' ? '#38bdf8' : 'transparent',
                color: activeTab === 'timeline' ? '#0f172a' : '#cbd5e1',
                border: 'none',
                borderRadius: '4px',
                fontWeight: 600,
                cursor: 'pointer'
              }}
            >
              Action Overview
            </button>
            <button
              onClick={() => setActiveTab('diff')}
              style={{
                padding: '6px 12px',
                backgroundColor: activeTab === 'diff' ? '#38bdf8' : 'transparent',
                color: activeTab === 'diff' ? '#0f172a' : '#cbd5e1',
                border: 'none',
                borderRadius: '4px',
                fontWeight: 600,
                cursor: 'pointer'
              }}
            >
              Before / After Visual Diff
            </button>
            <button
              onClick={() => setActiveTab('tree')}
              style={{
                padding: '6px 12px',
                backgroundColor: activeTab === 'tree' ? '#38bdf8' : 'transparent',
                color: activeTab === 'tree' ? '#0f172a' : '#cbd5e1',
                border: 'none',
                borderRadius: '4px',
                fontWeight: 600,
                cursor: 'pointer'
              }}
            >
              UIA & DOM Hierarchy
            </button>
          </div>

          {/* Action Overview Tab */}
          {activeTab === 'timeline' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div style={{ padding: '16px', backgroundColor: '#1e293b', borderRadius: '8px', border: '1px solid #334155' }}>
                <h3 style={{ margin: '0 0 12px 0', fontSize: '16px', color: '#38bdf8' }}>Action #{selectedAction.id}: {selectedAction.type}</h3>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', fontSize: '13px' }}>
                  <div><strong>Application:</strong> {selectedAction.app}</div>
                  <div><strong>Window Title:</strong> {selectedAction.window}</div>
                  <div><strong>Target Element:</strong> {selectedAction.target}</div>
                  <div><strong>Confidence:</strong> {(selectedAction.confidence * 100).toFixed(0)}%</div>
                  <div><strong>Timestamp:</strong> {selectedAction.time} UTC</div>
                  <div><strong>Interaction Detail:</strong> {selectedAction.detail}</div>
                </div>
              </div>
            </div>
          )}

          {/* Visual Diff Tab */}
          {activeTab === 'diff' && (
            <div style={{ display: 'flex', gap: '16px', flex: 1 }}>
              <div style={{ flex: 1, padding: '12px', backgroundColor: '#1e293b', borderRadius: '8px', border: '1px solid #334155', display: 'flex', flexDirection: 'column' }}>
                <h4 style={{ margin: '0 0 8px 0', color: '#94a3b8' }}>State Before Interaction</h4>
                <div style={{ flex: 1, backgroundColor: '#0f172a', borderRadius: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center', border: '1px dashed #475569' }}>
                  <span style={{ color: '#64748b' }}>WebP Screenshot (Pre-Action)</span>
                </div>
              </div>
              <div style={{ flex: 1, padding: '12px', backgroundColor: '#1e293b', borderRadius: '8px', border: '1px solid #334155', display: 'flex', flexDirection: 'column' }}>
                <h4 style={{ margin: '0 0 8px 0', color: '#38bdf8' }}>State After Interaction (+200ms Diff Overlay)</h4>
                <div style={{ flex: 1, backgroundColor: '#0f172a', borderRadius: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center', border: '1px solid #38bdf8', position: 'relative' }}>
                  <span style={{ color: '#38bdf8' }}>Stabilized Diff (&lt;0.5% pixel delta)</span>
                </div>
              </div>
            </div>
          )}

          {/* UIA & DOM Tree Hierarchy Tab */}
          {activeTab === 'tree' && (
            <div style={{ padding: '16px', backgroundColor: '#1e293b', borderRadius: '8px', border: '1px solid #334155', fontFamily: 'monospace', fontSize: '13px' }}>
              <div style={{ color: '#94a3b8', marginBottom: '8px' }}>// 3-Level Ancestor Hierarchy</div>
              <div style={{ color: '#f8fafc', paddingLeft: '0px' }}>&lt;Window title="{selectedAction.window}" framework="WPF"&gt;</div>
              <div style={{ color: '#cbd5e1', paddingLeft: '20px' }}>&lt;Group name="HomeRibbon" automationId="RibbonGroup"&gt;</div>
              <div style={{ color: '#94a3b8', paddingLeft: '40px' }}>&lt;ToolBar name="MainToolBar"&gt;</div>
              <div style={{ color: '#38bdf8', paddingLeft: '60px', fontWeight: 600 }}>&lt;{selectedAction.type} name="{selectedAction.target}" id="btn_target" isPassword=false /&gt;</div>
              <div style={{ color: '#94a3b8', paddingLeft: '40px' }}>&lt;/ToolBar&gt;</div>
              <div style={{ color: '#cbd5e1', paddingLeft: '20px' }}>&lt;/Group&gt;</div>
              <div style={{ color: '#f8fafc', paddingLeft: '0px' }}>&lt;/Window&gt;</div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
