# Acceptance Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the first-version project management H5 closer to the requirements by fixing frontend/backend contract gaps, task collaboration actions, responsive form layout, rich text toolbar fidelity, backend failure logging, and admin-token smoke validation.

**Architecture:** Keep the existing Vue 3 + Element Plus pages and Rust/Actix backend shape. Add narrow API client methods and focused page logic rather than restructuring modules, then validate with unit tests, build checks, backend tests, and browser smoke tests.

**Tech Stack:** Vue 3, TypeScript, Vite, Vitest, Element Plus, TanStack Query, Tiptap, Rust, Actix Web, env_logger.

---

## File Map

- `frontend/src/types.ts`: align frontend project/settings/task attachment types with backend payloads.
- `frontend/src/api/client.ts`: add missing settings, task comments, task attachments, attachment download/delete, and project payload methods.
- `frontend/src/api/client.test.ts`: lock the API methods to backend route contracts.
- `frontend/src/features/projects/ProjectsPage.vue`: add missing project fields and submit payloads.
- `frontend/src/features/tasks/TaskDetailPage.vue`: wire status updates, comments, attachment upload/download/delete.
- `frontend/src/features/tasks/TaskFormPage.vue`: make task form responsive and replace the editor toolbar with the `.pen` toolbar set.
- `frontend/src/features/settings/SettingsPage.vue`: make settings values editable and call `PUT /settings`.
- `frontend/src/styles/main.css`: responsive form/grid/editor toolbar refinements.
- `backend/src/error.rs`: centrally log every backend `AppError` response.
- `backend/tests/config_and_error.rs`: verify failed API responses are loggable without leaking sensitive values.

## Tasks

- [ ] **Task 1: API contract tests and types**
  - Add Vitest coverage for project create/update payloads, settings update, comments, attachment upload/download/delete.
  - Run `npm test -- src/api/client.test.ts` and verify the new tests fail before implementation.
  - Add the minimal client methods/types and rerun the test.

- [ ] **Task 2: Project/settings forms**
  - Add project code, status, start date, end date to the project dialog.
  - Keep update payload compatible with the backend, which does not accept project code updates.
  - Make settings fields editable and save only non-empty edited values.
  - Run frontend tests and typecheck through `npm run build`.

- [ ] **Task 3: Task collaboration actions**
  - Add comment state and mutation for `POST /tasks/{id}/comments`.
  - Add upload input and mutation for `POST /tasks/{id}/attachments`.
  - Add download/delete handlers for attachment rows.
  - Add status selector and mutation for `PATCH /tasks/{id}/status`.
  - Invalidate task detail and task list queries after mutations.

- [ ] **Task 4: Rich text toolbar and responsive layout**
  - Add Tiptap underline and table extensions.
  - Match the `.pen` toolbar: 段落, H1, H2, H3, B, I, U, S, lists, help, link, image, table, code, quote, undo, redo.
  - Add wrapping toolbar styles and responsive `ElCol` breakpoints for task/project/user/settings forms.
  - Run `npm run build`.

- [ ] **Task 5: Backend failure logging**
  - Add a test that `AppError` display/debug contains stable error code context without sensitive values.
  - Log `AppError` in `ResponseError::error_response` with status, code, and details.
  - Run `cargo test`.

- [ ] **Task 6: Admin-token smoke validation**
  - Start backend and frontend locally with `ADMIN_AUTH_TOKEN`.
  - Open the app with `?admin-token=<token>` in the in-app browser.
  - Smoke test workbench, project creation form rendering, task detail actions, settings tabs, and responsive form wrapping at desktop/mobile widths.
