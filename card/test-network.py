"""Run network helper only against a fake OS tree and fake radio commands."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

MOCK = '''#!/usr/bin/env python3
import os, sys
from pathlib import Path
p=Path(os.environ['SLOT_WIFI_SYSROOT'])
name=Path(sys.argv[0]).name
args=sys.argv[1:]
with (p/'commands').open('a') as f: f.write(name+' '+ ' '.join(args)+'\\n')
active=p/'active'
if name=='id': print('0'); sys.exit(0)
if name=='pidof': sys.exit(1)
if name=='swapoff' and os.environ.get('FAIL_SWAP'): sys.exit(1)
if name=='systemctl' and os.environ.get('FAIL_STOP'): sys.exit(1)
if name=='wpa_supplicant': active.touch()
if name=='wpa_cli':
 op=args[4]
 if op=='ping': print('PONG' if active.exists() else 'FAIL')
 if op=='terminate': active.unlink(missing_ok=True)
 if op=='status': print('wpa_state='+('DISCONNECTED' if os.environ.get('FAIL_ASSOC') else 'COMPLETED'))
 if op=='scan_results': print('00:11\\t2412\\t-40\\t[WPA2-PSK-CCMP][ESS]\\tHome')
if name=='ip' and '-4' in args: print('3: wlan0 inet 192.168.1.5/24 scope global wlan0')
if name=='udhcpc' and os.environ.get('FAIL_DHCP'): sys.exit(1)
'''

class NetworkTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
  self.root=Path(self.temp.name).resolve()
  for d in ['run','data','etc','proc/sys/fs','proc/sys/kernel','sys/class/net/wlan0','bin']:
   (self.root/d).mkdir(parents=True,exist_ok=True)
  (self.root/'proc/mounts').write_text(f'tmpfs {self.root}/run tmpfs rw 0 0\n/dev/mmcblk0p6 {self.root}/data ext4 rw 0 0\n')
  (self.root/'proc/swaps').write_text('Filename Type Size Used Priority\n')
  (self.root/'run/shadow').write_text('root:default:0:0:99999:7:::\n')
  (self.root/'etc/shadow').symlink_to('/run/shadow')
  for name in ['id','wpa_cli','wpa_supplicant','udhcpc','ip','rfkill','systemctl','swapoff','pidof','killall','sleep']:
   script=self.root/'bin'/name; script.write_text(MOCK); script.chmod(0o755)
  self.env=dict(os.environ,SLOT_WIFI_SYSROOT=str(self.root),PATH=str(self.root/'bin')+':'+os.environ['PATH'])
 def run_helper(self, op, **env):
  return subprocess.run(['sh',str(Path(__file__).parent/'System/network.sh'),op],
   env=dict(self.env,**env),input='ssid=1234\npsk=sentinel-secret\n',text=True,capture_output=True)
 def test_connect_protects_login_and_removes_plaintext_configuration(self):
  r=self.run_helper('connect'); self.assertEqual(r.returncode,0,r.stderr)
  self.assertIn('IP 192.168.1.5/24',r.stdout)
  self.assertTrue((self.root/'run/shadow').read_text().startswith('root:!:'))
  self.assertFalse((self.root/'run/slot-wifi/network.conf').exists())
  self.assertFalse((self.root/'run/slot-wifi/operation').exists())
  commands=(self.root/'commands').read_text()
  self.assertNotIn('sentinel-secret',commands+r.stdout+r.stderr)
  self.assertNotIn(' -q ',commands)
 def test_association_and_dhcp_failures_disconnect_and_clean_up(self):
  for option in ['FAIL_ASSOC','FAIL_DHCP','FAIL_STOP']:
   r=self.run_helper('connect',**{option:'1'}); self.assertNotEqual(r.returncode,0)
   self.assertFalse((self.root/'active').exists())
   self.assertFalse((self.root/'run/slot-wifi/network.conf').exists())
   self.assertFalse((self.root/'run/slot-wifi/operation').exists())
 def test_swap_failure_prevents_connection(self):
  self.assertNotEqual(self.run_helper('connect',FAIL_SWAP='1').returncode,0)
  self.assertNotIn('wpa_supplicant -B',(self.root/'commands').read_text())
 def test_nonpersistent_storage_is_rejected(self):
  p=self.root/'proc/mounts'; p.write_text(p.read_text().replace(' ext4 ',' tmpfs '))
  self.assertNotEqual(self.run_helper('connect').returncode,0)
  self.assertFalse((self.root/'commands').exists())
 def test_link_play_and_parallel_operation_block_connection(self):
  (self.root/'run/slotlink.session').touch()
  self.assertNotEqual(self.run_helper('connect').returncode,0)
  self.assertNotIn('wpa_supplicant -B',(self.root/'commands').read_text())
  (self.root/'run/slotlink.session').unlink()
  (self.root/'run/slot-wifi/operation').mkdir()
  self.assertNotEqual(self.run_helper('scan').returncode,0)
 def test_scan_and_off_do_not_delete_saved_profile(self):
  self.assertEqual(self.run_helper('scan').returncode,0)
  saved=self.root/'data/slot-network/wifi.vault'; saved.write_bytes(b'ciphertext')
  self.assertEqual(self.run_helper('disconnect').returncode,0)
  self.assertEqual(saved.read_bytes(),b'ciphertext')

if __name__=='__main__': unittest.main()
