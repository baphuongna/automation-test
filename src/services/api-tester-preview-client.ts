import type {
  ApiAssertionDto,
  ApiAssertionResultDto,
  ApiExecutionResultDto,
  ApiRequestDto,
  ApiRequestPreviewDto,
  ApiTestCaseDto
} from "../types";

const PREVIEW_STORAGE_KEY = "testforge.apiTester.preview.v1";
const PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T9 verification path.";

interface PreviewState {
  testCases: ApiTestCaseDto[];
}

function nowId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function isSensitiveKey(key: string): boolean {
  return /(authorization|token|secret|api[-_]?key|password|cookie)/i.test(key);
}

function maskSensitiveValue(value: string): string {
  return value.trim().length === 0 ? "[REDACTED]" : "[REDACTED]";
}

function redactMap(values: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(values).map(([key, value]) => [key, isSensitiveKey(key) ? maskSensitiveValue(value) : value])
  );
}

function createAuthPreview(request: ApiRequestDto): string {
  const authType = request.auth?.type ?? "none";

  if (authType === "bearer") {
    return "Bearer [REDACTED]";
  }

  if (authType === "basic") {
    return "Basic [REDACTED]";
  }

  if (authType === "api_key") {
    return `${request.auth?.location ?? "header"}:${request.auth?.key ?? "api_key"}=[REDACTED]`;
  }

  return "No auth";
}

function createRequestPreview(request: ApiRequestDto): ApiRequestPreviewDto {
  return {
    method: request.method,
    url: request.url,
    headers: redactMap(request.headers),
    queryParams: redactMap(request.queryParams),
    ...(request.body
      ? {
          bodyPreview: request.body.slice(0, 300)
        }
      : {}),
    authPreview: createAuthPreview(request)
  };
}

function getValueAtSourcePath(payload: unknown, sourcePath: string | undefined): string | undefined {
  if (!sourcePath) {
    return undefined;
  }

  const normalizedPath = sourcePath.replace(/^\$\.?/, "");
  if (normalizedPath.length === 0) {
    return undefined;
  }

  const segments = normalizedPath.split(".").filter((segment) => segment.length > 0);
  let current: unknown = payload;

  for (const segment of segments) {
    if (typeof current !== "object" || current === null || !(segment in current)) {
      return undefined;
    }

    current = (current as Record<string, unknown>)[segment];
  }

  return current === undefined ? undefined : String(current);
}

function createAssertionResult(
  assertion: ApiAssertionDto,
  statusCode: number,
  responseHeaders: Record<string, string>,
  responseBody: string,
  bodyPayload: Record<string, unknown>
): ApiAssertionResultDto {
  let actualValue = "";
  let passed = false;

  switch (assertion.operator) {
    case "status_equals": {
      actualValue = String(statusCode);
      passed = actualValue === assertion.expectedValue;
      break;
    }
    case "body_contains": {
      actualValue = responseBody;
      passed = responseBody.includes(assertion.expectedValue);
      break;
    }
    case "header_equals": {
      const headerKey = assertion.sourcePath?.trim().toLowerCase() ?? "content-type";
      actualValue = responseHeaders[headerKey] ?? "";
      passed = actualValue === assertion.expectedValue;
      break;
    }
    case "json_path_exists": {
      actualValue = getValueAtSourcePath(bodyPayload, assertion.sourcePath) ?? "";
      passed = actualValue.length > 0;
      break;
    }
    case "json_path_equals": {
      actualValue = getValueAtSourcePath(bodyPayload, assertion.sourcePath) ?? "";
      passed = actualValue === assertion.expectedValue;
      break;
    }
    default: {
      actualValue = "";
      passed = false;
    }
  }

  return {
    assertionId: assertion.id,
    operator: assertion.operator,
    passed,
    expectedValue: assertion.expectedValue,
    ...(actualValue.length > 0 ? { actualValue } : {}),
    ...(assertion.sourcePath ? { sourcePath: assertion.sourcePath } : {}),
    ...(passed
      ? {}
      : {
          message: `Assertion failed for ${assertion.operator}.`
        })
  };
}

function createSeedState(): PreviewState {
  return {
    testCases: [
      {
        id: "api-preview-users",
        type: "api",
        name: "Preview · List users",
        request: {
          method: "GET",
          url: "https://preview.testforge.local/users",
          headers: {
            Accept: "application/json",
            Authorization: "preview-secret-token"
          },
          queryParams: {
            page: "1",
            api_key: "preview-query-secret"
          },
          auth: {
            type: "bearer",
            token: "preview-secret-token"
          }
        },
        assertions: [
          {
            id: "assert-preview-status",
            operator: "status_equals",
            expectedValue: "200"
          },
          {
            id: "assert-preview-json",
            operator: "json_path_equals",
            sourcePath: "$.data.0.role",
            expectedValue: "admin"
          }
        ]
      }
    ]
  };
}

function readStoredState(): PreviewState {
  if (!canUseBrowserStorage()) {
    return createSeedState();
  }

  const raw = window.localStorage.getItem(PREVIEW_STORAGE_KEY);
  if (!raw) {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }

  try {
    return JSON.parse(raw) as PreviewState;
  } catch {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }
}

function writeStoredState(state: PreviewState): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(state));
}

function buildPreviewExecutionResult(request: ApiRequestDto, assertions: ApiAssertionDto[]): ApiExecutionResultDto {
  const normalizedUrl = request.url.toLowerCase();

  if (normalizedUrl.includes("{{missing")) {
    return {
      status: "failed",
      transportSuccess: false,
      failureKind: "preflight",
      errorCode: "API_REQUEST_BUILD_FAILED",
      errorMessage: "Preview preflight failed because a required variable is missing.",
      durationMs: 0,
      bodyPreview: "",
      responseHeaders: {},
      assertions: [],
      requestPreview: createRequestPreview(request)
    };
  }

  if (normalizedUrl.includes("offline") || normalizedUrl.includes("transport")) {
    return {
      status: "failed",
      transportSuccess: false,
      failureKind: "transport",
      errorCode: "API_TRANSPORT_FAILED",
      errorMessage: "Preview transport failure simulated for offline endpoint.",
      durationMs: 142,
      bodyPreview: "",
      responseHeaders: {},
      assertions: [],
      requestPreview: createRequestPreview(request)
    };
  }

  const statusCode = normalizedUrl.includes("created") ? 201 : 200;
  const responseHeaders: Record<string, string> = {
    "content-type": "application/json",
    "x-preview-mode": "true"
  };
  const bodyPayload = {
    ok: true,
    data: [{ id: 1, role: "admin", email: "qa@testforge.local" }],
    message: "Preview response body"
  };
  const bodyPreview = JSON.stringify(bodyPayload, null, 2);
  const assertionResults = assertions.map((assertion) =>
    createAssertionResult(assertion, statusCode, responseHeaders, bodyPreview, bodyPayload)
  );
  const hasAssertionFailure = assertionResults.some((assertion) => !assertion.passed);

  return {
    status: hasAssertionFailure ? "failed" : "passed",
    transportSuccess: true,
    ...(hasAssertionFailure
      ? {
          failureKind: "assertion" as const,
          errorCode: "API_ASSERTION_FAILED",
          errorMessage: "One or more preview assertions failed."
        }
      : {}),
    statusCode,
    durationMs: 186,
    bodyPreview,
    responseHeaders,
    assertions: assertionResults,
    requestPreview: createRequestPreview(request)
  };
}

export const apiTesterPreviewClient = {
  banner: PREVIEW_FALLBACK_BANNER,

  isAvailable(): boolean {
    return typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
  },

  loadWorkspace(): Promise<ApiTestCaseDto[]> {
    return Promise.resolve(readStoredState().testCases);
  },

  upsert(testCase: ApiTestCaseDto): Promise<ApiTestCaseDto> {
    const state = readStoredState();
    const nextTestCase = {
      ...testCase,
      id: testCase.id.trim().length > 0 ? testCase.id : nowId("api-preview")
    };
    const existingIndex = state.testCases.findIndex((candidate) => candidate.id === nextTestCase.id);
    const testCases = [...state.testCases];

    if (existingIndex >= 0) {
      testCases.splice(existingIndex, 1, nextTestCase);
    } else {
      testCases.push(nextTestCase);
    }

    writeStoredState({ testCases });
    return Promise.resolve(nextTestCase);
  },

  delete(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    writeStoredState({
      testCases: state.testCases.filter((testCase) => testCase.id !== id)
    });
    return Promise.resolve({ deleted: true });
  },

  execute(input: { testCaseId?: string; environmentId: string; request: ApiRequestDto; assertions: ApiAssertionDto[] }): Promise<ApiExecutionResultDto> {
    return Promise.resolve(buildPreviewExecutionResult(input.request, input.assertions));
  }
};
