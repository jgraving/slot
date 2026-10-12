#!/bin/sh
# Secret input is stdin only. Never run with shell tracing or log command output.
set -eu
umask 077
RUN=/run/slot-wifi
CTRL="$RUN/control"
ROOT="${SLOT_ROOT:-/mnt/sdcard}"
fail() { echo "$1" >&2; exit 1; }
stop_ssh() {
    killall dropbear 2>/dev/null || true
    n=0
    while pidof dropbear >/dev/null 2>&1; do
        n=$((n+1)); [ "$n" -lt 20 ] || fail 'Could not disable password SSH'
        sleep 0.1
    done
}
prepare() {
    # Core dumps could otherwise copy keys from process memory onto the games card.
    echo 0 > /proc/sys/fs/suid_dumpable
    echo /dev/null > /proc/sys/kernel/core_pattern
    # Prevent credentials from being swapped onto persistent storage.
    swapoff -a || fail "Could not disable swap; Wi-Fi remains unavailable"
    [ "$(wc -l < /proc/swaps)" -le 1 ] || fail "Swap is still active"
    stop_ssh
    # Block password login even if BaseOS dev startup launches its daemon late.
    [ -L /etc/shadow ] && [ "$(readlink /etc/shadow)" = /run/shadow ] || fail "Unsupported SSH credential layout"
    awk -F: 'BEGIN { OFS=":" } $1 == "root" { $2="!" } { print }' /run/shadow > "$RUN-shadow.new"
    chmod 600 "$RUN-shadow.new"
    mv "$RUN-shadow.new" /run/shadow
    mkdir -p "$CTRL"
    chmod 700 "$RUN" "$CTRL"
}
cli() { wpa_cli -p "$CTRL" -i wlan0 "$@"; }
status() {
    ip -4 -o addr show dev wlan0 2>/dev/null | awk '{print "IP " $4}'
    cli status 2>/dev/null | sed -n 's/^wpa_state=/STATE /p'
}
disconnect() {
    if [ -f "$RUN/dhcp.pid" ]; then
        kill "$(cat "$RUN/dhcp.pid")" 2>/dev/null || true
        rm -f "$RUN/dhcp.pid"
    fi
    cli terminate >/dev/null 2>&1 || true
    rm -f "$RUN/network.conf"
    ip addr flush dev wlan0 2>/dev/null || true
}
case "${1:-}" in
prepare) prepare ;;
scan)
    [ ! -f /run/slotlink.session ] || fail "Finish link play before using home Wi-Fi"
    prepare
    [ -d /sys/class/net/wlan0 ] || fail 'Wi-Fi radio unavailable; try again after boot'
    rfkill unblock wifi
    ip link set wlan0 up
    if ! cli ping 2>/dev/null | grep -q PONG; then
        command -v wpa_supplicant >/dev/null || fail 'Wi-Fi tools unavailable'
        systemctl stop wpa_supplicant >/dev/null 2>&1 || true
        printf 'ctrl_interface=%s\nupdate_config=0\n' "$CTRL" > "$RUN/network.conf"
        wpa_supplicant -B -i wlan0 -c "$RUN/network.conf" -Dnl80211 >/dev/null 2>&1 || fail 'Could not start Wi-Fi'
    fi
    cli scan >/dev/null
    sleep 4
    cli scan_results
    ;;
connect)
    [ ! -f /run/slotlink.session ] || fail "Finish link play before using home Wi-Fi"
    prepare
    disconnect
    systemctl stop wpa_supplicant >/dev/null 2>&1 || true
    rfkill unblock wifi
    ip link set wlan0 up
    trap 'rm -f "$RUN/network.conf"' EXIT HUP INT TERM
    cat > "$RUN/network.conf"
    wpa_supplicant -B -i wlan0 -c "$RUN/network.conf" -Dnl80211 >/dev/null 2>&1 || fail 'Could not start Wi-Fi'
    n=0
    until cli status 2>/dev/null | grep -q '^wpa_state=COMPLETED$'; do
        n=$((n+1)); [ "$n" -lt 25 ] || { disconnect; fail 'Connection failed; check network and password'; }
        sleep 1
    done
    # Config remains only in RAM and is unlinked after the daemon has read it.
    rm -f "$RUN/network.conf"
    udhcpc -i wlan0 -n -p "$RUN/dhcp.pid" -t 5 -T 2 >/dev/null 2>&1 || { disconnect; fail 'Could not obtain an IP address'; }
    status
    ;;
status) status ;;
disconnect) disconnect; stop_ssh ;;
lock) disconnect; stop_ssh ;;
*) fail 'Unknown network operation' ;;
esac
