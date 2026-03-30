import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(message);
  }
}

const appSource = readFileSync(resolve('src/App.tsx'), 'utf8');
const sidebarSource = readFileSync(resolve('src/components/Sidebar.tsx'), 'utf8');
const tabBarSource = readFileSync(resolve('src/components/TabBar.tsx'), 'utf8');
const statusBarSource = readFileSync(resolve('src/components/StatusBar.tsx'), 'utf8');

assert(appSource.includes('<Sidebar />'), 'App shell must render the sidebar placeholder.');
assert(appSource.includes('<TabBar />'), 'App shell must render the tab bar placeholder.');
assert(appSource.includes('<StatusBar />'), 'App shell must render the status bar placeholder.');
assert(appSource.includes('Navigate to="/api-tester"'), 'App shell must redirect root to the default route skeleton.');

assert(sidebarSource.includes("'/api-tester'"), 'Sidebar must include the API Tester route.');
assert(sidebarSource.includes("'/web-recorder'"), 'Sidebar must include the Web Recorder route.');
assert(tabBarSource.includes('Shell Overview'), 'Tab bar must remain a placeholder-only shell element.');
assert(statusBarSource.includes('Ready'), 'Status bar must expose a ready placeholder state.');

console.log('Shell smoke test passed.');
