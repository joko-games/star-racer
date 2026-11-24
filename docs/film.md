# The film

In v0.1.71 and v0.1.72, at the user's word ("il doit tout y avoir pour que tu puisses prendre un
enregistrement depuis le menu d'accueil jusqu'au moment où tu fais quickplay et tu joues [...] juste
assez de films pour faire un montage sympa après, que tu feras. Tu te serviras de l'observateur mais
pas que pour le film. Monte le film avec de la musique techno acid que tu auras fait le plus
simplement possible"), the game can film itself, and `tools/film/make-film.sh` makes a film of a
minute and twenty-two seconds out of it: `target/film/star-racer-film.mp4`, 1280×720 at 30 frames a
second, with its own acid techno.

## Recording what the game draws

`--record <dir>` (`client/src/record.rs`, native only) saves the window's image as the game renders
it, up to `--record-fps` (30) times a second for `--record-seconds`, then quits. Only the game's own
image is saved: a first try filmed the window from outside with ffmpeg's `gdigrab`, which grabs the
screen where the window stands and got whatever covered it, another program, and was thrown away.

Each frame is named by the milliseconds since the recording began, and encoded to PNG on four
threads of its own so the game keeps its pace. The debug client manages about 23 frames a second
while recording; a frame it was too busy to take leaves a gap in time rather than speeding the film
up, since the frames are put together by their times (ffmpeg's concat demuxer, each frame lasting
until the next).

## The takes

Two takes, against a local server started afresh:

- **A**, the player: the client starts on the first screen with a fresh identity, as a guest under
  a generated name, and presses QUICK PLAY itself after 4 s (`--press-quick-play 4`, the real button
  through its `Activate`); nobody else comes, so the pool of bots fills the race (see
  [Lobbies](lobbies.md#bots)), and the drifting autopilot drives it. 130 s.
- **B**, the spectator: a second client races in another quick play lobby, and a third watches it
  (`--watch "Quick Play 1"`), its camera changing every 9 s on its own (`--camera orbit
  --camera-cycle 9`) between ORBIT, TOP and CHASE. 85 s.

The spectator served the film, and the work too: it is how the bots, the top camera and the race
seen from outside were checked.

## The music

`tools/film/acid.rs`, one Rust file and no dependency: a kick, closed and open hats, a clap, and a
bass line after the Roland TB-303, a sawtooth through a four-pole resonant low-pass whose cutoff an
envelope opens on every note, wider on accents, with slides between some notes and a filter
breathing over eight bars. 132 BPM, 44 bars: the bass alone for four, the kick in, the hats, the
clap, a break at bar 24 with the filter open, a second bass line from bar 28, and the end faded out.
Never heard by Claude: it was checked by its levels (peak −1 dB, RMS −14 dB) and its spectrogram.

## The cut

`tools/film/cuts.txt` lists the shots, each a take, a start and a length in bars, so every cut falls
on the music's bars: the first screen and the press while the bass plays alone, the start lights as
the kick comes in, the launch and the first drifts, a bot's drift seen in ORBIT, the course from
above in TOP, the flight over the leap's girders in the break, the turbo at 314 km/h and the flight
over the rift as the second bass line comes in, more of TOP, and a last drift under the title, STAR
RACER and the site's address (`SITE`, given when the film is made) in the game's font over a dark band, before the fade.

## The short

In v0.1.77, at the user's word ("Refais moi une vidéo de présentation plus courte, sans voir les
menus [...] Ca doit tenir sous les 20MB limite de discord. Tu mettras la masse bot"),
`tools/film/make-short.sh` cuts a film of 36.6 seconds at 1920×1080 and 30 frames a second, 18.2
MB, for Discord: `target/film2/star-racer-short.mp4` (not versioned). No menu: a full grid of
sixteen cars, its start lights, an S start, the course from above, drifts, a flight over the void,
cars side by side, and the title.

The two takes were recorded at 1920×1080 (about 25 frames a second) against a local server, the
same way as the film's: one client holding a lobby with `--lobby "Grand Prix" --laps 3
--min-players 16 --bots 14` (`--bots`, added for it, puts bots in the lobby `--lobby` creates; the
lobby then holds sixteen), so the race waits for the filming client, which joins it and races from
the grid (take **a**, 80 s); then a spectator with `--watch "Grand Prix" --camera orbit
--camera-cycle 7` (take **b**, 70 s). Bots count among the players: with fewer players to start,
the race leaves before the filming client arrives, and it only watches.

`tools/film/short-cuts.txt` lists the shots, nine of them over 20 bars, on the film's acid techno
from bar 4, where the kick comes in. The video is encoded in two passes at 3.8 Mb/s, so the size is
known beforehand.

## The trailer

**Taken off the site in v0.1.14** (the user's list of 2026-11-23, the videos to be made again),
with the loop behind the title; the poster stays. The tools below still make them, for the next
ones.

In v0.1.6, at the user's word ("une nouvelle vidéo trailer du jeu, disant que ce sera release le 1er
janvier 2027, un truc bien monté, bien rythmé, avec une musique qui donne envie, soutenue, presque
acid trance", then "Elle doit faire 15 secondes" and "Il faut qu'on voie l'interface de race dans
la vidéo"), `tools/film/make-trailer.sh` makes `target/trailer/star-racer-trailer.mp4` (not
versioned): 15 seconds at 1920×1080 and 30 frames a second, about 15 MB.

- **The music** (`tools/film/trance.rs`): acid trance at 140 BPM in A minor, 8 bars and the last
  chord ringing. A build of two bars (detuned pads, the acid line closed, a white-noise riser and a
  snare roll), the drop with a crash (the kick, every other part pumping under it, a rolling bass
  three sixteenths after each kick, the acid line opening, hats, claps, an arpeggio through a
  dotted-eighth echo), and the end on an impact and A minor. Checked by its levels (peak −1 dB,
  about −11 LUFS) and its spectrogram; never heard by Claude.
- **The take**: `make-trailer.sh record` films a racer on Cascade from the grid of a lobby of its
  own filled with fifteen bots, with the race's interface, at 1600×900 so the debug client keeps
  its pace, alone on the machine (a second game client beside it slowed both, and the autopilot, a
  few frames late, found the walls); and a spectator, whose take failed and is not used. `--clean`
  draws a race without any HUD, for shots that want none.
- **The cut** (`make-trailer.sh cut`, `tools/film/trailer-cuts.txt`): every shot a whole number of
  half bars, so every cut falls on the beat: the lights and the start over the build, with 16 RACERS
  and NO BRAKES; a white flash on the drop and the summit leap (FLY); then a cut every half bar,
  drifts, the flight over the void, 300 KM/H, NEW CIRCUIT · CASCADE; and the title over the car past
  the line, STAR RACER, JANUARY 1, 2027 and the site's address (`SITE`), before a short fade.
