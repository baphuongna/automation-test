import { useEffect, useState } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { TabBar } from './components/TabBar';
import { useTauriEvent } from './hooks/useTauriEvent';
import { getShellMetadata } from './services/tauri-client';
import ApiTester from './routes/api-tester';
import DataManager from './routes/data-manager';
import EnvironmentManager from './routes/environment-manager';
import Settings from './routes/settings';
import TestRunner from './routes/test-runner';
import WebRecorder from './routes/web-recorder';
import type { BrowserHealthDto, ShellMetadataDto } from './types';

function App() {
  const [shellMetadata, setShellMetadata] = useState<ShellMetadataDto | null>(null);

  useEffect(() => {
    let isActive = true;

    void getShellMetadata()
      .then((metadata) => {
        if (isActive) {
          setShellMetadata(metadata);
        }
      })
      .catch(() => {
        if (isActive) {
          setShellMetadata(null);
        }
      });

    return () => {
      isActive = false;
    };
  }, []);

  useTauriEvent('browser.health.changed', (payload: BrowserHealthDto) => {
    setShellMetadata((current) => {
      if (!current) {
        return current;
      }

      return {
        ...current,
        browserRuntime: payload
      };
    });
  });

  return (
    <div className="app-shell" data-testid="app-shell">
      <aside className="app-sidebar" aria-label="Primary navigation">
        <div className="app-brand">
          <span className="app-brand__title">TestForge</span>
          <span className="app-brand__subtitle">MVP shell placeholder</span>
        </div>
        <Sidebar />
      </aside>

      <div className="app-main">
        <TabBar />

        <main className="app-content" aria-label="Main content">
          <Routes>
            <Route path="/" element={<Navigate to="/api-tester" replace />} />
            <Route path="/api-tester" element={<ApiTester />} />
            <Route path="/web-recorder" element={<WebRecorder />} />
            <Route path="/test-runner" element={<TestRunner />} />
            <Route path="/environment-manager" element={<EnvironmentManager />} />
            <Route path="/data-manager" element={<DataManager />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/api-tester" replace />} />
          </Routes>
        </main>

        <StatusBar shellMetadata={shellMetadata} />
      </div>
    </div>
  );
}

export default App;
