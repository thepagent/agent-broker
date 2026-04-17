#!/bin/sh
set -e

# Seed ~/.gemini/projects.json with the required registry schema if missing or empty
PROJECTS_JSON="${HOME}/.gemini/projects.json"
mkdir -p "${HOME}/.gemini"
if [ ! -f "$PROJECTS_JSON" ] || [ "$(cat "$PROJECTS_JSON" 2>/dev/null)" = "{}" ] || [ ! -s "$PROJECTS_JSON" ]; then
  echo '{"projects":{}}' > "$PROJECTS_JSON"
fi

exec openab "$@"
