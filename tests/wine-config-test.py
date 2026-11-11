#!/usr/bin/python3
"""Validate the Wine build hook using fake commands, never modify host apt."""
from pathlib import Path
import json
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
hook = root / 'scripts/lingmo-config/common/hooks/live/05-wine-multiarch.chroot'
packages = root / 'scripts/lingmo-config/common/package-lists/wine.list.chroot'
for arch in ('amd64', 'arm64', 'i386'):
    with tempfile.TemporaryDirectory(prefix='lingmo-wine-hook-') as temp:
        directory = Path(temp)
        trace = directory / 'calls'
        script = '''#!/usr/bin/python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
with open(os.environ['TRACE'], 'a') as file:
    file.write(json.dumps([name] + sys.argv[1:]) + chr(10))
if name == 'dpkg' and sys.argv[1:] == ['--print-architecture']:
    print(os.environ['TEST_ARCH'])
if name == 'dpkg-query':
    print('install ok installed')
if name == 'apt-get' and os.environ.get('FAIL_APT'):
    sys.exit(42)
'''
        for name in ('dpkg', 'apt-get', 'dpkg-query'):
            executable = directory / name
            executable.write_text(script)
            executable.chmod(0o755)
        env = dict(os.environ, PATH=str(directory) + ':' + os.environ['PATH'],
                   TRACE=str(trace), TEST_ARCH=arch)
        subprocess.run(['sh', str(hook)], env=env, check=True)
        calls = [json.loads(line) for line in trace.read_text().splitlines()]
        if arch == 'amd64':
            assert calls[1] == ['dpkg', '--add-architecture', 'i386']
            assert calls[2] == ['apt-get', 'update']
            assert 'wine32:i386' in calls[3]
            assert 'libgl1-mesa-dri:i386' in calls[3]
            assert calls[-1][0] == 'dpkg-query'
            env['FAIL_APT'] = '1'
            assert subprocess.run(['sh', str(hook)], env=env).returncode == 42
        else:
            assert calls == [['dpkg', '--print-architecture']]
        selected = []
        enabled = True
        for line in packages.read_text().splitlines():
            if line.startswith('#if ARCHITECTURES '):
                enabled = arch in line.split()[2:]
            elif line == '#endif':
                enabled = True
            elif enabled and line and not line.startswith('#'):
                selected.append(line)
        assert ('lingmo-wine' in selected) == (arch == 'amd64')
        print(arch + ': Wine image configuration passed')
