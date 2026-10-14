"""Exercise fail-closed linking, and runtime policy without hardware."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).parent / 'System/offline.sh'

class Offline(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for directory in ['bin', 'run', 'data', 'sys/class/net/wlan0',
                          'sys/class/net/wlan1', 'sys/class/net/lo', 'proc/123',
                          'proc/sys/net/ipv6/conf/all', 'usr/sbin']:
            (self.root / directory).mkdir(parents=True, exist_ok=True)
        (self.root / 'usr/sbin/dropbear').touch()
        (self.root / 'proc/sys/net/ipv6/conf/all/disable_ipv6').write_text('0')
        self.log = self.root / 'events'
        for command in ['mount', 'killall', 'rfkill', 'ip']:
            path = self.root / 'bin' / command
            path.write_text('#!/bin/sh\nprintf "%s %s\\n" "${0##*/}" "$*" >> "$EVENTS"\n'
                            'if [ "${0##*/}" = mount ]; then exit "${MOUNT_FAIL:-0}"; fi\n')
            path.chmod(0o755)
        self.env = dict(os.environ, SLOT_OFFLINE_SYSROOT=str(self.root),
                        EVENTS=str(self.log), PATH=f'{self.root}/bin:' + os.environ['PATH'])

    def run_policy(self, verb):
        return subprocess.run(['sh', str(SCRIPT), verb], env=self.env, capture_output=True)

    def test_start_blocks_services(self):
        self.assertEqual(self.run_policy('start').returncode, 0)
        self.assertTrue((self.root / 'data/no-adb').exists())
        self.assertTrue((self.root / 'run/slot-offline-ready').exists())
        events = self.log.read_text()
        self.assertIn('killall dropbear', events)
        self.assertIn('killall smbd', events)
        self.assertIn('rfkill block wifi', events)
        self.assertNotIn('killall bluetoothd', events)
        self.assertEqual((self.root / 'proc/sys/net/ipv6/conf/all/disable_ipv6').read_text(), '1\n')

    def test_failed_mask_does_not_authorize_link(self):
        self.env['MOUNT_FAIL'] = '1'
        self.assertNotEqual(self.run_policy('start').returncode, 0)
        self.assertFalse((self.root / 'run/slot-offline-ready').exists())

    def test_live_link_is_preserved_but_stale_session_turns_off(self):
        marker = self.root / 'run/slotlink.session'
        marker.write_text('123\n')
        self.assertEqual(self.run_policy('once').returncode, 0)
        self.assertNotIn('rfkill block wifi', self.log.read_text())
        self.assertIn('killall dropbear', self.log.read_text())
        (self.root / 'proc/123').rmdir()
        self.assertEqual(self.run_policy('once').returncode, 0)
        self.assertIn('rfkill block wifi', self.log.read_text())
        self.assertFalse(marker.exists())

if __name__ == '__main__':
    unittest.main()
