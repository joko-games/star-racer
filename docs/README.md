# Documentation

The reasoned technical decisions behind Star Racer. `CLAUDE.md` holds the working rules and
the current status; these pages explain *why* the code looks the way it does.

- [To do](todo.md) — what has been asked for and is not done yet, in order.

- [Architecture](architecture.md) — workspace layout, transport choice, deployment constraints, how the race is drawn, the trail behind every car.
- [Protocol](protocol.md) — wire format, versioning, handshake, authentication.
- [Simulation](simulation.md) — conventions, the car model and its tuning, drift, walls, autopilot.
- [Tracks](tracks.md) — the track file format, transitions, scenery, validation, built geometry.
- [Level editor](editor.md) — the editor in the game, closing the road by itself, trying and publishing circuits.
- [Lobbies](lobbies.md) — lobby settings and phases, race rules, how the server runs them.
- [Quick play and ranked](ranked.md) — finding a race at once, the queue, matchmaking, ratings.
- [Rewards](rewards.md) — experience, levels, and the cars, trails and horns they unlock.
- [Interface](ui.md) — screens, look, widgets, keyboard and gamepad navigation.
- [The beta panel and feedback](feedback.md) — the controls, the version and FEEDBACK in an amber frame, the editor, and feedback kept and mailed by the server.
- [Sound](sound.md) — sounds synthesized in code, and when they play.
- [Graphics quality](graphics.md) — the machine measured silently, four levels, AUTO stepping down when a race runs slow.
- [Sky](sky.md) — the starry night over the circuits, at infinity and made of triangles.
- [Night](night.md) — the glow, the haze, the light on the road, the piers, and the city that moves.
- [Latency](latency.md) — where the time between a press and the car's answer goes, and how it is measured.
- [Web build](web.md) — building and serving the browser client, what differs from native.
- [Deployment](deploy.md) — setting the machine up once, and the two ways a build goes online.
- [The film](film.md) — the game filming itself, the takes, the acid techno and the cut, the short for Discord, and the 15-second trailer.
- [Releases and downloads](releases.md) — a GitHub release for every deployed version, and the download page.
- [The logos](logo.md) — the studio's and the game's, on the user's ring, and how they become the icons.
- [Continuous integration](ci.md) — the four checks, the deployment, and why it could not exist before.
