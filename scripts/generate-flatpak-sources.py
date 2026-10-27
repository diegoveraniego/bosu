#!/usr/bin/env python3
"""Generate pinned Flatpak crate downloads from Cargo.lock; uses only stdlib."""
import json
from pathlib import Path
import tomllib
root=Path(__file__).resolve().parents[1]
packages=tomllib.loads((root/'Cargo.lock').read_text())['package']
sources=[]
for package in packages:
    if 'source' not in package:
        continue
    assert package['source']=='registry+https://github.com/rust-lang/crates.io-index'
    name,version,checksum=package['name'],package['version'],package['checksum']
    destination=f'cargo/vendor/{name}-{version}'
    sources.append({'type':'archive','archive-type':'tar-gzip','url':f'https://static.crates.io/crates/{name}/{name}-{version}.crate','sha256':checksum,'dest':destination})
    sources.append({'type':'inline','contents':json.dumps({'package':checksum,'files':{}}),'dest':destination,'dest-filename':'.cargo-checksum.json'})
(root/'build-aux/cargo-sources.json').write_text(json.dumps(sources,indent=2)+'\n')
print(f'Pinned {len(sources)//2} crate downloads')
