import subprocess
import json
import sys

desired_crate = sys.argv[1].split('==')[0]

config = subprocess.run(["cargo", "+miri", "metadata"], capture_output=True)
# If we can't get metadata, for whatever reason, just exit without printing anything
if config.returncode != 0:
    sys.exit(config.returncode)
config = json.loads(config.stdout)

metadata = {}
for crate in config['packages']:
    if crate['name'] == desired_crate:
        metadata = crate['metadata']
        if metadata is None :
            metadata = {}

metadata_sections = [
    metadata.get('docs', {}).get('rs', {}),
    metadata.get('docs.rs', {}),
    metadata.get('playground', {}),
]

args = []

for section in metadata_sections:
    if section.get('no-default-features', False) == True:
        args.append('--no-default-features')
        break

for section in metadata_sections:
    if section.get('all-features', False) == True:
        args.append("--all-features")
        break

features = set()
for section in metadata_sections:
    for feat in section.get('features', []):
        features.add(feat)

features = ",".join(features)
if features:
    args.append("--features=" + features)

print(" ".join(args))
