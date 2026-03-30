import { Navigate, Route, Routes } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { TabBar } from './components/TabBar';
import ApiTester from './routes/api-tester';
import DataManager from './routes/data-manager';
import EnvironmentManager from './routes/environment-manager';
import Settings from './routes/settings';
import TestRunner from './routes/test-runner';
import WebRecorder from './routes/web-recorder';

function App() {
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

        <StatusBar />
      </div>
    </div>
  );
}

export default App;
