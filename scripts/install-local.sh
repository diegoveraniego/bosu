#!/bin/sh
set -eu
# User installation; no administrator privileges or changes to health data.
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bosu_prefix=${BOSU_PREFIX:-"$HOME/.local"}
cargo build --release --locked --offline --manifest-path "$project_dir/Cargo.toml"
install -Dm755 "$project_dir/target/release/bosu" "$bosu_prefix/bin/.bosu-install-$$"
mv -f "$bosu_prefix/bin/.bosu-install-$$" "$bosu_prefix/bin/bosu"
install -Dm644 "$project_dir/data/org.diego.Bosu.desktop" "$bosu_prefix/share/applications/org.diego.Bosu.desktop"
install -Dm644 "$project_dir/data/org.diego.Bosu.metainfo.xml" "$bosu_prefix/share/metainfo/org.diego.Bosu.metainfo.xml"
install -Dm644 "$project_dir/data/icons/hicolor/scalable/apps/org.diego.Bosu.svg" "$bosu_prefix/share/icons/hicolor/scalable/apps/org.diego.Bosu.svg"
install -Dm644 "$project_dir/data/icons/hicolor/symbolic/apps/org.diego.Bosu-symbolic.svg" "$bosu_prefix/share/icons/hicolor/symbolic/apps/org.diego.Bosu-symbolic.svg"
install -Dm644 "$project_dir/LICENSE" "$bosu_prefix/share/licenses/bosu/LICENSE"
install -Dm644 "$project_dir/src/assets/body-map/NOTICE.md" "$bosu_prefix/share/doc/bosu/body-map-NOTICE.md"
# Desktop sessions do not necessarily include ~/.local/bin in PATH.
python3 - "$project_dir" "$bosu_prefix" <<'PYCODE'
from pathlib import Path
import sys
project, prefix = map(Path, sys.argv[1:])
executable = str(prefix / 'bin/bosu')
quoted = '"' + executable.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$') + '"'
for source, destination, command in [
    ('data/org.diego.Bosu.desktop', 'share/applications/org.diego.Bosu.desktop', quoted),
    ('data/org.diego.Bosu.service', 'share/dbus-1/services/org.diego.Bosu.service', quoted + ' --gapplication-service'),
]:
    target = prefix / destination
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text('\n'.join('Exec=' + command if line.startswith('Exec=') else line for line in (project / source).read_text().splitlines()) + '\n')
PYCODE
# A broken pre-existing user theme cache must not prevent installing this app.
bosu_cache_command=
if command -v gtk4-update-icon-cache >/dev/null 2>&1; then bosu_cache_command=gtk4-update-icon-cache
elif command -v gtk-update-icon-cache >/dev/null 2>&1; then bosu_cache_command=gtk-update-icon-cache
fi
if [ -n "$bosu_cache_command" ] && ! "$bosu_cache_command" -f -t "$bosu_prefix/share/icons/hicolor"; then
    python3 - "$bosu_prefix" <<'PYCODE'
from pathlib import Path
import sys
prefix = Path(sys.argv[1])
launcher = prefix / 'share/applications/org.diego.Bosu.desktop'
icon = prefix / 'share/icons/hicolor/scalable/apps/org.diego.Bosu.svg'
launcher.write_text('\n'.join('Icon=' + str(icon) if line.startswith('Icon=') else line for line in launcher.read_text().splitlines()) + '\n')
PYCODE
    printf 'Se usará el icono de Bosu directamente, sin depender del caché del tema.\n'
fi
if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$bosu_prefix/share/applications"; fi
printf 'Bosu instalado en %s/bin/bosu\n' "$bosu_prefix"
