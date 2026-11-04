#!/usr/bin/env python3
"""Select an iPhone runtime compatible with the active Xcode's simulator SDK."""
import json
import subprocess


def output(*args):
    return subprocess.check_output(args, text=True)


sdk_major = output('xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version').strip().split('.')[0]
runtimes = json.loads(output('xcrun', 'simctl', 'list', 'runtimes', '-j'))['runtimes']
compatible = [runtime for runtime in runtimes
              if runtime.get('isAvailable')
              and runtime['identifier'].startswith('com.apple.CoreSimulator.SimRuntime.iOS-')
              and runtime['version'].split('.')[0] == sdk_major]
if not compatible:
    raise SystemExit(f'Install an iOS {sdk_major}.x simulator runtime for the selected Xcode.')
devices = json.loads(output('xcrun', 'simctl', 'list', 'devices', 'available', '-j'))['devices']
for runtime in sorted(compatible, key=lambda r: tuple(map(int, r['version'].split('.'))), reverse=True):
    phones = [device for device in devices.get(runtime['identifier'], [])
              if device.get('isAvailable') and device['name'].startswith('iPhone')]
    if phones:
        print(phones[0]['udid'])
        break
else:
    raise SystemExit('No available iPhone exists for an SDK-compatible runtime.')
