#!/bin/sh
# BaseOS runtime policy: Bluetooth audio and explicit local link sessions only.
set -eu
P=${SLOT_OFFLINE_SYSROOT:-}
RUN="$P/run"
NET="$P/sys/class/net"
mkdir -p "$RUN"
stop_services() {
    for service in dropbear sshd smbd nmbd vsftpd proftpd pure-ftpd ftpd telnetd inetd udhcpc dhcpcd connmand NetworkManager avahi-daemon ntpd adbd; do
        killall "$service" >/dev/null 2>&1 || true
    done
}
radio_off() {
    rfkill block wifi >/dev/null 2>&1 || true
    killall wpa_supplicant hostapd >/dev/null 2>&1 || true
    for path in "$NET"/*; do
        [ -d "$path" ] || continue
        dev=${path##*/}
        [ "$dev" = lo ] && continue
        ip addr flush dev "$dev" >/dev/null 2>&1 || true
        ip link set "$dev" down >/dev/null 2>&1 || true
    done
}
active_link() {
    [ -r "$RUN/slotlink.session" ] || return 1
    read -r owner < "$RUN/slotlink.session"
    case "$owner" in ''|*[!0-9]*) return 1 ;; esac
    [ -d "$P/proc/$owner" ]
}
enforce() {
    stop_services
    if ! active_link; then
        radio_off
        rm -f "$RUN/slotlink.session"
    fi
    # No internet route, even during local play. IPv6 is disabled below.
    ip route flush default >/dev/null 2>&1 || true
}
case "${1:-}" in
start)
    rm -f "$RUN/slot-offline-ready" "$RUN/slotlink.session"
    radio_off
    # Stop dev services from starting late after rcS hands off to the frontend.
    # Bind mounts are RAM-only and leave the BaseOS image untouched.
    failed=0
    for binary in /etc/init.d/dev /usr/sbin/dropbearmulti /usr/sbin/dropbear /usr/sbin/sshd /usr/sbin/smbd /usr/sbin/nmbd /usr/sbin/vsftpd /usr/sbin/inetd /usr/bin/adbd /usr/sbin/adbd; do
        target="$P$binary"
        [ -e "$target" ] || continue
        mount -o bind "$P/bin/false" "$target" || failed=1
    done
    stop_services
    for setting in "$P/proc/sys/net/ipv6/conf"/*/disable_ipv6; do
        [ ! -f "$setting" ] || echo 1 > "$setting"
    done
    [ ! -f "$P/proc/sys/net/ipv4/ip_forward" ] || echo 0 > "$P/proc/sys/net/ipv4/ip_forward"
    mkdir -p "$P/data"
    : > "$P/data/no-adb"
    # Remove only Slot-owned retired credentials; never follow a directory symlink.
    if [ ! -L "$P/data/slot-network" ]; then
        rm -rf "$P/data/slot-network"
    else
        rm -f "$P/data/slot-network"
    fi
    rm -rf "$RUN/slot-wifi"
    [ "$failed" = 0 ] || { echo 'Offline policy could not disable a BaseOS service; link play remains blocked' >&2; exit 1; }
    : > "$RUN/slot-offline-ready"
    ;;
once) enforce ;;
watch)
    while :; do enforce; sleep 2; done
    ;;
*) exit 2 ;;
esac
