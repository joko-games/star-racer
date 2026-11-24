# To do

What the user has asked for and is not done yet, in the order it will be taken up (2026-11-23).
To be tidied later, at the user's word: "Mets le TODO dans la docs, on fera le ménage plus tard".

**Since v1.0.0-rc1 (2026-11-24) the history is one commit**, and the versions below that it squashed
no longer exist as commits. Done in it besides: the cars drawn thicker, the garage in three
groups, signing in and creating accounts taken out of the game (they will be the site's), and no
pseudo or domain left in the repository, hosts being parameters.

Done in v0.1.10: the held drift checked (it survives a recentred stick, nothing to fix,
docs/simulation.md#a-held-drift-pays), the editor taken out with its button greyed
(docs/editor.md), DOWNLOAD gone from the game's first screen, and the test
`feedback_is_kept_and_limited` made reliable (a tokio file written but never flushed). v0.1.11
tried building on the machine that serves the game, given up the same day
(docs/deploy.md#why-nothing-is-built-on-the-machine). Production is deployed only when the user
asks, for the launch; every version is tested locally with `run-release.ps1`. Done in
v0.1.12: the beta panel no longer covers the server's address, and an empty leaderboard opens
nothing (docs/ui.md). Done in v0.1.13: a turbo skin seen burning in a race (ION, four cyan streaks
behind the car at 306 km/h), the lobby list and the rows, badges and chips around it made easier
to read, and the beta panel kept clear of the garage's DONE and of a phone's lobby list
(docs/ui.md#widgets, docs/feedback.md). Done in v0.1.14: the videos taken off the site, the poster staying
(docs/deploy.md#the-site), and the game server offered on the download page with its data, a
README and `--listen` (docs/releases.md). The Scaleway machine was deleted the same day: the site
and the online game are down until the user brings them back. Done in v0.1.15: the D-pad
and the arrows no longer stop on greyed buttons, the gamepad's shoulders change the first screen's
tab, quick play's bots stand on the grid from the start of the countdown, and the language module
the French will go through (`client/src/i18n.rs`, only QUICK PLAY translated yet).

1. **The user's list of the evening of 2026-11-23**, in this order (v0.1.16 did the create dialog's
   places and bots, and the sound at each try of a lost connection; v0.1.17 the race HUD's sides
   and the hundredths; v0.1.18 the holds and the give-up ring; v0.1.19 the drift charge
   slowing in its last third; v0.1.20 a first iteration of the stacked gauges, more to come at the
   user's judgement; v0.1.21 the settings reachable during a race): done.
2. **Signing in on the site, by a mailed code**: the login and the sign-up move from the game to
   the site, so the player has the browser's own tools; a code sent by mail, never a password; the
   session kept as long as possible, the page closed or not; the game takes the account from it
   for what is reserved to ranked (the user's word on 2026-11-23: "SI je me trompe pas, ça revient
   à juste dégriser le bouton ranked"). It needs a site and a way to send mail, both gone with the
   machine on 2026-11-23, so it waits for them; the native client will also need a way to receive
   the session.
3. **French**: the game and the portal translated, the player's language detected (the system's
   natively, the browser's on the web), with English for everyone else. Set aside on 2026-11-23 at the
   user's word ("On fera les traductions une autre fois"); `client/src/i18n.rs` is ready for it,
   only QUICK PLAY translated.

4. **The user's list of 2026-11-24**, after the site went up again (their words: "un installeur
   moderne pour windows", "un auto updater", "Une icone sympa qui sera le premier logo du jeu, ce
   sera la favicon et l'icone du raccourci/lanceur. Inspire toi de mon logo, le cercle bleu", "rend
   le jeu plus mais beau, plus polygoné, ce sera la qualité "ultra", et en meme temps scale davantage,
   avec plus de crans, et un "very low" qui est vraiment très simple geometriquement et tout, mais
   toujours équitable et empechant la triche"):
   - the logos, done in v1.0.0-rc5 (docs/logo.md): the studio's, the ring with a gamepad from
     above, and the game's, the ring with a car from the side, which is the favicon and the
     program's icon (not deployed yet: the site still shows the old favicon);
   - a modern Windows installer and an auto-updater, done in v1.0.0-rc6 by Velopack
     (docs/installer.md), to be seen working once a release is built with them;
   - a richer, more detailed ULTRA, more graphics levels, and a VERY LOW that is geometrically
     very simple yet fair: done in v1.0.0-rc7, six levels (docs/graphics.md#shapes), to be judged
     by the user, above all ULTRA's cars and VERY LOW's dark city.

Also waiting:

- The small sound bug on the web, which the user is to describe.
- Linux and macOS clients, which this Windows machine cannot build (the GitHub workflows could, now
  that the repository has an account again: `star-racer/star-racer`).
