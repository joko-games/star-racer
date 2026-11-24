STAR RACER - GAME SERVER

The same server the game runs online, with its data: the circuits (data/tracks) and the car's
tuning (data/car.ron), which it reloads every second while it runs.

RUNNING IT

  Start star-racer-server from inside this folder. It listens on port 8080 of this computer
  only, and logs what it does.

  In the downloaded game: SETTINGS, then SERVER, and type 127.0.0.1.

RACING FRIENDS

  On the same network: start it with --listen 0.0.0.0, and let your friends type ws:// and your
  computer's address with the port in their SETTINGS, for example ws://192.168.1.20:8080.
  Your system's firewall may ask whether to let it be reached.

  Over the internet: the port also has to be opened (forwarded) on your router, and your friends
  type your public address.

  Only the downloaded game can join such a server: the game in a browser, served over HTTPS,
  may not reach a plain ws:// address.

WHAT IT KEEPS

  Beside it, in the folder it was started from: accounts.ron (players' accounts), ratings.ron
  (the ranked ladder), progress.ron (experience and levels), feedback.log (feedback sent from the
  game). Keep them to keep your players.

OPTIONS

  --port <PORT>      another port than 8080
  --listen <IP>      0.0.0.0 to be reached from other computers
  --data-dir <DIR>   another data folder
  --help             every option

The game is free and in the public domain (LICENSE).
