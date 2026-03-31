import { accessSync, constants, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

type SmokeStatus = "SMOKE_PASS" | "SMOKE_BLOCKED" | "SMOKE_FAIL";

interface SmokeReport {
  status: SmokeStatus;
  details: string;
  diagnostics: Record<string, unknown>;
}

const BROWSER_REPLAY_T14_SMOKE = "BROWSER_REPLAY_T14_SMOKE";

const NODE_CDP_INTERACTION_SCRIPT = `
const cp = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

async function sleep(ms){ return new Promise((r)=>setTimeout(r, ms)); }

async function waitWs(port, timeoutMs){
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const response = await fetch('http://127.0.0.1:' + port + '/json/list');
      if (response.ok) {
        const pages = await response.json();
        const page = Array.isArray(pages) ? pages.find((item)=>item.webSocketDebuggerUrl) : null;
        if (page && page.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
      }
    } catch (_) {}
    await sleep(60);
  }
  throw new Error('Cannot discover CDP websocket endpoint');
}

async function run(){
  const payload = JSON.parse(process.argv[1]);
  const port = 46000 + Math.floor(Math.random() * 1000);
  const userDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tf-smoke-cdp-'));
  const chrome = cp.spawn(payload.chromiumPath, [
    '--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    '--window-size=1280,720',
    '--remote-debugging-port=' + port,
    '--virtual-time-budget=3000',
    '--user-data-dir=' + userDir,
    payload.url,
  ], { stdio: ['ignore', 'pipe', 'pipe'] });

  let ws;
  try {
    const wsUrl = await waitWs(port, 7000);
    ws = new WebSocket(wsUrl);
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', () => resolve());
      ws.addEventListener('error', () => reject(new Error('WebSocket open failed')));
    });

    let id = 0;
    const pending = new Map();
    ws.addEventListener('message', (event) => {
      const data = JSON.parse(event.data.toString());
      if (data.id && pending.has(data.id)) {
        const entry = pending.get(data.id);
        pending.delete(data.id);
        if (data.error) entry.reject(new Error(data.error.message || 'CDP error'));
        else entry.resolve(data.result || {});
      }
    });

    const send = (method, params = {}) => {
      id += 1;
      const currentId = id;
      ws.send(JSON.stringify({ id: currentId, method, params }));
      return new Promise((resolve, reject) => pending.set(currentId, { resolve, reject }));
    };

    await send('Page.enable');
    await send('Runtime.enable');
    await send('DOM.enable');

    const clickExpr = "(() => { const el = document.querySelector('#btn'); if (!el) throw new Error('btn missing'); el.click(); return true; })()";
    await send('Runtime.evaluate', { expression: clickExpr, awaitPromise: true, returnByValue: true });

    const fillExpr = "(() => { const el = document.querySelector('#name'); if (!el) throw new Error('name missing'); el.value = 'alice'; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); return el.value; })()";
    await send('Runtime.evaluate', { expression: fillExpr, awaitPromise: true, returnByValue: true });

    const selectExpr = "(() => { const el = document.querySelector('#role'); if (!el) throw new Error('role missing'); el.value = 'admin'; el.dispatchEvent(new Event('change', { bubbles: true })); return el.value; })()";
    await send('Runtime.evaluate', { expression: selectExpr, awaitPromise: true, returnByValue: true });

    const checkExpr = "(() => { const el = document.querySelector('#agree'); if (!el) throw new Error('agree missing'); el.checked = true; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); return el.checked; })()";
    await send('Runtime.evaluate', { expression: checkExpr, awaitPromise: true, returnByValue: true });

    const domResult = await send('Runtime.evaluate', {
      expression: 'document.documentElement.outerHTML',
      awaitPromise: true,
      returnByValue: true,
    });
    const dom = domResult && domResult.result ? String(domResult.result.value || '') : '';

    const passed = dom.includes('clicked') && dom.includes('alice') && dom.includes('admin') && dom.includes('checked');
    if (!passed) {
      throw new Error('DOM did not reflect expected interaction side-effects');
    }

    process.stdout.write(JSON.stringify({ ok: true, dom }));
  } catch (error) {
    process.stdout.write(JSON.stringify({ ok: false, error: String(error && error.message ? error.message : error) }));
    process.exitCode = 1;
  } finally {
    try { if (ws && ws.readyState === WebSocket.OPEN) ws.close(); } catch (_) {}
    try { chrome.kill('SIGKILL'); } catch (_) {}
    try { fs.rmSync(userDir, { recursive: true, force: true }); } catch (_) {}
  }
}

run();
`;

function resolveChromiumExecutableCandidates(): string[] {
  const candidates: string[] = [];
  const envDirect = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
  if (envDirect && envDirect.trim().length > 0) {
    candidates.push(envDirect.trim());
  }

  const envBrowsers = process.env.PLAYWRIGHT_BROWSERS_PATH;
  if (envBrowsers && envBrowsers.trim().length > 0) {
    candidates.push(join(envBrowsers.trim(), "chromium", "chrome-win", "chrome.exe"));
  }

  candidates.push(
    resolve("ms-playwright", "chromium", "chrome-win", "chrome.exe"),
    resolve("src-tauri", "ms-playwright", "chromium", "chrome-win", "chrome.exe")
  );

  return candidates;
}

function firstExistingExecutable(candidates: string[]): string | null {
  for (const candidate of candidates) {
    try {
      accessSync(candidate, constants.F_OK);
      return candidate;
    } catch {
      // keep scanning
    }
  }
  return null;
}

function ensureNodeRuntime(): string | null {
  const check = spawnSync("node", ["--version"], { encoding: "utf8" });
  if (check.status !== 0) {
    return null;
  }
  return check.stdout.trim() || "node";
}

function createDeterministicTarget(): { url: string; cleanup: () => void } {
  const dir = mkdtempSync(join(tmpdir(), "tf-t14-smoke-"));
  const htmlPath = join(dir, "smoke.html");
  const html = `<!doctype html>
<html><body>
  <button id="btn" onclick="document.getElementById('status').textContent='clicked'">Click</button>
  <input id="name" value="" oninput="document.getElementById('typed').textContent=this.value" />
  <select id="role" onchange="document.getElementById('selected').textContent=this.value">
    <option value="">none</option><option value="admin">admin</option>
  </select>
  <input id="agree" type="checkbox" onchange="document.getElementById('checked').textContent=this.checked?'checked':'unchecked'" />
  <div id="status">idle</div><div id="typed"></div><div id="selected"></div><div id="checked">unchecked</div>
</body></html>`;
  writeFileSync(htmlPath, html, "utf8");
  return {
    url: `file://${htmlPath.replace(/\\/g, "/")}`,
    cleanup: () => rmSync(dir, { recursive: true, force: true }),
  };
}

function runSmoke(): SmokeReport {
  const nodeVersion = ensureNodeRuntime();
  if (!nodeVersion) {
    return {
      status: "SMOKE_BLOCKED",
      details: "Node runtime is missing; cannot execute replay interaction smoke.",
      diagnostics: { missing: "node" },
    };
  }

  const candidates = resolveChromiumExecutableCandidates();
  const chromiumPath = firstExistingExecutable(candidates);
  if (!chromiumPath) {
    return {
      status: "SMOKE_BLOCKED",
      details:
        "Chromium executable is missing; set PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH or install browser runtime.",
      diagnostics: { missing: "chromium executable", candidates },
    };
  }

  const target = createDeterministicTarget();
  try {
    const payload = JSON.stringify({ chromiumPath, url: target.url });
    const result = spawnSync("node", ["-e", NODE_CDP_INTERACTION_SCRIPT, payload], {
      encoding: "utf8",
      timeout: 30000,
    });

    if (result.status !== 0) {
      return {
        status: "SMOKE_FAIL",
        details: "Replay interaction runtime execution failed.",
        diagnostics: {
          exitCode: result.status,
          stderr: result.stderr.trim(),
          stdout: result.stdout.trim(),
        },
      };
    }

    const parsed = JSON.parse(result.stdout || "{}");
    if (!parsed.ok) {
      return {
        status: "SMOKE_FAIL",
        details: "Replay interaction runtime returned non-ok result.",
        diagnostics: parsed,
      };
    }

    const dom = String(parsed.dom || "");
    const validated = dom.includes("clicked") && dom.includes("alice") && dom.includes("admin") && dom.includes("checked");
    if (!validated) {
      return {
        status: "SMOKE_FAIL",
        details: "Replay interaction did not produce expected browser-side DOM transitions.",
        diagnostics: { domPreview: dom.slice(0, 500) },
      };
    }

    return {
      status: "SMOKE_PASS",
      details: "Replay interaction smoke succeeded with browser-side state transitions.",
      diagnostics: { chromiumPath, nodeVersion, url: target.url },
    };
  } catch (error) {
    return {
      status: "SMOKE_FAIL",
      details: "Unexpected error while executing replay smoke harness.",
      diagnostics: { error: String(error) },
    };
  } finally {
    target.cleanup();
  }
}

const report = runSmoke();
console.log(`[${BROWSER_REPLAY_T14_SMOKE}] ${report.status} :: ${report.details}`);
console.log(JSON.stringify(report.diagnostics, null, 2));

if (report.status !== "SMOKE_PASS") {
  process.exit(1);
}
