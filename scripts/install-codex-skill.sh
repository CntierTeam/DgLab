#!/usr/bin/env bash
# Back-compat wrapper: install Codex skill from local checkout or from GitHub Release.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL="${SCRIPT_DIR}/install.sh"

MODE=from-source
FORCE_ARGS=()
EXTRA=(--skill-only)

usage() {
  cat <<'EOF'
Usage: ./scripts/install-codex-skill.sh [--symlink|--copy|--release] [--force] [--uninstall]

  --symlink    Local checkout: symlink skill into ~/.codex/skills (default)
  --copy       Local checkout: copy skill tree
  --release    Download dglab-skill.tar.gz from GitHub Releases
  --force      Replace existing install
  --uninstall  Remove installed skill (and leave binary alone via install.sh --skill-only)

This is a thin wrapper around scripts/install.sh.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --symlink) MODE=from-source; EXTRA=(--skill-only --from-source --symlink-skill); shift ;;
    --copy) MODE=from-source; EXTRA=(--skill-only --from-source); shift ;;
    --release) MODE=release; EXTRA=(--skill-only); shift ;;
    --force|-f) FORCE_ARGS+=(--force); shift ;;
    --uninstall) exec bash "${INSTALL}" --skill-only --uninstall ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "${MODE}" == "from-source" && ${#EXTRA[@]} -eq 1 ]]; then
  # default when only --force etc.
  EXTRA=(--skill-only --from-source --symlink-skill)
fi

exec bash "${INSTALL}" "${EXTRA[@]}" "${FORCE_ARGS[@]+"${FORCE_ARGS[@]}"}"
