const tabs = ['Shell Overview', 'Placeholder Route', 'Build Verified'];

export function TabBar() {
  return (
    <header className="tab-bar" aria-label="Tab placeholder">
      {tabs.map((tab) => (
        <span key={tab} className="tab-bar__item">
          {tab}
        </span>
      ))}
    </header>
  );
}
