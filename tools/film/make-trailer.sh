#!/usr/bin/env bash
# The trailer of Star Racer (docs/film.md#the-trailer): 15 seconds at 1920x1080 and 30 frames a
# second, cut to its own acid trance (tools/film/trance.rs), announcing the release on
# 1 January 2027. Two steps, so the cut can be reworked without filming again:
#
#     bash tools/film/make-trailer.sh record   # films the takes against a local server (minutes)
#     bash tools/film/make-trailer.sh cut      # the music and the cut, from the takes
#
# Out: target/trailer/star-racer-trailer.mp4. Takes: target/trailer/{a,b}.mp4, a the racer, b a spectator.
set -euo pipefail

# The site's address written under the title: given when the film is made (SITE=...), never
# stored here; a space when it is not.
SITE=${SITE:- }
FF=${FFMPEG:-ffmpeg}
OUT=target/trailer
CLIENT=target/debug/star-racer-client
SERVER=target/debug/star-racer-server
PORT=8093
mkdir -p "$OUT"

# Each take's frames as a 30 fps video, every frame shown until the next one's time.
to_video() {
    local take=$1
    ls "$OUT/$take" | sort | awk -v d="$take" '
        { t = substr($0, 7, 8) + 0; if (prev != "") printf "file %s/%s\nduration %.4f\n", d, prev, (t - pt) / 1000.0; prev = $0; pt = t }
        END { printf "file %s/%s\nduration 0.0333\nfile %s/%s\n", d, prev, d, prev }' > "$OUT/$take.txt"
    (cd "$OUT" && "$FF" -y -loglevel error -f concat -safe 0 -i "$take.txt" \
        -vf "fps=30,format=yuv420p" -c:v libx264 -preset fast -crf 14 "$take.mp4")
    rm -rf "${OUT:?}/$take" "$OUT/$take.txt"
}

record() {
    rm -rf "$OUT/a" "$OUT/b" "$OUT/user-tracks"
    "$SERVER" --port $PORT --user-tracks "$OUT/user-tracks" --ratings "$OUT/ratings.ron" \
        --progress "$OUT/progress.ron" --accounts "$OUT/accounts.ron" > "$OUT/server.log" 2>&1 &
    local server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    sleep 3
    local common=(--server "127.0.0.1:$PORT" --guest --volume 0)

    # a: the racer, with the race's interface, as a player sees it: the trailer shows it.
    # a: the racer, from the grid of a lobby of its own filled with fifteen bots, alone on the
    # machine (a second game client beside it slowed both, and the autopilot, a few frames late,
    # found the walls), at 1600x900 so the debug client keeps its pace while it records.
    "$CLIENT" "${common[@]}" --identity "$OUT/id-a" --lobby "Grand Prix" --track cascade --laps 1 \
        --min-players 1 --bots 15 --autopilot-drift --window-size 1600x900 \
        --record "$OUT/a" --record-fps 30 --record-seconds 95

    # b: a spectator, its camera changing on its own, over a race a small client holds with
    # fifteen bots and drives, out of sight.
    "$CLIENT" "${common[@]}" --identity "$OUT/id-host" --lobby "Watch" --track cascade --laps 3 \
        --min-players 1 --bots 15 --autopilot-drift --window-size 320x180 > /dev/null 2>&1 &
    local host=$!
    sleep 8
    "$CLIENT" "${common[@]}" --identity "$OUT/id-b" --watch "Watch" --camera orbit \
        --camera-cycle 6 --window-size 1920x1080 --record "$OUT/b" --record-fps 30 --record-seconds 100
    kill $host 2>/dev/null || true
    kill $server 2>/dev/null || true
    trap - EXIT
    for take in a b; do to_video $take; done
    ls -la "$OUT"/*.mp4
}

cut() {
    rustc -O tools/film/trance.rs -o "$OUT/trance.exe"
    "$OUT/trance.exe" "$OUT/trance.wav"
    cp client/assets/fonts/Saira-Italic.ttf "$OUT/saira.ttf"
    # A half bar of 140 BPM.
    local half=0.857143
    calc() { awk "BEGIN { printf \"%.4f\", $1 }"; }
    local filters="" inputs="" n=0 t=0 captions=""
    # tools/film/trailer-cuts.txt: take, start in seconds, length in half bars, and a caption shown
    # over the shot (underscores for spaces, - for none).
    while read -r take start halves caption; do
        [ -z "$take" ] && continue
        case "$take" in \#*) continue ;; esac
        local len; len=$(calc "$halves * $half")
        filters="${filters}[$n:v]trim=start=0:duration=${len},setpts=PTS-STARTPTS,fps=30,scale=1920:1080,format=yuv420p[v$n];"
        inputs="$inputs -ss $start -t $(calc "$len + 0.5") -i $take.mp4"
        if [ "$caption" != "-" ]; then
            local from; from=$(calc "$t + 0.08")
            local to; to=$(calc "$t + $len - 0.05")
            local text=${caption//_/ }
            captions="${captions}drawtext=fontfile=saira.ttf:text='${text}':fontsize=118:fontcolor=white:borderw=3:bordercolor=white:shadowcolor=0x1E9BFF:shadowx=7:shadowy=7:x=(w-text_w)/2:y=h*0.16:enable='between(t,$from,$to)':alpha='min(1,(t-$from)/0.08)',"
        fi
        t=$(calc "$t + $len")
        n=$((n + 1))
    done < tools/film/trailer-cuts.txt
    local concat=""; for i in $(seq 0 $((n - 1))); do concat="${concat}[v$i]"; done
    local total=15.0
    # The drop, 2 bars in: a white flash fading over a beat.
    local drop; drop=$(calc "4 * $half")
    # The title over the last 3 seconds.
    local title; title=$(calc "$total - 3.0")
    local date; date=$(calc "$title + 0.43")
    local url; url=$(calc "$title + 0.86")
    local fade; fade=$(calc "$total - 0.5")
    local graph="${filters}${concat}concat=n=$n:v=1:a=0[cat];\
[cat]${captions}null[pre];\
color=c=white:s=1920x1080:r=30:d=0.45,format=rgba,fade=t=out:st=0:d=0.45:alpha=1,setpts=PTS+$drop/TB[flash];\
[pre][flash]overlay=eof_action=pass:format=auto,\
drawbox=x=0:y=ih/2-260:w=iw:h=520:color=black@0.66:t=fill:enable='gte(t,$title)',\
drawtext=fontfile=saira.ttf:text='STAR RACER':fontsize=200:fontcolor=white:borderw=4:bordercolor=white:shadowcolor=0x1E9BFF:shadowx=10:shadowy=10:x=(w-text_w)/2:y=(h/2)-215:alpha='if(lt(t,$title),0,min(1,(t-$title)/0.25))',\
drawtext=fontfile=saira.ttf:text='JANUARY 1, 2027':fontsize=96:fontcolor=0xFF4FD8:borderw=2:bordercolor=0xFF4FD8:shadowcolor=0x000000:shadowx=4:shadowy=4:x=(w-text_w)/2:y=(h/2)+20:alpha='if(lt(t,$date),0,min(1,(t-$date)/0.2))',\
drawtext=fontfile=saira.ttf:text='${SITE}':fontsize=56:fontcolor=0x9FD8FF:borderw=1:bordercolor=0x9FD8FF:x=(w-text_w)/2:y=(h/2)+150:alpha='if(lt(t,$url),0,min(1,(t-$url)/0.25))',\
fade=t=in:st=0:d=0.3,fade=t=out:st=$fade:d=0.5[v];\
[$n:a]atrim=0:$total,asetpts=PTS-STARTPTS,afade=t=out:st=$(calc "$total - 1.2"):d=1.2[a]"
    cd "$OUT"
    # shellcheck disable=SC2086
    "$FF" -y -loglevel error $inputs -i trance.wav -filter_complex "$graph" -map "[v]" -map "[a]" \
        -c:v libx264 -preset slow -crf 17 -pix_fmt yuv420p -r 30 -c:a aac -b:a 192k -t $total \
        -movflags +faststart star-racer-trailer.mp4
    echo "$OUT/star-racer-trailer.mp4"
}

# The site's background (site/index.html): a racer among fifteen bots on Cascade with no HUD at all
# (--clean), cut into a silent loop of 16 s, light enough to start at once.
loop() {
    rm -rf "$OUT/c"
    "$SERVER" --port $PORT --user-tracks "$OUT/user-tracks" --ratings "$OUT/ratings.ron" \
        --progress "$OUT/progress.ron" --accounts "$OUT/accounts.ron" > "$OUT/server.log" 2>&1 &
    local server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    sleep 3
    "$CLIENT" --server "127.0.0.1:$PORT" --guest --volume 0 --clean --identity "$OUT/id-c" \
        --lobby "Loop" --track cascade --laps 1 --min-players 1 --bots 15 --autopilot-drift \
        --window-size 1600x900 --record "$OUT/c" --record-fps 30 --record-seconds 70
    kill $server 2>/dev/null || true
    trap - EXIT
    to_video c
    # Four stretches of the lap: the hairpins, the summit leap, the cascade, the canyon.
    local parts="" n=0
    for start in 20 40 50 60; do
        parts="$parts -ss $start -t 4 -i c.mp4"
        n=$((n + 1))
    done
    local graph="" concat=""
    for i in $(seq 0 $((n - 1))); do
        graph="${graph}[$i:v]setpts=PTS-STARTPTS,scale=1280:720,fps=30[v$i];"
        concat="${concat}[v$i]"
    done
    # shellcheck disable=SC2086
    (cd "$OUT" && "$FF" -y -loglevel error $parts -filter_complex "${graph}${concat}concat=n=$n:v=1:a=0[v]" \
        -map "[v]" -an -c:v libx264 -preset slow -crf 27 -pix_fmt yuv420p -movflags +faststart loop.mp4)
    ls -la "$OUT/loop.mp4"
}

case "${1:-}" in
    record) record ;;
    cut) cut ;;
    loop) loop ;;
    *) echo "usage: $0 record|cut|loop" >&2; exit 2 ;;
esac
