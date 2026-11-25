# The level editor

Made again from nothing in v1.0.0-rc11 (2026-11-25), at the user's word: "Il faut que ce soit +
permissif pour les niveaux, et le l'éditeur, que tu commences maintenant [...] Faire le début de
l'éditeur". Asked, they chose it **in the game**, and a first version that **lays a circuit, tries
it and keeps it locally**; publishing waits for the accounts, which are to be the site's. The
circuits were made freer first, in v1.0.0-rc10 (crossings, a width per stretch, slopes to 40
degrees: [Tracks](tracks.md#crossings)), so the editor can lay all of that.

An editor came before, from v1.0.0-rc2 to v0.1.10 (in the history before the squashes), with tabs
for the road, the relief, what is played and the scenery; it was taken out at the user's word on
2026-11-23, "on reprendra de zéro plus tard", having been found hard to use ("c'est pas intuitif du
tout"). This one starts small and simple, the road alone, laid piece by piece.

## What the player sees

EDITOR on the first screen (not on a phone: it is laid out for a mouse and a wide screen). It
connects first, as LOBBIES does (`Intent::Editor`), the server being needed to try a circuit. Code:
`client/src/editor/` (`mod.rs` the state, undo, keeping the draft and trying it; `model.rs` the
pieces and the track file they make, nothing shown; `map.rs` the circuit seen from above;
`panel.rs` the panel).

- **The map**, right of the panel: the circuit as it will be raced, built by the race's own code
  (`track::spawn_scene`) at the graphics level's detail, seen from above and a little behind. The
  piece selected is drawn as a magenta band down the road, the way back to the line the editor
  added as a pale blue one; a road that does not close is drawn as a red line as far as it goes.
  The wheel zooms, the right or middle button drags, WASD and the arrows pan; a left click picks
  the piece under the mouse. It frames the whole circuit on opening and on NEW.
- **The panel**, down the left: THE ROAD, the pieces in order (a click selects one; the wheel
  scrolls the list); + STRAIGHT, + LEFT and + RIGHT, which add a piece after the one selected,
  as wide and as high as it, a straight of 200 m or a turn of 90 degrees with a radius of 100 m;
  the numbers of the piece selected, each with «, ‹, › and » (ten steps and one); DELETE, UNDO and
  REDO; THE WAY BACK, the radius of the turns that close the road; the circuit's length and how
  many pieces it took to close it, in green, or why it does not build, in red, in the simulation's
  own words; TRY, NEW and BACK.
- **Keys**: Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes (up to 100 changes back), Delete removes
  the piece selected, Escape goes back to the first screen.

## The pieces

A piece is a straight or a turn, with everything the player sets on it (`model::Piece`):

| | Straight | Turn |
| --- | --- | --- |
| LENGTH | 10 to 2000 m, by 10 | |
| ANGLE | | 5 to 350 degrees left or right, by 5; stepped through zero, it turns the other way |
| RADIUS | | 20 to 800 m, by 5 (more than half the road's width) |
| BANKING | | 0 to 50 degrees, by 2 |
| WIDTH | 12 to 120 m, by 2 | 12 to 120 m, by 2 |
| HEIGHT | -100 to 200 m, by 2: the road's height where the piece ends | the same |

A turn eases in and out of its curve over a transition of a fifth of its length, at most 40 m, so
steering into it is progressive and a banked turn has the transition it needs to rise into its
banking.

The track file wants the widths as changes along the lap and the heights as a relief, so the
editor places them (`Draft::description`): a change of width at the start of every piece not as
wide as the one before it, blended over 30 m, and back to the first piece's width on the way home;
a height at the end of every piece, the line at height 0, the way home coming back down to it. A
piece raised 14 m or more over the road it crosses makes a bridge ([Tracks](tracks.md#crossings));
less, and the status says where the two are too close.

## Closing the road

A circuit must turn through one whole turn and end where it began, which nobody laying turns by
hand hits. So the player lays the road as far as they like, and the editor closes it:
`editing::closing` finds the shortest way back to the line, heading `+x`, made of a turn, a
straight and a turn of one radius (THE WAY BACK, 120 m to begin with, 30 to 500), keeping only the
ways that turn the whole circuit through exactly one turn.

## Trying, and keeping the draft

- **TRY** (`ClientMessage::TryTrack`): the server checks the circuit as it would any other, then
  opens a lobby of one racing it, which the lobby list never shows (`TRY_TRACK`). The client keeps
  the circuit under that key for the lobby to draw (`Tracks::store_try`), and leaving the lobby
  comes back to the editor (`AfterLobby`), as it was.
- **The draft** is kept beside the player's other choices, in `editor.ron` (natively in the user
  data folder, in a browser its local storage), written on leaving the editor, so it survives a
  restart; NEW starts again from the first circuit (a straight, a long left, a straight).
- **Not yet**: publishing (it waits for the accounts on the site; the server still takes
  `PublishTrack` from an account), naming the circuit, what is played on the road (gaps, pads,
  checkpoints, breaches) and the scenery.

## Checked

`--editor` (with `--guest`) opens it once connected, and `--editor-try` presses TRY as soon as the
draft builds, for unattended looks: captures of the first circuit, of a draft laid as a figure
eight whose raised diagonal crosses the road twice (1.54 km, green), and of TRY racing that draft
on the autopilot, the server opening its lobby of one. The pieces' rules are covered by tests
(`model.rs`: the first circuit builds, pieces added and stepped, widths and heights where their
pieces are, a crossing refused flat and built raised; `mod.rs`: undo and redo). Not checked: any of
it by hand, synthetic clicks not reaching the window.
