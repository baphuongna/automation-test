import type { ShellMetadataDto } from '../types';

interface StatusBarProps {
  shellMetadata: ShellMetadataDto | null;
}

const BROWSER_AUTOMATION_UNAVAILABLE_MESSAGE =
  'Browser automation unavailable. Browser flows are blocked while API/data features remain usable.';

const BROWSER_AUTOMATION_DEGRADED_MESSAGE =
  'Browser automation unavailable until Chromium runtime is restored. API/data features remain usable.';

function getBootstrapLabel(shellMetadata: ShellMetadataDto | null): string {
  if (!shellMetadata) {
    return 'Shell metadata unavailable';
  }

  if (!shellMetadata.masterKeyInitialized) {
    return 'Bootstrap incomplete';
  }

  if (shellMetadata.degradedMode) {
    return shellMetadata.isFirstRun ? 'First run in degraded mode' : 'Running in degraded mode';
  }

  return shellMetadata.isFirstRun ? 'First run bootstrap completed' : 'Bootstrap ready';
}

function getRuntimeLabel(shellMetadata: ShellMetadataDto | null): string {
  if (!shellMetadata) {
    return 'Runtime status unavailable';
  }

  const message = shellMetadata.browserRuntime.message.trim();

  if (message.length > 0) {
    return message;
  }

  if (shellMetadata.browserRuntime.runtimeStatus === 'healthy') {
    return 'Browser runtime healthy';
  }

  if (shellMetadata.browserRuntime.runtimeStatus === 'degraded') {
    return BROWSER_AUTOMATION_DEGRADED_MESSAGE;
  }

  return BROWSER_AUTOMATION_UNAVAILABLE_MESSAGE;
}

function getVersionLabel(shellMetadata: ShellMetadataDto | null): string {
  if (!shellMetadata) {
    return 'Version unavailable';
  }

  const version = shellMetadata.appVersion.trim();

  return version.length > 0 ? version : 'Version unavailable';
}

export function StatusBar({ shellMetadata }: StatusBarProps) {
  const versionLabel = getVersionLabel(shellMetadata);
  const bootstrapLabel = getBootstrapLabel(shellMetadata);
  const runtimeLabel = getRuntimeLabel(shellMetadata);

  return (
    <footer className="status-bar" aria-label="Status bar placeholder">
      <span>{bootstrapLabel}</span>
      <span>{runtimeLabel}</span>
      <span>{versionLabel}</span>
    </footer>
  );
}
