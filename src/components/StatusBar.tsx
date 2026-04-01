import type { ShellMetadataDto } from '../types';

interface StatusBarProps {
  shellMetadata: ShellMetadataDto | null;
  runtimeStatusMessage: string;
}

const BROWSER_AUTOMATION_UNAVAILABLE_MESSAGE =
  'Browser automation unavailable. Browser flows are blocked while API/data features remain usable.';

export function StatusBar({ shellMetadata, runtimeStatusMessage }: StatusBarProps) {
  const versionLabel = shellMetadata ? `v${shellMetadata.appVersion}` : 'Version unavailable';
  const bootstrapLabel = shellMetadata
    ? shellMetadata.isFirstRun
      ? 'First run bootstrap completed'
      : 'Bootstrap ready'
    : 'Shell metadata unavailable';
  const runtimeLabel = runtimeStatusMessage.trim().length > 0
    ? runtimeStatusMessage
    : BROWSER_AUTOMATION_UNAVAILABLE_MESSAGE;

  return (
    <footer className="status-bar" aria-label="Status bar placeholder">
      <span>{bootstrapLabel}</span>
      <span>{runtimeLabel}</span>
      <span>{versionLabel}</span>
    </footer>
  );
}
