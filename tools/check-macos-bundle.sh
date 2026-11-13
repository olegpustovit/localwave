#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
app=${1:-target/bundle/LocalWave.app}
codesign --verify --deep --strict "$app"
test "$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$app/Contents/Info.plist")" = com.olegpustovit.localwave
test "$(/usr/libexec/PlistBuddy -c 'Print LSMinimumSystemVersion' "$app/Contents/Info.plist")" = 15.0
python3 - "$app/Contents/MacOS/localwave" "$app/Contents/Frameworks/libpxui.1.dylib" <<'PY'
import re, subprocess, sys

def version(text):
    return (tuple(map(int, text.split('.'))) + (0, 0))[:3]

assert version('15.0') == version('15.0.0')
assert version('15.0.1') > version('15.0')
for binary in sys.argv[1:]:
    build = subprocess.run(['xcrun', 'vtool', '-show-build', binary], check=True, text=True, capture_output=True).stdout
    minimums = re.findall(r'\bminos\s+([0-9.]+)', build)
    assert re.search(r'\bplatform\s+MACOS\b', build), binary
    assert minimums and all(version(value) <= (15, 0, 0) for value in minimums), (binary, minimums)
print('LocalWave bundle: macOS 15 minimum load commands verified')
load_commands = subprocess.run(['otool', '-l', sys.argv[1]], check=True, text=True, capture_output=True).stdout
stack_sizes = re.findall(r'cmd LC_MAIN\s+cmdsize \d+\s+entryoff \d+\s+stacksize (0x[0-9a-fA-F]+|\d+)', load_commands)
assert stack_sizes and all(int(value, 0) == 0 for value in stack_sizes), ('app stack override', stack_sizes)
print('LocalWave bundle: default executable stack verified')
rpaths = re.findall(r'cmd LC_RPATH\s+cmdsize \d+\s+path (.+) \(offset \d+\)', load_commands)
assert rpaths == ['@executable_path/../Frameworks'], ('bundle rpaths', rpaths)
identity = subprocess.run(['otool', '-D', sys.argv[2]], check=True, text=True, capture_output=True).stdout.splitlines()[1:]
assert identity == ['@rpath/libpxui.1.dylib'], ('bundled dylib ID', identity)
print('LocalWave bundle: bundled dylib ID and portable rpath verified')
PY
test "$(find "$app/Contents/Frameworks" -type f | wc -l | tr -d ' ')" = 1
otool -L "$app/Contents/MacOS/localwave" | grep -q '@rpath/libpxui.1.dylib'
for binary in "$app/Contents/MacOS/localwave" "$app/Contents/Frameworks/libpxui.1.dylib"; do
    dependencies=$(otool -L "$binary" | tail -n +2 | awk '{print $1}')
    if printf '%s\n' "$dependencies" | grep -Ev '^(/usr/lib/|/System/Library/|@rpath/libpxui.1.dylib$)'; then
        echo "Unbundled dependency in $binary" >&2; exit 1
    fi
done
codesign -d --entitlements - --xml "$app" 2>/dev/null | python3 -c '
import plistlib, sys
values = plistlib.loads(sys.stdin.buffer.read())
for key in ("com.apple.security.app-sandbox", "com.apple.security.files.user-selected.read-only", "com.apple.security.files.bookmarks.app-scope"):
    assert values.get(key) is True, key
'
"$app/Contents/MacOS/localwave" --smoke-test
