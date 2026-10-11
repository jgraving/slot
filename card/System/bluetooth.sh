#!/bin/sh
# slot Bluetooth v2: a live BlueZ agent, structured results and attachable reports.
set -eu
ROOT=${SLOT_ROOT:-/mnt/sdcard}
ROUTE=${SLOT_AUDIO_ROUTE:-/run/slot-audio-device}
PREF="$ROOT/Config/bluetooth-headphones"
BT=${SLOT_BLUETOOTHCTL:-bluetoothctl}
LOCK=${SLOT_BT_LOCK:-/run/slot-bluetooth.lock}
LOGDIR="$ROOT/Bluetooth"
VERB=${1:-}
TARGET=${2:-}
STAGE=starting
ERROR=''
SESSION_PID=''
SESSION_OPEN=0
mkdir -p "$LOGDIR"
SESSION_LOG="$LOGDIR/operation-$(date -u '+%Y%m%d-%H%M%S')-$$.log"
: > "$SESSION_LOG"
log() { printf '%s %s\n' "$(date -u '+%H:%M:%S')" "$*" >> "$SESSION_LOG"; }
valid_mac() { printf '%s\n' "$1" | LC_ALL=C grep -Eq '^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$'; }
raw_name() { printf '%s\n' "$1" | LC_ALL=C grep -Eq '^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$|^[0-9A-Fa-f]{12}$'; }
fail() { ERROR=$*; printf '%s\n' "$ERROR" >&2; exit 1; }
# bluetoothctl exit status alone is unreliable: the properties are checked below.
bt() {
    log "bluetoothctl $*"
    rc=0
    timeout 6 "$BT" "$@" > "$LOCK/command-output" 2>&1 || rc=$?
    cat "$LOCK/command-output" >> "$SESSION_LOG"
    cat "$LOCK/command-output"
    log "exit=$rc"
    return "$rc"
}
route() {
    mkdir -p "$(dirname "$ROUTE")"
    printf '%s\n' "$1" > "$ROUTE.$$"
    mv -f "$ROUTE.$$" "$ROUTE"
}
stop_session() {
    if [ "$SESSION_OPEN" = 1 ]; then
        printf 'scan off\nquit\n' >&3 2>/dev/null || true
        exec 3>&-
        SESSION_OPEN=0
    fi
    if [ -n "$SESSION_PID" ]; then
        kill "$SESSION_PID" 2>/dev/null || true
        wait "$SESSION_PID" 2>/dev/null || true
        SESSION_PID=''
    fi
    if [ -f "$LOCK/session-output" ]; then
        printf '\n--- Live pairing session ---\n' >> "$SESSION_LOG"
        cat "$LOCK/session-output" >> "$SESSION_LOG"
    fi
}
report() {
    kind=$1
    file="$LOGDIR/latest-$kind.txt"
    tmp="$file.$$"
    {
        echo 'slot Bluetooth diagnostic report — v2'
        date -u '+UTC: %Y-%m-%d %H:%M:%S'
        echo "Operation: $VERB"
        echo "Stage: $STAGE"
        echo "Target: $TARGET"
        echo "Error: ${ERROR:-none}"
        echo
        echo '--- Kernel / OS / build ---'
        uname -a
        for f in /etc/os-release /etc/baseos-release "$ROOT/System/bluetooth-build.txt"; do
            [ ! -r "$f" ] || cat "$f"
        done
        timeout 3 "$BT" --version 2>&1 || true
        timeout 3 bluealsa --version 2>&1 || true
        echo
        echo '--- Services ---'
        for service in dbus-daemon bluetoothd bluealsa rtk_hciattach; do
            printf '%s: ' "$service"
            pidof "$service" 2>/dev/null || true
            echo
        done
        echo '--- Adapter ---'
        timeout 3 "$BT" show 2>&1 || true
        timeout 3 hciconfig -a 2>&1 || true
        echo '--- All discovered devices (unfiltered) ---'
        timeout 3 "$BT" devices 2>&1 || true
        if valid_mac "$TARGET"; then
            echo '--- Selected device ---'
            timeout 3 "$BT" info "$TARGET" 2>&1 || true
        fi
        echo '--- Audio route / saved preference ---'
        [ ! -r "$ROUTE" ] || cat "$ROUTE"
        [ ! -r "$PREF" ] || cat "$PREF"
        echo '--- ALSA outputs ---'
        timeout 3 aplay -L 2>&1 || true
        echo '--- Operation transcript ---'
        cat "$SESSION_LOG"
        echo '--- Installed emulator files ---'
        ls -l "$ROOT/System/"*libretro.so 2>&1 || true
        echo '--- slot audio / application log (last 120 lines) ---'
        [ ! -r "$ROOT/slot.log" ] || tail -n 120 "$ROOT/slot.log"
    } > "$tmp" 2>&1
    mv -f "$tmp" "$file"
    if [ "$kind" = error ]; then
        mkdir -p "$LOGDIR/errors"
        cp "$file" "$LOGDIR/errors/$(date -u '+%Y%m%d-%H%M%S')-$$.txt"
    fi
    printf 'REPORT Bluetooth/latest-%s.txt\n' "$kind"
}
cleanup() {
    rc=$?
    trap - 0 1 2 15
    stop_session
    if [ "$rc" -ne 0 ]; then
        [ -n "$ERROR" ] || ERROR="Bluetooth failed at $STAGE (exit $rc)"
        report error >&2 || true
    fi
    rm -rf "$LOCK"
    exit "$rc"
}
start() {
    STAGE='adapter startup'
    command -v "$BT" >/dev/null || fail 'Bluetooth tools are missing'
    command -v bluealsa >/dev/null || fail 'BlueALSA is missing'
    rfkill unblock bluetooth >> "$SESSION_LOG" 2>&1 || true
    if [ ! -d "${SLOT_HCI_PATH:-/sys/class/bluetooth/hci0}" ]; then
        if pidof rtk_hciattach >/dev/null 2>&1; then killall rtk_hciattach 2>/dev/null || true; fi
        rm -f /tmp/.init_bt
    fi
    sh "${SLOT_BT_INIT:-/mnt/vendor/ctrl/setBluetooth.sh}" all >> "$SESSION_LOG" 2>&1
    systemctl start bluetooth >> "$SESSION_LOG" 2>&1
    ready=0
    while ! bt show | grep -q '^Controller '; do
        ready=$((ready + 1))
        [ "$ready" -lt 5 ] || fail 'Bluetooth adapter did not appear'
        sleep 1
    done
    bt power on >/dev/null
    bt show | grep -q 'Powered: yes' || fail 'Bluetooth adapter did not power on'
    bt pairable on >/dev/null
    if ! pidof bluealsa >/dev/null 2>&1; then
        bluealsa -p a2dp-source --initial-volume=100 < /dev/null >> "$SESSION_LOG" 2>&1 &
        n=0
        while ! pidof bluealsa >/dev/null 2>&1 && [ "$n" -lt 20 ]; do sleep 0.1; n=$((n + 1)); done
        pidof bluealsa >/dev/null 2>&1 || fail 'BlueALSA did not start'
    fi
}
# No command argument: BlueZ 5.53 only registers --agent in interactive mode.
# A FIFO keeps stdin open, and every asynchronous transition is acknowledged.
send() { log "session: $*"; printf '%s\n' "$*" >&3; }
wait_text() {
    expected=$1
    limit=$2
    n=0
    while ! grep -q "$expected" "$LOCK/session-output"; do
        kill -0 "$SESSION_PID" 2>/dev/null || fail 'Bluetooth pairing session stopped'
        if grep -q 'Failed to register agent\|Failed to request default agent' "$LOCK/session-output"; then
            fail 'BlueZ could not register the pairing agent'
        fi
        n=$((n + 1))
        [ "$n" -lt "$limit" ] || fail "Bluetooth timed out at $STAGE"
        sleep "${SLOT_BT_POLL_INTERVAL:-1}"
    done
}
start_session() {
    STAGE='pairing agent registration'
    mkfifo "$LOCK/input"
    : > "$LOCK/session-output"
    "$BT" --agent NoInputNoOutput < "$LOCK/input" > "$LOCK/session-output" 2>&1 &
    SESSION_PID=$!
    exec 3> "$LOCK/input"
    SESSION_OPEN=1
    wait_text 'Agent registered' 12
    send default-agent
    wait_text 'Default agent request successful' 10
    STAGE='headphone discovery'
    send 'menu scan'
    send 'transport bredr'
    send back
    send 'scan on'
    wait_text 'Discovery started' 10
}
wait_property() {
    property=$1
    limit=$2
    n=0
    pin_sent=0
    auth_sent=0
    while :; do
        info=$(bt info "$TARGET") || true
        printf '%s\n' "$info" | grep -q "$property: yes" && return 0
        if [ -n "$SESSION_PID" ]; then
            if grep -q 'Failed to pair:' "$LOCK/session-output"; then
                reason=$(sed -n 's/.*Failed to pair: //p' "$LOCK/session-output" | tail -n 1)
                fail "Pairing failed: $reason"
            fi
            # Sony documents 0000 if these headphones request a legacy PIN.
            if [ "$pin_sent" = 0 ] && grep -q 'Enter PIN code\|Enter passkey' "$LOCK/session-output"; then
                send 0000
                pin_sent=1
            fi
            if [ "$auth_sent" = 0 ] && grep -q 'Authorize service\|Authorize pairing' "$LOCK/session-output"; then
                send yes
                auth_sent=1
            fi
        fi
        n=$((n + 1))
        [ "$n" -lt "$limit" ] || fail "Timed out waiting for $property"
        sleep "${SLOT_BT_POLL_INTERVAL:-1}"
    done
}
emit_list() {
    STAGE='device list'
    all=$(bt devices) || fail 'Could not read Bluetooth devices'
    printf '%s\n' "$all" | while read -r kind address name; do
        [ "$kind" = Device ] || continue
        valid_mac "$address" || continue
        [ -n "$name" ] || continue
        raw_name "$name" && continue
        printf 'DEVICE %s %s\n' "$address" "$name"
        info=$(bt info "$address") || true
        paired=no
        connected=no
        printf '%s\n' "$info" | grep -q 'Paired: yes' && paired=yes
        printf '%s\n' "$info" | grep -q 'Connected: yes' && connected=yes
        printf 'STATE %s %s %s\n' "$address" "$paired" "$connected"
    done
    selected=''
    [ ! -r "$ROUTE" ] || selected=$(sed -n 's/^bluealsa:DEV=\([^,]*\),PROFILE=a2dp$/\1/p' "$ROUTE")
    printf 'OUTPUT %s\n' "${selected:-speaker}"
}
connect() {
    valid_mac "$TARGET" || fail 'Invalid Bluetooth address'
    start
    info=$(bt info "$TARGET") || true
    if ! printf '%s\n' "$info" | grep -q 'Paired: yes'; then
        start_session
        STAGE=pairing
        send "pair $TARGET"
        wait_property Paired 45
        send 'scan off'
    fi
    STAGE='audio connection'
    bt trust "$TARGET" >/dev/null
    if [ -n "$SESSION_PID" ]; then send "connect $TARGET"; else bt connect "$TARGET" >/dev/null || true; fi
    wait_property Connected 20
    STAGE='audio service discovery'
    n=0
    while :; do
        info=$(bt info "$TARGET") || true
        printf '%s\n' "$info" | grep -qi '0000110b-0000-1000-8000-00805f9b34fb' && break
        n=$((n + 1))
        [ "$n" -lt 10 ] || fail 'This device does not offer headphone audio (A2DP)'
        sleep "${SLOT_BT_POLL_INTERVAL:-1}"
    done
    mkdir -p "$ROOT/Config"
    printf '%s\n' "$TARGET" > "$PREF.$$"
    mv -f "$PREF.$$" "$PREF"
    route "bluealsa:DEV=$TARGET,PROFILE=a2dp"
    stop_session
}
[ "$VERB" = restore ] && [ ! -f "$PREF" ] && { rm -f "$SESSION_LOG"; exit 0; }
# Invalid input is rejected before making any Bluetooth request.
case "$VERB" in connect|forget) valid_mac "$TARGET" || { echo 'Invalid Bluetooth address' >&2; exit 1; };; esac
tries=0
while ! mkdir "$LOCK" 2>/dev/null; do
    if [ -r "$LOCK/pid" ] && ! kill -0 "$(cat "$LOCK/pid")" 2>/dev/null; then rm -rf "$LOCK"; continue; fi
    tries=$((tries + 1))
    [ "$tries" -lt 10 ] || { echo 'Bluetooth is busy; try again' >&2; exit 1; }
    sleep 1
done
printf '%s\n' "$$" > "$LOCK/pid"
trap cleanup 0
trap 'ERROR="Bluetooth operation interrupted"; exit 1' 1 2 15
log "slot Bluetooth v2: verb=$VERB target=$TARGET"
case "$VERB" in
    list) start; emit_list ;;
    scan)
        start
        start_session
        sleep "${SLOT_BT_SCAN_SECONDS:-12}"
        emit_list
        stop_session
        ;;
    connect) connect; emit_list ;;
    speaker)
        STAGE='speaker selection'
        route ''
        if [ -f "$PREF" ]; then
            TARGET=$(cat "$PREF")
            if valid_mac "$TARGET"; then bt disconnect "$TARGET" >/dev/null || true; fi
        fi
        rm -f "$PREF"
        emit_list
        ;;
    forget)
        STAGE='forget pairing'
        if [ -f "$PREF" ] && [ "$(cat "$PREF")" = "$TARGET" ]; then route ''; rm -f "$PREF"; fi
        bt disconnect "$TARGET" >/dev/null || true
        bt remove "$TARGET" >/dev/null
        emit_list
        ;;
    diagnose) report report ;;
    restore) TARGET=$(cat "$PREF"); connect ;;
    *) fail 'Unknown Bluetooth operation' ;;
esac
