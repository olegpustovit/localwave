#!/usr/bin/env python3
"""Fast contract checks for the Apple versions emitted by release CI."""
import os
from pathlib import Path
import subprocess

script = Path(__file__).with_name('release-metadata.sh')

def metadata(**overrides):
    env = dict(os.environ, GITHUB_REF_TYPE='branch', RELEASE_VERSION='1.0.0',
               GITHUB_RUN_NUMBER='23', GITHUB_RUN_ATTEMPT='2')
    return subprocess.run(['bash', str(script)], env=dict(env, **overrides),
                          text=True, capture_output=True)

assert metadata().stdout == 'version=1.0.0\nbuild=23.2\n'
for tag in ('v1.2.3', 'v1.2.3-beta.2', 'v1.2.3+build.4'):
    result = metadata(GITHUB_REF_TYPE='tag', GITHUB_REF_NAME=tag)
    assert result.returncode == 0 and result.stdout == 'version=1.2.3\nbuild=23.2\n'
for overrides in ({'RELEASE_VERSION': '1.0'}, {'RELEASE_VERSION': 'bad\nversion=1.0.0'},
                  {'GITHUB_RUN_NUMBER': '10000'}, {'GITHUB_RUN_ATTEMPT': '100'}):
    assert metadata(**overrides).returncode != 0
print('Release metadata checks passed')
