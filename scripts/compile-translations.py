#!/usr/bin/env python3
"""Compile the PO message catalog to the embedded JSON format (stdlib only)."""
import argparse
import json
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
messages={}
message_id=translation=None
field=None
for raw in (root/'po/en.po').read_text().splitlines()+['']:
    line=raw.strip()
    if not line:
        if message_id:
            assert translation, f'Missing translation: {message_id}'
            assert message_id not in messages, f'Duplicate message: {message_id}'
            messages[message_id]=translation
        message_id=translation=None
        field=None
    elif line.startswith('#'):
        continue
    elif line.startswith('msgid '):
        message_id=json.loads(line[6:]);field='id'
    elif line.startswith('msgstr '):
        translation=json.loads(line[7:]);field='translation'
    elif line.startswith('"'):
        if field=='id':message_id+=json.loads(line)
        elif field=='translation':translation+=json.loads(line)
        else:raise ValueError(f'Unexpected continuation: {line}')
    else:
        raise ValueError(f'Unsupported PO syntax: {line}')
compiled=json.dumps(messages,ensure_ascii=False,sort_keys=True,indent=2)+'\n'
output=root/'po/en.json'
if args.check:
    assert output.read_text()==compiled, 'Run python scripts/compile-translations.py'
else:
    output.write_text(compiled)
print(f'{len(messages)} translated messages verified')
