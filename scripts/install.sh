#!/usr/bin/env bash
# Install DgLab binary + Codex skill from GitHub Releases (CntierTeam/DgLab).
# Also supports --from-source when run inside a git checkout.
# Works on Linux/macOS and Windows (Git Bash / MSYS2).
set -euo pipefail

REPO="${DGLAB_REPO:-CntierTeam/DgLab}"
VERSION="${DGLAB_VERSION:-latest}"
PREFIX="${DGLAB_PREFIX:-${HOME}/.local}"
BIN_DIR="${PREFIX}/bin"
CODEX_HOME="${CODEX_HOME:-${HOME}/.codex}"
SKILLS_DIR="${CODEX_HOME}/skills"
SKILL_DST="${SKILLS_DIR}/dglab"
API="https://api.github.com/repos/${REPO}"

INSTALL_BIN=1
INSTALL_SKILL=1
FROM_SOURCE=0
FORCE=0
UNINSTALL=0
SYMLINK_SKILL=0

BIN_NAME="dglab"
case "$(uname -s 2>/dev/null || true)" in
  MINGW*|MSYS*|CYGWIN*) BIN_NAME="dglab.exe" ;;
esac

usage() {
  cat <<EOF
Usage: install.sh [options]

Installs the dglab binary and/or Codex skill from GitHub Releases.

Options:
  --bin-only         Install binary only
  --skill-only       Install Codex skill only
  --from-source      Use local repo (binary via cargo, skill via copy/symlink)
  --symlink-skill    With --from-source: symlink skill (default: copy)
  --prefix DIR       Binary prefix (default: ~/.local) → DIR/bin/${BIN_NAME}
  --version VER      Release tag (default: latest), e.g. v0.1.2
  --repo OWNER/NAME  GitHub repo (default: ${REPO})
  --force            Replace existing install
  --uninstall        Remove binary + skill installed by this script
  -h, --help         Show help

Environment:
  DGLAB_REPO / DGLAB_VERSION / DGLAB_PREFIX / CODEX_HOME / GITHUB_TOKEN

Examples:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/scripts/install.sh | bash
  # Windows PowerShell:
  irm https://raw.githubusercontent.com/${REPO}/main/scripts/install.ps1 | iex
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin-only) INSTALL_SKILL=0; shift ;;
    --skill-only) INSTALL_BIN=0; shift ;;
    --from-source) FROM_SOURCE=1; shift ;;
    --symlink-skill) SYMLINK_SKILL=1; shift ;;
    --prefix) PREFIX="$2"; BIN_DIR="${PREFIX}/bin"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    --repo) REPO="$2"; API="https://api.github.com/repos/${REPO}"; shift 2 ;;
    --force|-f) FORCE=1; shift ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: missing required command: $1" >&2
    exit 1
  }
}

python_bin() {
  if command -v python3 >/dev/null 2>&1; then
    echo python3
  elif command -v python >/dev/null 2>&1; then
    echo python
  else
    echo "error: missing python3/python" >&2
    exit 1
  fi
}

install_file() {
  local src="$1" dst="$2"
  mkdir -p "$(dirname "${dst}")"
  if command -v install >/dev/null 2>&1; then
    install -m 0755 "${src}" "${dst}"
  else
    cp "${src}" "${dst}"
    chmod 0755 "${dst}" 2>/dev/null || true
  fi
}

CURL_OPTS=(--fail --show-error --location --retry 5 --retry-all-errors --retry-delay 2 --connect-timeout 20)

http_get() {
  local url="$1" out="$2"
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    curl "${CURL_OPTS[@]}" -H "Authorization: Bearer ${GITHUB_TOKEN}" -H "Accept: application/octet-stream" "$url" -o "$out"
  else
    curl "${CURL_OPTS[@]}" -H "Accept: application/octet-stream" "$url" -o "$out"
  fi
}

http_json() {
  local url="$1"
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    curl "${CURL_OPTS[@]}" -H "Authorization: Bearer ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json" "$url"
  else
    curl "${CURL_OPTS[@]}" -H "Accept: application/vnd.github+json" "$url"
  fi
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}/${arch}" in
    Linux/x86_64|Linux/amd64) echo "x86_64-unknown-linux-gnu" ;;
    Linux/aarch64|Linux/arm64) echo "aarch64-unknown-linux-gnu" ;;
    Darwin/arm64|Darwin/aarch64) echo "aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "x86_64-apple-darwin" ;;
    MINGW64_NT*|MSYS_NT*|CYGWIN_NT*)
      case "${arch}" in
        x86_64|amd64) echo "x86_64-pc-windows-msvc" ;;
        *)
          echo "error: unsupported Windows arch: ${arch}" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      # Git Bash sometimes reports MINGW64_NT-10.0-... with slash form above failing
      case "${os}" in
        MINGW*|MSYS*|CYGWIN*)
          case "${arch}" in
            x86_64|amd64) echo "x86_64-pc-windows-msvc" ;;
            *)
              echo "error: unsupported Windows arch: ${arch}" >&2
              exit 1
              ;;
          esac
          ;;
        *)
          echo "error: unsupported platform: ${os}/${arch}" >&2
          exit 1
          ;;
      esac
      ;;
  esac
}

uninstall_all() {
  if [[ -e "${BIN_DIR}/${BIN_NAME}" || -L "${BIN_DIR}/${BIN_NAME}" ]]; then
    rm -f "${BIN_DIR}/${BIN_NAME}"
    echo "removed ${BIN_DIR}/${BIN_NAME}"
  fi
  # legacy unix name on Windows installs
  if [[ "${BIN_NAME}" == "dglab.exe" && ( -e "${BIN_DIR}/dglab" || -L "${BIN_DIR}/dglab" ) ]]; then
    rm -f "${BIN_DIR}/dglab"
    echo "removed ${BIN_DIR}/dglab"
  fi
  if [[ -e "${SKILL_DST}" || -L "${SKILL_DST}" ]]; then
    rm -rf "${SKILL_DST}"
    echo "removed ${SKILL_DST}"
  fi
}

if [[ "${UNINSTALL}" -eq 1 ]]; then
  uninstall_all
  exit 0
fi

need_cmd curl
need_cmd tar
need_cmd uname

TMP="$(mktemp -d)"
cleanup() { rm -rf "${TMP}"; }
trap cleanup EXIT

ensure_replace() {
  local path="$1"
  if [[ -e "${path}" || -L "${path}" ]]; then
    if [[ "${FORCE}" -ne 1 ]]; then
      echo "error: already exists: ${path}" >&2
      echo "hint: re-run with --force" >&2
      exit 1
    fi
    rm -rf "${path}"
  fi
}

install_skill_dir() {
  local src="$1"
  mkdir -p "${SKILLS_DIR}"
  ensure_replace "${SKILL_DST}"
  if [[ "${SYMLINK_SKILL}" -eq 1 ]]; then
    ln -s "$(cd "${src}" && pwd)" "${SKILL_DST}"
    echo "skill (symlink): ${SKILL_DST} -> ${src}"
  else
    mkdir -p "${SKILL_DST}"
    cp -a "${src}/." "${SKILL_DST}/" 2>/dev/null || cp -R "${src}/." "${SKILL_DST}/"
    echo "skill (copy): ${SKILL_DST}"
  fi
  grep -q '^name: dglab$' "${SKILL_DST}/SKILL.md"
}

install_from_source() {
  local script_dir repo_root built
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  repo_root="$(cd "${script_dir}/.." && pwd)"

  if [[ "${INSTALL_BIN}" -eq 1 ]]; then
    need_cmd cargo
    (cd "${repo_root}" && cargo build --release)
    built="${repo_root}/target/release/${BIN_NAME}"
    if [[ ! -f "${built}" && "${BIN_NAME}" == "dglab.exe" ]]; then
      built="${repo_root}/target/release/dglab"
    fi
    [[ -f "${built}" ]] || { echo "error: built binary not found" >&2; exit 1; }
    mkdir -p "${BIN_DIR}"
    ensure_replace "${BIN_DIR}/${BIN_NAME}"
    install_file "${built}" "${BIN_DIR}/${BIN_NAME}"
    echo "binary: ${BIN_DIR}/${BIN_NAME}"
  fi

  if [[ "${INSTALL_SKILL}" -eq 1 ]]; then
    install_skill_dir "${repo_root}/.codex/skills/dglab"
  fi
}

asset_url_by_name() {
  local json="$1" name="$2" py
  py="$(python_bin)"
  "${py}" - "$json" "$name" <<'PY' 2>/dev/null || true
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
want = sys.argv[2]
for a in data.get("assets", []):
    if a.get("name") == want:
        print(a.get("browser_download_url") or "")
        break
PY
}

fetch_release_json() {
  local url
  if [[ "${VERSION}" == "latest" ]]; then
    url="${API}/releases/latest"
  else
    url="${API}/releases/tags/${VERSION}"
  fi
  http_json "${url}" > "${TMP}/release.json"
}

install_from_release() {
  local py
  py="$(python_bin)"
  fetch_release_json
  local tag
  tag="$("${py}" -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["tag_name"])' "${TMP}/release.json")"
  echo "release: ${REPO}@${tag}"

  if [[ "${INSTALL_BIN}" -eq 1 ]]; then
    local target asset url extracted_bin
    target="$(detect_target)"
    asset="dglab-${target}.tar.gz"
    url="$(asset_url_by_name "${TMP}/release.json" "${asset}")"
    if [[ -z "${url}" ]]; then
      echo "error: asset not found in release: ${asset}" >&2
      echo "available assets:" >&2
      "${py}" -c 'import json,sys; [print(" -", a["name"]) for a in json.load(open(sys.argv[1], encoding="utf-8")).get("assets",[])]' "${TMP}/release.json" >&2
      exit 1
    fi
    echo "downloading ${asset}"
    http_get "${url}" "${TMP}/${asset}"
    tar -C "${TMP}" -xzf "${TMP}/${asset}"
    extracted_bin="$(find "${TMP}" -type f \( -name dglab -o -name 'dglab.exe' \) | head -n1)"
    [[ -n "${extracted_bin}" ]] || { echo "error: dglab binary missing in archive" >&2; exit 1; }
    mkdir -p "${BIN_DIR}"
    ensure_replace "${BIN_DIR}/${BIN_NAME}"
    install_file "${extracted_bin}" "${BIN_DIR}/${BIN_NAME}"
    echo "binary: ${BIN_DIR}/${BIN_NAME}"
  fi

  if [[ "${INSTALL_SKILL}" -eq 1 ]]; then
    local skill_asset skill_url
    skill_asset="dglab-skill.tar.gz"
    skill_url="$(asset_url_by_name "${TMP}/release.json" "${skill_asset}")"
    if [[ -z "${skill_url}" ]]; then
      echo "error: asset not found in release: ${skill_asset}" >&2
      exit 1
    fi
    echo "downloading ${skill_asset}"
    http_get "${skill_url}" "${TMP}/${skill_asset}"
    mkdir -p "${TMP}/skill"
    tar -C "${TMP}/skill" -xzf "${TMP}/${skill_asset}"
    install_skill_dir "${TMP}/skill/dglab"
  fi
}

if [[ "${FROM_SOURCE}" -eq 1 ]]; then
  install_from_source
else
  install_from_release
fi

echo
echo "Done."
if [[ "${INSTALL_BIN}" -eq 1 ]]; then
  case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
      echo "note: add to PATH → export PATH=\"${BIN_DIR}:\$PATH\""
      ;;
  esac
  echo "try: ${BIN_NAME} --mock --headless"
fi
if [[ "${INSTALL_SKILL}" -eq 1 ]]; then
  echo "Codex skill: \$dglab (restart Codex / new session if already running)"
  echo "Codex home: ${CODEX_HOME}"
fi
