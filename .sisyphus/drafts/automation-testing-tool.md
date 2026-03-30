# Draft: Automation Testing Tool Design

## Confirmed Requirements

### Core Features
- **API Testing**: Send requests, check responses, status codes, data structure
- **Web UI Testing**: Automated browser actions (click, fill forms, verify UI)
- **Record & Replay**: Capture user actions on web, replay for regression tests
- **Target Audience**: QA/Tester team (non-coders, low-code friendly)

### Team Context
- **Rust Experience**: Basic (has learned, can write simple programs)
- **Automation Testing Experience**: None — learning from A-Z
- **Budget/Timeline**: Small team, 3-6 months
- **Dev Environment**: Starting from scratch (D:\my\research is empty)

### Chosen Architecture
- **Approach**: Phương án 3 - Tauri + Playwright (Lightweight Desktop)
- **Frontend**: React/TypeScript (WebView)
- **Backend**: Rust
- **Testing Core**: Playwright integration
- **Storage**: SQLite

## Research Findings

### Architecture Comparison (from librarian research)
- **Electron**: 150-200MB bundle, 300-500MB memory, mature ecosystem
- **Tauri**: 10-30MB bundle, 100-200MB memory, emerging tech
- **Browser Extensions**: Limited by Manifest V3, no CORS bypass, no full browser control
- **Web Apps**: No installation, but limited access to system APIs

### Key Tradeoffs for Tauri
- **Pros**: Lightweight (10x smaller), better performance, native speed
- **Cons**: Higher dev complexity (Rust learning curve), smaller ecosystem

## Risk Assessment (after review)

### playwright-rs Status
- **Version**: v0.9.0 (pre-1.0) — Mar 27, 2026
- **Architecture**: JSON-RPC to Playwright Server (same as official Python/Java/.NET)
- **Coverage**: Locator/Response/Request 100%, Page ~81%, BrowserContext ~66%, Frame ~38%
- **Stars**: 64, Downloads: 3,435 (growing fast)
- **Risk**: Pre-1.0 API may change, small community

### Bundle Size Reality
- Tauri base: 10-30MB
- Playwright browsers: 200-400MB (dominates)
- Total: ~300-450MB (NOT 10-30MB as initially claimed)
- "Lightweight" advantage over Electron is ~15-20%, not 10x

### Alternatives Considered
- **chromiumoxide**: Production-grade (1,221 stars), direct CDP, Chromium only, no Node.js
- **fantoccini**: WebDriver (1,997 stars), cross-browser, mature but no Playwright features
- **Decision**: Use playwright-rs for Playwright API compatibility

### Mitigation Strategies
- Pin playwright-rs version, test thoroughly before upgrading
- For missing APIs (Frame ~38%), implement workarounds or contribute upstream
- Monitor playwright-rs releases for stability
- Fallback: chromiumoxide if playwright-rs becomes unviable

## Open Questions
- (Remaining questions from brainstorming)

## Design Decisions

### Phased Approach
- **Phase 1 (3 months)**: Core + Essential
  - API Testing Engine
  - Web UI Recorder & Player (Playwright)
  - Test Runner (basic local execution)
  - Test Data Management (CRUD)
  - Basic Reporting (pass/fail, logs)
  - Environment switching (dev/staging/prod)

- **Phase 2 (3 months after)**: Advanced Features
  - Advanced Reporting (charts, trends, history)
  - Scheduling test runs
  - CI/CD integration (GitHub Actions, Jenkins)
  - Team collaboration (users, roles, permissions)
  - Integrations (Jira, Slack, email alerts)
