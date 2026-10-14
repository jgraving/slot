#!/bin/sh
# No secrets in arguments, environment, logs, or persistent supplicant config.
set -eu
umask 077
P="${SLOT_WIFI_SYSROOT:-}"
RUN="$P/run/slot-wifi"
CTRL="$RUN/control"
PRIVATE="$P/data/slot-network"
OP="${1:-}"
fail() { echo "$1" >&2; exit 1; }
[ "$(id -u)" = 0 ] || fail 'Wi-Fi requires the BaseOS frontend'
# /run must be RAM; /data must really be persistent (BaseOS can fall back to tmpfs).
awk -v p="$P/run" '$2 == p && $3 == "tmpfs" {found=1} END {exit !found}' "$P/proc/mounts" || fail 'RAM runtime unavailable'
awk -v p="$P/data" '$2 == p && $3 == "ext4" {found=1} END {exit !found}' "$P/proc/mounts" || fail 'Persistent private storage unavailable'
[ ! -L "$RUN" ] && [ ! -L "$PRIVATE" ] || fail 'Unsafe Wi-Fi directory'
mkdir -p "$RUN" "$PRIVATE"
chmod 700 "$RUN" "$PRIVATE"
mkdir "$RUN/operation" 2>/dev/null || fail 'Wi-Fi is busy'
cli() { wpa_cli -p "$CTRL" -i wlan0 "$@"; }
stop_dhcp() {
    if [ -f "$RUN/dhcp.pid" ]; then
        pid=$(cat "$RUN/dhcp.pid")
        case "$pid" in ''|*[!0-9]*) ;; *)
            # Never kill a recycled PID belonging to another program.
            if [ -r "$P/proc/$pid/comm" ] && [ "$(cat "$P/proc/$pid/comm")" = udhcpc ]; then
                kill "$pid" 2>/dev/null || true
            fi ;;
        esac
        rm -f "$RUN/dhcp.pid"
    fi
}
disconnect() {
    stop_dhcp
    cli terminate >/dev/null 2>&1 || true
    rm -f "$RUN/network.conf"
    ip addr flush dev wlan0 >/dev/null 2>&1 || true
    ip link set wlan0 down >/dev/null 2>&1 || true
}
cleanup() {
    result=$?
    rm -f "$RUN/network.conf"
    if [ "$OP" = connect ] && [ "$result" != 0 ]; then disconnect; fi
    rmdir "$RUN/operation" 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
prepare() {
    # Disable core files globally before accepting or decrypting credentials.
    echo 0 > "$P/proc/sys/fs/suid_dumpable"
    echo /dev/null > "$P/proc/sys/kernel/core_pattern"
    swapoff -a || fail 'Could not disable persistent swap'
    [ "$(wc -l < "$P/proc/swaps")" -le 1 ] || fail 'Swap still active'
    # Dev BaseOS can launch dropbear late. A locked root password also closes that race.
    [ -L "$P/etc/shadow" ] && [ "$(readlink "$P/etc/shadow")" = /run/shadow ] || fail 'Unknown BaseOS login layout'
    awk -F: 'BEGIN {OFS=":"} $1 == "root" {$2="!"} {print}' "$P/run/shadow" > "$RUN/shadow.new"
    chmod 600 "$RUN/shadow.new"
    mv "$RUN/shadow.new" "$P/run/shadow"
    killall dropbear >/dev/null 2>&1 || true
    n=0
    while pidof dropbear >/dev/null 2>&1; do
        n=$((n+1)); [ "$n" -lt 20 ] || fail 'Could not stop password SSH'
        sleep 0.1
    done
}
radio() {
    [ ! -f "$P/run/slotlink.session" ] || fail 'Finish link play before using home Wi-Fi'
    n=0
    while [ ! -d "$P/sys/class/net/wlan0" ]; do
        n=$((n+1)); [ "$n" -lt 25 ] || fail 'Wi-Fi radio unavailable'
        sleep 1
    done
    rfkill unblock wifi
    ip link set wlan0 up
    mkdir -p "$CTRL"
    chmod 700 "$CTRL"
}
status() {
    ip -4 -o addr show dev wlan0 2>/dev/null | awk '{print "IP " $4}'
    cli status 2>/dev/null | sed -n 's/^wpa_state=/STATE /p'
}
case "$OP" in
prepare) prepare ;;
scan)
    prepare
    radio
    if ! cli ping 2>/dev/null | grep -q PONG; then
        systemctl stop wpa_supplicant >/dev/null 2>&1 || fail 'Could not stop old Wi-Fi session'
        printf 'ctrl_interface=%s\nupdate_config=0\n' "$CTRL" > "$RUN/network.conf"
        wpa_supplicant -B -i wlan0 -c "$RUN/network.conf" -Dnl80211 >/dev/null 2>&1 || fail 'Could not start Wi-Fi'
    fi
    cli scan >/dev/null
    sleep 4
    cli scan_results
    ;;
connect)
    prepare
    radio
    disconnect
    systemctl stop wpa_supplicant >/dev/null 2>&1 || fail 'Could not stop old Wi-Fi session'
    radio
    # Rust sends a bounded, hex-encoded WPA2 configuration over stdin.
    cat > "$RUN/network.conf"
    [ "$(wc -c < "$RUN/network.conf")" -le 4096 ] || fail 'Invalid network configuration'
    wpa_supplicant -B -i wlan0 -c "$RUN/network.conf" -Dnl80211 >/dev/null 2>&1 || fail 'Could not start Wi-Fi'
    n=0
    until cli status 2>/dev/null | grep -q '^wpa_state=COMPLETED$'; do
        n=$((n+1)); [ "$n" -lt 25 ] || fail 'Connection failed; check network and password'
        sleep 1
    done
    rm -f "$RUN/network.conf"
    # Keep lease renewal alive; suppress optional mDNS advertising for this connection.
    BASEOS_MDNS_HELPER=/bin/true udhcpc -i wlan0 -n -p "$RUN/dhcp.pid" -t 5 -T 2 >/dev/null 2>&1 || fail 'Could not obtain an IP address'
    status
    ;;
status) status ;;
disconnect) disconnect ;;
*) fail 'Unknown network operation' ;;
esac
