#!/usr/bin/env bash
set -euo pipefail

cargo metadata --no-deps --format-version 1 | python3 -c '
import json
import sys

meta = json.load(sys.stdin)
members = meta.get("workspace_members", [])
pkgs = {pkg["id"]: pkg for pkg in meta.get("packages", [])}

for member_id in members:
    pkg = pkgs.get(member_id)
    if not pkg:
        continue
    publish = pkg.get("publish", None)
    if publish is False:
        continue
    print(pkg["name"])
'
