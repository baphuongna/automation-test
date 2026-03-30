import { NavLink } from 'react-router-dom';

interface NavItem {
  to: string;
  icon: string;
  label: string;
}

const navItems: NavItem[] = [
  { to: '/api-tester', icon: '🔌', label: 'API Tester' },
  { to: '/web-recorder', icon: '🎬', label: 'Web Recorder' },
  { to: '/test-runner', icon: '▶️', label: 'Test Runner' },
  { to: '/environment-manager', icon: '🌐', label: 'Environment' },
  { to: '/data-manager', icon: '📦', label: 'Data Manager' },
  { to: '/settings', icon: '⚙️', label: 'Settings' },
];

export function Sidebar() {
  return (
    <nav className="sidebar-nav">
      {navItems.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          className={({ isActive }) =>
            `sidebar-link${isActive ? ' sidebar-link--active' : ''}`
          }
        >
          <span aria-hidden="true">{item.icon}</span>
          <span>{item.label}</span>
        </NavLink>
      ))}
    </nav>
  );
}
