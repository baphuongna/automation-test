-- Migration 002: add query params persistence for API endpoints

ALTER TABLE api_endpoints
ADD COLUMN query_params_json TEXT NOT NULL DEFAULT '{}';
