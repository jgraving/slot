"""Exercise pairing and routing using fake BaseOS commands; no radio required."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).parent / 'System/bluetooth.sh'
MAC = 'AA:BB:CC:DD:EE:FF'

class BluetoothTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        bin_dir = self.root / 'bin'
        bin_dir.mkdir()
        (self.root / 'hci0').mkdir()
        init = bin_dir / 'init'
        init.write_text('#!/bin/sh\nexit 0\n')
        for name in ['init', 'pidof', 'systemctl', 'bluealsa', 'rfkill']:
            p = bin_dir / name
            p.write_text('#!/bin/sh\nexit 0\n')
            p.chmod(0o755)
        bt = bin_dir / 'bluetoothctl'
        bt.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> "$SLOT_ROOT/calls"
case "$*" in
 '--agent NoInputNoOutput')
   echo 'Agent registered'
   while read -r command; do
     printf 'session %s\\n' "$command" >> "$SLOT_ROOT/calls"
     case "$command" in
       default-agent) echo 'Default agent request successful';;
       'scan on') echo 'Discovery started';;
       pair*) if [ "${MOCK_PAIR_FAIL:-no}" = yes ]; then echo 'Failed to pair: org.bluez.Error.AuthenticationFailed'; else touch "$SLOT_ROOT/paired"; echo 'Pairing successful'; fi;;
       quit) exit 0;;
     esac
   done;;
 *show*) echo "Controller AA:BB:CC:DD:EE:00"; echo "Powered: yes";;
 *devices*) echo 'Device AA:BB:CC:DD:EE:FF My headphones'; echo 'Device 00:11:22:33:44:55 00-11-22-33-44-55';;
 *info*)
   if [ -f "$SLOT_ROOT/paired" ]; then echo 'Paired: yes'; else echo "Paired: ${MOCK_PAIRED:-yes}"; fi
   echo "Connected: ${MOCK_CONNECTED:-yes}"
   echo "UUID: Audio Sink (${MOCK_UUID:-0000110b-0000-1000-8000-00805f9b34fb})";;
esac
''')
        bt.chmod(0o755)
        self.env = dict(os.environ, PATH=str(bin_dir) + ':' + os.environ['PATH'],
            SLOT_ROOT=str(self.root), SLOT_AUDIO_ROUTE=str(self.root / 'route'),
            SLOT_BT_LOCK=str(self.root / 'lock'), SLOT_BT_INIT=str(init),
            SLOT_HCI_PATH=str(self.root / 'hci0'), SLOT_BT_POLL_INTERVAL='0.01', SLOT_BT_SCAN_SECONDS='0')

    def tearDown(self):
        self.temp.cleanup()

    def run_command(self, *args):
        return subprocess.run(['sh', str(SCRIPT), *args], env=self.env,
            capture_output=True, text=True, timeout=5)

    def test_connect_and_speaker(self):
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'route').read_text().strip(), f'bluealsa:DEV={MAC},PROFILE=a2dp')
        self.assertEqual((self.root / 'Config/bluetooth-headphones').read_text().strip(), MAC)
        self.assertIn('DEVICE ' + MAC + ' My headphones', result.stdout)
        self.assertEqual(self.run_command('speaker').returncode, 0)
        self.assertEqual((self.root / 'route').read_text().strip(), '')
        self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())

    def test_failed_connection_does_not_select_audio(self):
        self.env['MOCK_CONNECTED'] = 'no'
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / 'route').exists())
        self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())

    def test_controller_without_headphone_profile_is_rejected(self):
        self.env['MOCK_UUID'] = '00001124-0000-1000-8000-00805f9b34fb'
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('A2DP', result.stderr)
        self.assertFalse((self.root / 'route').exists())

    def test_invalid_address_never_reaches_bluetoothctl(self):
        result = self.run_command('connect', '$(touch /bad)')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / 'calls').exists())

    def test_restore_and_forget(self):
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)
        self.assertEqual(self.run_command('restore').returncode, 0)
        self.assertEqual(self.run_command('forget', MAC).returncode, 0)
        self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())
        self.assertEqual((self.root / 'route').read_text().strip(), '')

    def test_live_agent_pairs_unpaired_headphones(self):
        self.env['MOCK_PAIRED'] = 'no'
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = (self.root / 'calls').read_text()
        self.assertIn('session default-agent', calls)
        self.assertIn('session pair ' + MAC, calls)

    def test_pair_failure_creates_attachable_report(self):
        self.env.update(MOCK_PAIRED='no', MOCK_PAIR_FAIL='yes')
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        report = (self.root / 'Bluetooth/latest-error.txt').read_text()
        self.assertIn('AuthenticationFailed', report)
        self.assertIn('Live pairing session', report)

    def test_scan_hides_mac_only_names(self):
        result = self.run_command('scan')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('My headphones', result.stdout)
        self.assertNotIn('DEVICE 00:11:22:33:44:55', result.stdout)

    def test_restore_without_preference_does_not_start_radio(self):
        self.assertEqual(self.run_command('restore').returncode, 0)
        self.assertFalse((self.root / 'calls').exists())

if __name__ == '__main__':
    unittest.main()
