"""Exercise pairing and routing using fake BaseOS commands; no radio required."""
import os
import time
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
        (self.root / 'bluealsa-plugin.so').touch()
        init = bin_dir / 'init'
        init.write_text('#!/bin/sh\nexit 0\n')
        for name in ['init', 'pidof', 'systemctl', 'bluealsa', 'rfkill']:
            p = bin_dir / name
            p.write_text('#!/bin/sh\nexit 0\n')
            p.chmod(0o755)
        aplay = bin_dir / 'aplay'
        aplay.write_text('#!/bin/sh\necho "${MOCK_PCM:-bluealsa}"\n')
        aplay.chmod(0o755)
        dbus = bin_dir / 'dbus-send'
        dbus.write_text('''#!/bin/sh
count=$(cat "$SLOT_ROOT/pcm-count" 2>/dev/null || echo 0)
count=$((count + 1)); echo "$count" > "$SLOT_ROOT/pcm-count"
if [ "${MOCK_AUDIO_PCM:-yes}" = yes ] && [ "$count" -gt "${MOCK_PCM_DELAY:-0}" ]; then
  printf 'object path "%s"\\n' "/org/bluealsa/hci0/dev_${MOCK_PCM_DEVICE:-AA_BB_CC_DD_EE_FF}/a2dpsrc/sink"
else
  echo 'array []'
fi
''')
        dbus.chmod(0o755)
        bt = bin_dir / 'bluetoothctl'
        bt.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> "$SLOT_ROOT/calls"
case "$*" in
 ''|'--agent NoInputNoOutput')
   echo 'Agent registered'
   while read -r command; do
     printf 'session %s\\n' "$command" >> "$SLOT_ROOT/calls"
     case "$command" in
       default-agent) echo 'Default agent request successful';;
       'scan on') echo 'Discovery started';;
       pair*) if [ "${MOCK_AUTH_PROMPT:-no}" = yes ]; then echo "[agent] Authorize pairing (yes/no):"; elif [ "${MOCK_PAIR_FAIL:-no}" = yes ]; then echo 'Failed to pair: org.bluez.Error.AuthenticationFailed'; else touch "$SLOT_ROOT/paired"; echo 'Pairing successful'; fi;;
       connect*)
         if [ "${MOCK_REQUIRE_TRUST:-no}" = yes ] && [ ! -f "$SLOT_ROOT/trusted" ]; then echo 'Failed to connect: org.bluez.Error.Rejected';
         else
           sleep "${MOCK_AUDIO_DELAY:-0}"
           touch "$SLOT_ROOT/audio-ready"
           echo 'Connection successful'
         fi;;
       quit) exit 0;;
     esac
   done;;
 *show*)
   [ "${MOCK_CONTROLLER:-yes}" = yes ] || exit 1
   echo "Controller AA:BB:CC:DD:EE:00"; if [ -f "$SLOT_ROOT/powered-off" ]; then echo "Powered: no"; else echo "Powered: yes"; fi
   count=$(cat "$SLOT_ROOT/show-count" 2>/dev/null || echo 0)
   count=$((count + 1)); echo "$count" > "$SLOT_ROOT/show-count"
   if [ "$count" -gt "${MOCK_SOURCE_DELAY:-0}" ]; then echo 'UUID: Audio Source (0000110a-0000-1000-8000-00805f9b34fb)'; touch "$SLOT_ROOT/source-ready"; fi;;
 trust*) touch "$SLOT_ROOT/trusted";;
 'power off') touch "$SLOT_ROOT/powered-off";;
 'power on') rm -f "$SLOT_ROOT/powered-off";;
 connect*)
   if [ "${MOCK_REQUIRE_READY:-no}" = yes ] && [ ! -f "$SLOT_ROOT/source-ready" ]; then echo 'Failed to connect: org.bluez.Error.Failed br-connection-profile-unavailable'; exit 1; fi
   if [ "${MOCK_CONNECT_ERROR:-}" != '' ]; then echo "Failed to connect: $MOCK_CONNECT_ERROR"; exit 1; fi
   if [ "${MOCK_PROFILE_ONCE:-no}" = yes ] && [ ! -f "$SLOT_ROOT/connect-retried" ]; then touch "$SLOT_ROOT/connect-retried"; echo 'Failed to connect: org.bluez.Error.Failed br-connection-profile-unavailable'; exit 1; fi;;
 *devices*) echo 'Device AA:BB:CC:DD:EE:FF My headphones'; echo 'Device 00:11:22:33:44:55 00-11-22-33-44-55';;
 *info*)
   [ "${MOCK_ALIAS:-}" = "" ] || echo "Alias: $MOCK_ALIAS"
   if [ -f "$SLOT_ROOT/paired" ]; then echo 'Paired: yes'; else echo "Paired: ${MOCK_PAIRED:-yes}"; fi
   echo "Connected: ${MOCK_CONNECTED:-yes}"
   echo "UUID: Audio Sink (${MOCK_UUID:-0000110b-0000-1000-8000-00805f9b34fb})";;
esac
''')
        bt.chmod(0o755)
        self.env = dict(os.environ, PATH=str(bin_dir) + ':' + os.environ['PATH'],
            SLOT_ROOT=str(self.root), SLOT_AUDIO_ROUTE=str(self.root / 'route'),
            SLOT_BT_LOCK=str(self.root / 'lock'), SLOT_BT_INIT=str(init),
            SLOT_HCI_PATH=str(self.root / 'hci0'), SLOT_BASEOS_RELEASE=str(self.root / 'baseos-release'),
            SLOT_BLUEALSA_PLUGIN=str(self.root / 'bluealsa-plugin.so'), SLOT_BT_POLL_INTERVAL='0.01', SLOT_BT_SCAN_SECONDS='0', SLOT_BT_LOCK_TRIES='150')

    def tearDown(self):
        self.temp.cleanup()

    def run_command(self, *args):
        return subprocess.run(['sh', str(SCRIPT), *args], env=self.env,
            capture_output=True, text=True, timeout=5)

    def test_supported_requires_real_controller_and_audio_plugin(self):
        result = self.run_command('supported')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / 'Bluetooth').exists())
        self.assertFalse((self.root / 'route').exists())
        calls = (self.root / 'calls').read_text()
        self.assertEqual(calls.strip(), 'show')
        self.env['MOCK_CONTROLLER'] = 'no'
        self.assertNotEqual(self.run_command('supported').returncode, 0)
        self.env['MOCK_CONTROLLER'] = 'yes'
        (self.root / 'bluealsa-plugin.so').unlink()
        self.assertNotEqual(self.run_command('supported').returncode, 0)

    def test_clean_baseos_install_does_not_need_running_radio_or_pcm_hint(self):
        (self.root / 'baseos-release').write_text('BASEOS=1\nBASEOS_TARGET=rg35xxsp\nBASEOS_BLUETOOTH=1\n')
        self.env.update(MOCK_CONTROLLER='no', MOCK_PCM='default')
        Path(self.env['SLOT_BT_INIT']).write_text('#!/bin/sh\nexit 1\n')
        result = self.run_command('supported')
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertFalse((self.root / 'calls').exists())
        self.assertFalse((self.root / 'route').exists())
        self.assertIn('radio starts when needed', (self.root / 'Logs/bluetooth-support.log').read_text())

    def test_baseos_without_bluetooth_stays_hidden(self):
        (self.root / 'baseos-release').write_text('BASEOS_BLUETOOTH=0\n')
        self.assertNotEqual(self.run_command('supported').returncode, 0)
        self.assertFalse((self.root / 'calls').exists())
        self.assertIn('no Bluetooth hardware', (self.root / 'Logs/bluetooth-support.log').read_text())

    def test_hardware_flag_does_not_bypass_missing_audio_software(self):
        (self.root / 'baseos-release').write_text('BASEOS_BLUETOOTH=1\n')
        (self.root / 'bluealsa-plugin.so').unlink()
        self.assertNotEqual(self.run_command('supported').returncode, 0)
        self.assertIn('plugin is missing', (self.root / 'Logs/bluetooth-support.log').read_text())

    def test_audio_plugin_does_not_require_pcm_listing(self):
        self.env['MOCK_PCM'] = 'default'
        self.assertEqual(self.run_command('supported').returncode, 0)

    def test_missing_tools_hide_support_without_initializing(self):
        self.env['SLOT_BLUETOOTHCTL'] = str(self.root / 'missing-bluetoothctl')
        self.assertNotEqual(self.run_command('supported').returncode, 0)
        self.assertFalse((self.root / 'calls').exists())
        self.assertFalse((self.root / 'Bluetooth').exists())

    def test_generic_adapter_does_not_need_baseos_init(self):
        self.env['SLOT_BT_INIT'] = str(self.root / 'not-baseos')
        self.assertEqual(self.run_command('supported').returncode, 0)
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)

    def test_failed_vendor_initialization_hides_support(self):
        (self.root / 'hci0').rmdir()
        (self.root / 'bin/pidof').write_text('#!/bin/sh\nexit 1\n')
        Path(self.env['SLOT_BT_INIT']).write_text('#!/bin/sh\nexit 1\n')
        self.assertNotEqual(self.run_command('supported').returncode, 0)
        self.assertFalse((self.root / 'calls').exists())

    def test_disconnect_fallback_keeps_pairing_preference(self):
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)
        self.env['MOCK_CONNECTED'] = 'no'
        self.assertEqual(self.run_command('fallback', MAC).returncode, 0)
        self.assertEqual((self.root / 'route').read_text().strip(), '')
        self.assertEqual((self.root / 'Config/bluetooth-headphones').read_text().strip(), MAC)

    def test_stale_disconnect_cannot_replace_a_new_output_or_reconnected_device(self):
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)
        route = (self.root / 'route').read_text()
        self.assertEqual(self.run_command('fallback', MAC).returncode, 0)
        self.assertEqual((self.root / 'route').read_text(), route)
        self.env['MOCK_CONNECTED'] = 'no'
        self.assertEqual(self.run_command('fallback', '00:11:22:33:44:55').returncode, 0)
        self.assertEqual((self.root / 'route').read_text(), route)

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

    def test_connection_waits_for_audio_source_registration(self):
        self.env.update(MOCK_SOURCE_DELAY='4', MOCK_REQUIRE_READY='yes')
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = (self.root / 'calls').read_text().splitlines()
        first_connect = calls.index('connect ' + MAC)
        self.assertGreaterEqual(calls[:first_connect].count('show'), 5)

    def test_endpoint_race_gets_one_connection_retry(self):
        self.env['MOCK_PROFILE_ONCE'] = 'yes'
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'calls').read_text().splitlines().count('connect ' + MAC), 2)

    def test_list_and_diagnostics_do_not_remove_or_wait_for_another_lock(self):
        lock = self.root / 'lock'; lock.mkdir()
        (lock / 'pid').write_text(str(os.getpid()))
        (lock / 'verb').write_text('restore')
        for command in ['list', 'diagnose']:
            result = self.run_command(command)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((lock / 'pid').read_text(), str(os.getpid()))
        self.assertFalse((lock / 'cancel').exists())

    def test_manual_speaker_cancels_only_automatic_reconnect(self):
        (self.root / 'Config').mkdir()
        (self.root / 'Config/bluetooth-headphones').write_text(MAC)
        env = dict(self.env, MOCK_CONNECTED='no', SLOT_BT_POLL_INTERVAL='0.05')
        worker = subprocess.Popen(['sh', str(SCRIPT), 'restore'], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 2
            while not (self.root / 'lock/verb').exists() and time.monotonic() < deadline: time.sleep(0.01)
            self.assertEqual((self.root / 'lock/verb').read_text().strip(), 'restore')
            result = self.run_command('speaker')
            self.assertEqual(result.returncode, 0, result.stderr)
            _, error = worker.communicate(timeout=3)
            self.assertEqual(worker.returncode, 0, error)
            self.assertFalse((self.root / 'lock').exists())
            self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())
            self.assertFalse(list((self.root / 'Bluetooth').glob('operation-*.log')))
        finally:
            if worker.poll() is None: worker.kill(); worker.communicate()

    def test_restore_with_unavailable_headphones_keeps_preference_and_manual_errors(self):
        (self.root / 'Config').mkdir()
        (self.root / 'Config/bluetooth-headphones').write_text(MAC)
        (self.root / 'Bluetooth').mkdir()
        previous = self.root / 'Bluetooth/latest-error.txt'; previous.write_text('previous manual error')
        self.env['MOCK_CONNECTED'] = 'no'
        result = self.run_command('restore')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(previous.read_text(), 'previous manual error')
        self.assertTrue((self.root / 'Bluetooth/latest-restore.txt').exists())
        self.assertTrue((self.root / 'Config/bluetooth-headphones').exists())
        self.assertFalse((self.root / 'lock').exists())

    def test_live_manual_lock_is_preserved_and_busy_failure_is_logged(self):
        lock = self.root / 'lock'; lock.mkdir()
        (lock / 'pid').write_text(str(os.getpid())); (lock / 'verb').write_text('scan')
        self.env['SLOT_BT_LOCK_TRIES'] = '3'
        result = self.run_command('speaker')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((lock / 'pid').read_text(), str(os.getpid()))
        self.assertFalse((lock / 'cancel').exists())
        self.assertIn('Another Bluetooth operation', (self.root / 'Bluetooth/latest-error.txt').read_text())

    def test_orphaned_lock_without_pid_is_reclaimed(self):
        (self.root / 'lock').mkdir()
        result = self.run_command('speaker')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / 'lock').exists())

    def test_restore_without_preference_does_not_start_radio(self):
        self.assertEqual(self.run_command('restore').returncode, 0)
        self.assertFalse((self.root / 'calls').exists())

    def test_incoming_pairing_stays_disabled_for_scan_connect_and_restore(self):
        for verb, args in [('scan', []), ('connect', [MAC]), ('restore', [])]:
            result = self.run_command(verb, *args)
            self.assertEqual(result.returncode, 0, result.stderr)
        calls = (self.root / 'calls').read_text()
        self.assertNotIn('pairable on', calls)
        self.assertNotIn('discoverable on', calls)
        self.assertNotIn('session default-agent', calls)

    def test_ambiguous_authorization_is_rejected_and_never_trusted(self):
        self.env.update(MOCK_PAIRED='no', MOCK_AUTH_PROMPT='yes')
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        calls = (self.root / 'calls').read_text()
        self.assertNotIn('session yes', calls)
        self.assertNotIn('session 0000', calls)
        self.assertNotIn('trust ' + MAC, calls)
        self.assertFalse((self.root / 'route').exists())

    def test_automatic_reconnect_never_re_pairs_and_does_not_write_reports(self):
        (self.root / 'Config').mkdir()
        (self.root / 'Config/bluetooth-headphones').write_text(MAC)
        self.env['MOCK_PAIRED'] = 'no'
        result = self.run_command('reconnect')
        self.assertNotEqual(result.returncode, 0)
        calls = (self.root / 'calls').read_text()
        self.assertNotIn('--agent', calls)
        self.assertNotIn('session pair', calls)
        self.assertEqual(list((self.root / 'Bluetooth').iterdir()), [])

    def test_off_clears_reconnect_choice_and_stays_off_until_explicit_use(self):
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)
        result = self.run_command('off')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / 'Config/bluetooth-off').exists())
        self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())
        self.assertEqual((self.root / 'route').read_text().strip(), '')
        calls = (self.root / 'calls').read_text()
        for verb in ['restore', 'reconnect']:
            self.assertEqual(self.run_command(verb).returncode, 0)
        self.assertEqual((self.root / 'calls').read_text(), calls)
        self.assertEqual(self.run_command('connect', MAC).returncode, 0)
        self.assertFalse((self.root / 'Config/bluetooth-off').exists())

    def test_rejected_non_audio_device_is_never_trusted(self):
        self.env['MOCK_UUID'] = '00001124-0000-1000-8000-00805f9b34fb'
        self.assertNotEqual(self.run_command('connect', MAC).returncode, 0)
        self.assertNotIn('trust ' + MAC, (self.root / 'calls').read_text())

    def test_off_cancels_an_in_flight_periodic_reconnect(self):
        (self.root / 'Config').mkdir()
        (self.root / 'Config/bluetooth-headphones').write_text(MAC)
        env = dict(self.env, MOCK_CONNECTED='no', SLOT_BT_POLL_INTERVAL='0.05')
        worker = subprocess.Popen(['sh', str(SCRIPT), 'reconnect'], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 2
            while not (self.root / 'lock/verb').exists() and time.monotonic() < deadline: time.sleep(0.01)
            self.assertEqual((self.root / 'lock/verb').read_text().strip(), 'reconnect')
            result = self.run_command('off')
            self.assertEqual(result.returncode, 0, result.stderr)
            worker.communicate(timeout=3)
            self.assertTrue((self.root / 'Config/bluetooth-off').exists())
            self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())
            self.assertEqual((self.root / 'route').read_text().strip(), '')
        finally:
            if worker.poll() is None: worker.kill(); worker.communicate()

    def test_device_alias_cannot_fake_pairing_or_audio_service(self):
        (self.root / 'Config').mkdir()
        (self.root / 'Config/bluetooth-headphones').write_text(MAC)
        self.env.update(MOCK_PAIRED='no', MOCK_ALIAS='Paired: yes')
        self.assertNotEqual(self.run_command('reconnect').returncode, 0)
        self.assertNotIn('connect ' + MAC, (self.root / 'calls').read_text())
        self.env.update(MOCK_PAIRED='yes', MOCK_UUID='00001124-0000-1000-8000-00805f9b34fb', MOCK_ALIAS='0000110b-0000-1000-8000-00805f9b34fb')
        self.assertNotEqual(self.run_command('connect', MAC).returncode, 0)
        self.assertNotIn('trust ' + MAC, (self.root / 'calls').read_text())

    def test_logs_are_bounded_and_only_owned_legacy_files_are_removed(self):
        logs = self.root / 'Bluetooth'
        (logs / 'errors').mkdir(parents=True)
        for name in ['operation-20261015-120000-123.log', 'errors/20261015-120000-124.txt']:
            (logs / name).write_text('old')
        (logs / 'operation-personal.log').write_text('keep')
        (logs / 'errors/notes.txt').write_text('keep')
        self.assertEqual(self.run_command('list').returncode, 0)
        self.assertFalse((logs / 'operation-20261015-120000-123.log').exists())
        self.assertFalse((logs / 'errors/20261015-120000-124.txt').exists())
        self.assertEqual((logs / 'operation-personal.log').read_text(), 'keep')
        self.assertEqual((logs / 'errors/notes.txt').read_text(), 'keep')
        (self.root / 'Logs').mkdir()
        (self.root / 'Logs/slot.log').write_text('x' * 300000)
        self.assertEqual(self.run_command('diagnose').returncode, 0)
        self.assertLessEqual((logs / 'latest-report.txt').stat().st_size, 131072)
        self.assertFalse(list(self.root.glob('lock.work.*')))

    def test_pairing_waits_for_audio_connect_before_closing_agent(self):
        self.env.update(MOCK_PAIRED='no', MOCK_AUDIO_DELAY='0.12', MOCK_REQUIRE_TRUST='yes')
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / 'audio-ready').exists(), 'must not stop the agent while Connect is pending')
        calls = (self.root / 'calls').read_text().splitlines()
        self.assertLess(calls.index('trust ' + MAC), calls.index('session connect ' + MAC))
        self.assertEqual((self.root / 'route').read_text().strip(), 'bluealsa:DEV=' + MAC + ',PROFILE=a2dp')

    def test_automatic_fallback_never_cancels_a_running_reconnect(self):
        lock = self.root / 'lock'
        lock.mkdir()
        (lock / 'pid').write_text(str(os.getpid()))
        (lock / 'verb').write_text('reconnect')
        route = self.root / 'route'
        route.write_text('bluealsa:DEV=' + MAC + ',PROFILE=a2dp')
        self.env['MOCK_CONNECTED'] = 'no'
        result = self.run_command('fallback', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((lock / 'cancel').exists())
        self.assertEqual(route.read_text(), 'bluealsa:DEV=' + MAC + ',PROFILE=a2dp')
        calls = (self.root / 'calls').read_text() if (self.root / 'calls').exists() else ''
        self.assertNotIn('disconnect ' + MAC, calls)

    def test_existing_adapter_never_runs_vendor_initialization(self):
        Path(self.env['SLOT_BT_INIT']).write_text('#!/bin/sh\necho called > "$SLOT_ROOT/vendor-called"\nexit 1\n')
        for verb, args in [('connect', [MAC]), ('scan', [])]:
            result = self.run_command(verb, *args)
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / 'vendor-called').exists())

    def test_missing_adapter_uses_attach_only_not_low_level_enable(self):
        (self.root / 'hci0').rmdir()
        (self.root / 'bin/pidof').write_text('#!/bin/sh\n[ "$1" != rtk_hciattach ]\n')
        Path(self.env['SLOT_BT_INIT']).write_text('#!/bin/sh\necho "$1" > "$SLOT_ROOT/vendor-called"\n[ "$1" = init ]\n')
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'vendor-called').read_text().strip(), 'init')

    def test_connected_link_without_audio_never_publishes_route(self):
        self.env['MOCK_AUDIO_PCM'] = 'no'
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('audio did not become ready', result.stderr)
        self.assertFalse((self.root / 'route').exists())
        self.assertFalse((self.root / 'Config/bluetooth-headphones').exists())

    def test_audio_pcm_can_arrive_after_bluetooth_connects(self):
        self.env['MOCK_PCM_DELAY'] = '3'
        result = self.run_command('connect', MAC)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'pcm-count').read_text().strip(), '4')
        self.assertTrue((self.root / 'route').exists())

    def test_another_devices_pcm_does_not_replace_the_current_output(self):
        route = self.root / 'route'
        old = 'bluealsa:DEV=00:11:22:33:44:55,PROFILE=a2dp'
        route.write_text(old)
        self.env['MOCK_PCM_DEVICE'] = '00_11_22_33_44_55'
        result = self.run_command('connect', MAC)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(route.read_text(), old)

if __name__ == '__main__':
    unittest.main()
