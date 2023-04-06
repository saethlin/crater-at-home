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

docsrs_metadata = metadata.get('docs', {}).get('rs', {})
playground_metadata = metadata.get('playground', {})

args = []
if metadata.get('no-default-features', False) == True or playground_metadata.get('no-default-features') == True:
    args.append("--no-default-features")

if metadata.get('all-features', False) == True or playground_metadata.get('all-features') == True:
    args.append("--all-features")

features = set(docsrs_metadata.get('features', [])) | set(playground_metadata.get('features', []))
features = ",".join(features)
if features:
    args.append("--features=" + features)

print(" ".join(args))
