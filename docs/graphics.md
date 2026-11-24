# Graphics quality

Since v1.0.0-rc4, at the user's word: the game measures the machine it runs on, silently ("je ne
veux pas déranger avec un popup d'autorisation"), and scales how fine it is drawn to it, from about
60 frames a second on an entry-level phone, ugly being fine there, to 60 frames a second in a
browser on a GTX 1050 Ti at the top; and the top was raised. Code: `client/src/quality.rs`.

## Six levels

Four, LOW to ULTRA, from v1.0.0-rc4; six since v1.0.0-rc7, at the user's word on 2026-11-24 ("rend
le jeu plus mais beau, plus polygoné, ce sera la qualité "ultra", et en meme temps scale davantage,
avec plus de crans, et un "very low" qui est vraiment très simple geometriquement et tout, mais
toujours équitable et empechant la triche"): the ULTRA of before became VERY HIGH, and a new ULTRA
and VERY LOW were added at the two ends.

| | VERY LOW | LOW | MEDIUM | HIGH | VERY HIGH | ULTRA |
| --- | --- | --- | --- | --- | --- | --- |
| Pixels drawn on a web page, per page pixel (at most the screen's density) | 0.45 | 0.55 | 0.8 | 1 | 1.25 | 1.5 |
| The moon's shadows | none | none | 1024 | 2048 | 4096 | 4096 |
| Glow (bloom) | off | off | on | on | on | on |
| Multisampling | off | off | off | off | 4× | 4× |
| The city's traffic and searchlights | off | off | off | on | on | on |
| Shapes ([below](#shapes)) | very low | low | standard | standard | standard | ultra |

A native window always draws its own pixels: the first row is the web's only, where a phone's
dense screen is the costliest thing of all.

## Shapes

Since v1.0.0-rc7 a level also says how many polygons the shapes are built with
(`quality::Detail`):

| | VERY LOW | LOW | standard | ULTRA |
| --- | --- | --- | --- | --- |
| Strips across the road (how closely it follows a banked turn's curve) | 4 | 6 | 8 | 16 |
| A car's body | every other section | as drawn | as drawn | 3 sections more between each two, rounded |
| Sides of a wheel | 6 | 8 | 8 | 20 |
| Stars, in percent of the standard sky | 20 | 50 | 100 | 160 |
| The galaxy's glow and the nebulae | off | on | on | on |
| Pieces round a planet | 16 | 32 | 64 | 128 |
| The scenery's neon (windows, bands, crowns, stripes, arches) | off | on | on | on |

On ULTRA a car's sections are joined by a curve through their neighbors (Catmull-Rom, never past
either section it lies between), and each ring round a section gets a point between each two of
its eight, pushed out a little: the body rounds off between its lines, which stay where they were,
and the neon and the stripe with them (`car_model::shaped`, `rounded_ring`).

**Fair at every level.** No level sees more, or further, than another:

- the simulation never sees any of this: it is the server's, and the same for everyone;
- what can stand in a player's way is there at every level, in the same place and of the same size:
  the road, its walls and every marking a driver reads (pads, gaps' lips, checkpoints, kerbs), the
  other cars (a coarse car is the same size as a fine one, `the_levels_cut_the_cars_finer_or_coarser`),
  every building, and the fog;
- VERY LOW takes away only what hides nothing: the scenery's neon (its buildings are still built,
  from the same dice, in the same places, only dark; `track_meshes`), and the sky's decoration.

When the level changes, the sky is built again at once (`sky::rebuild_sky`); a car spawned after
it is built at the new level, those on the road keep theirs (`race::CarAssets` keeps one set of
meshes per look and level); the circuit takes it at the next race, its meshes being built on
entering one.

## Fitted to the machine

Nothing that brings up a prompt, only what the game is told anyway (`measure_machine`,
`detect`):

- the graphics adapter the renderer was given: its name, and whether it is a discrete GPU, an
  integrated one or software (a browser often hides the kind and gives the name, read for
  `GeForce`, `RTX`, `GTX`, `Radeon RX`, `Arc A`, `Intel`, `Adreno`, `Mali`, `PowerVR`, `Apple`);
- the processor's threads (natively `available_parallelism`, in a browser
  `navigator.hardwareConcurrency`);
- in a browser, the memory it admits to (`navigator.deviceMemory`, where there is one), and the
  screen's pixel density;
- whether the screen is a touchscreen.

A card of the last few years (an RTX, a Radeon RX 6000 or later, an Arc) holds ULTRA; another
discrete GPU, or a strong name, VERY HIGH; an integrated one MEDIUM; software VERY LOW; a phone's
chip LOW, MEDIUM for the big ones; four threads or fewer at most MEDIUM; 4 GB or less, LOW, and
2 GB or less, VERY LOW. The
level chosen is logged at `info` with what it was chosen from ("graphics fitted to the machine").

Then the race measures it: on AUTO, a frame rate under 48 for three seconds of a race steps the
level down one (`adapt`), at most every six seconds, never under VERY LOW, and never back up past what
was detected. A hitch over half a second is not counted (a hidden tab, loading).

## In the settings

GRAPHICS & SOUND, a sub-menu of the settings (`ui/sound_graphics.rs`, `--sound-graphics`): AUTO,
showing the level it draws at, or LOW, MEDIUM, HIGH or ULTRA, kept with the menu choices
(`Preferences::graphics`), with what the machine was taken for written under it. A level picked
holds, races included.

## Measured

On the development machine (i5-9600K, GTX 1050 Ti), natively: detected as a discrete GTX 1050 Ti
with 6 threads, ULTRA then (VERY HIGH since v1.0.0-rc7), a drifting race at 60 frames a second
with vsync. On v1.0.0-rc7, the same quick play race captured at VERY LOW, HIGH and ULTRA in debug,
at 70, about 60 and 58 frames a second shown: the city the same buildings in the same places at
every level, dark on VERY LOW. Not measured: a phone, the browser on this machine, ULTRA in
release.
