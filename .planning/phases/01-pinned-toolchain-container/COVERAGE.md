No external API integration: Phase 1 builds a pinned container and a subprocess runner; GitHub's REST API is only read by executors (gh api) to observe CI, and GHCR is reached only by docker push.
