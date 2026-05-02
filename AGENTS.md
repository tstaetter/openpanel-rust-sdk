# Repository Guidelines

## Project Overview

`openpanel_rust` is a Rust SDK (published as `openpanel_sdk`) for sending tracking events to the [OpenPanel](https://openpanel.dev) analytics platform. It exposes a `Tracker` struct for tracking events, identifying users, incrementing/decrementing properties, and tracking revenue.

---

## Project Structure

```
openpanel_rust/
├── src/
│   ├── lib.rs              # Crate root: TrackerResult, TrackerError
│   └── sdk/
│       ├── mod.rs          # Tracker struct and all public methods
│       └── user.rs         # IdentifyUser struct
├── tests/
│   ├── tracking.rs         # Integration tests for track, identify, increment, decrement, revenue
│   ├── identify_user.rs    # Integration tests for user identification via From impl
│   └── global_properties.rs # Integration tests for global property merging
├── Cargo.toml
└── README.md
```

---

## Build, Test & Development Commands

```bash
# Build the library
cargo build

# Run all tests (unit + integration — requires a live OpenPanel endpoint and a .env file)
cargo nextest

# Run only unit tests (inside src/sdk/mod.rs)
cargo nextest --lib

# Run a specific integration test file
cargo nextest --test tracking

# Check for compile errors without producing a binary
cargo check

# Lint with Clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

> **Note:** Integration tests hit a real OpenPanel API. Create a `.env` file in the project root before running them:
> ```
> OPENPANEL_TRACK_URL=https://<your-instance>/api/track
> OPENPANEL_CLIENT_ID=<your-client-id>
> OPENPANEL_CLIENT_SECRET=<your-client-secret>
> ```

---

## Coding Style & Naming Conventions

- **Edition:** Rust 2024. **MSRV:** 1.85.1 — do not use features unavailable in this version.
- **Formatting:** `cargo fmt` (default `rustfmt` settings). All submitted code must be formatted.
- **Linting:** `cargo clippy -- -D warnings`. No Clippy warnings are acceptable.
- **Naming:** Follow standard Rust conventions — `snake_case` for functions/variables, `PascalCase` for types/enums, `SCREAMING_SNAKE_CASE` for constants.
- **Error handling:** Use `TrackerResult<T>` (alias for `Result<T, TrackerError>`) for all fallible public APIs. Extend `TrackerError` with new variants rather than using `anyhow` in library code.
- **Serialization:** JSON field names are `camelCase` via `#[serde(rename_all = "camelCase")]`; keep this consistent on any new serializable types.

---

## Testing Guidelines

- **Framework:** Standard `#[test]` / `#[tokio::test]` — no external test framework.
- **Unit tests:** Live in `#[cfg(test)]` modules within `src/sdk/mod.rs`.
- **Integration tests:** Live in `tests/` as separate files. Each file focuses on a single concern (tracking, user identification, global properties).
- **Naming pattern:** Test functions are named `can_<action>` or `cannot_<action>_if_<condition>` (e.g., `can_track_event`, `cannot_send_request_if_disabled`).
- **Return type:** All tests return `anyhow::Result<()>` for ergonomic `?` propagation.
- Integration tests require a valid `.env` file and a reachable OpenPanel instance.

---

## Commit & Pull Request Guidelines

- **Commit message format:** `<type>: <short description>` — e.g., `feat: added revenue tracking functionality`, `fix: filter error`, `doc: updated README`, `test: fixed tests`, `cargo: changed version`, `refactor: changed filter parameter`.
- **Types in use:** `feat`, `fix`, `doc`, `test`, `cargo`, `refactor`.
- **PRs:** Link related issues, include a short description of what changed and why. For behaviour-changing PRs, add or update the relevant integration test in `tests/`.
- Keep commits focused — one logical change per commit.
