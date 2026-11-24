# The beta panel and feedback

Since v0.1.5, at the user's word: "Diminue la taille des contrôles clavier/souris à l'écran,
réorganise les et mets les dans un cadre qui devra être distinctif un peu, car uniquement lié à
l'état beta du jeu, il y aura donc dedans les contrôles, la version, et un bouton pour envoyer
feedback/déclaration de bug à the game's mailbox, avec un éditeur de texte simple et pratique",
and "Nous sommes donc en beta et plus en alpha".

## The panel

`client/src/ui/beta.rs`. A small frame outlined in amber, the one color nothing else in the
interface frames with, a BETA tag standing on its top edge:

- **In a race** (drawn by the HUD): the controls as small key hints on two lines (the bindings the
  player chose, in the device they last used), the version and the frame rate, and FEEDBACK / BUG.
  Bottom right on a desk; top left on a touchscreen, which has no keys to hint at and whose bottom
  belongs to the thumbs and the gauges, while the race leaves the top left empty (the circuit's
  name is hidden during a race since v0.1.5).
- **On every other screen** (`spawn_menu_panel`): STAR RACER · BETA and the version, and FEEDBACK /
  BUG. It replaced the version written small in the bottom left corner, where it stays, except
  where a screen holds something there (`menu_panel_place`, since v0.1.13, after the user found
  it covering the first screen): in the garage it sits at the bottom middle, under the car, clear
  of DONE; on a phone's lobby list, which fills the screen down to the players online, and its
  garage, whose rows run down past the bottom edge, it is not shown, FEEDBACK being on the first screen.

The first screen's badge under the title says BETA, in amber, where it said ALPHA in magenta.

## The editor

FEEDBACK / BUG opens a dialog over whatever is on screen: a text area of several lines (up to
`MAX_FEEDBACK_CHARS`, 4,000 characters, counted under it), SEND and CANCEL, Ctrl+Enter to send and
Escape to close. While it is open the keyboard is its own: `take_the_keyboard` reads Escape and
Ctrl+Enter, then clears every key from the game's input for the frame, so a W typed does not
accelerate the car and Escape opens no menu; the text area reads the keyboard's own events, which
that leaves alone. On a touchscreen the area is typed into through the browser's own text box, as
every field there is. `--feedback` opens it at launch, for unattended checks.

What is sent: the text, and a line of context the client writes itself, the version, native or web,
touch or not, and the screen it was sent from. The dialog says *Sent. Thank you!* and empties the
area once the server has kept it, or why not; offline it says to connect first.

## On the server

`ClientMessage::Feedback { text, context }` (protocol 46), answered by
`ServerMessage::FeedbackReceived(kept)`, from the menus and from a lobby alike. The server
(`server/src/feedback.rs`) refuses one that is empty or too long, or comes less than
`FEEDBACK_EVERY_SECONDS` (20) after the last from the same connection, then:

- **keeps it**: appended to `feedback.log` (`--feedback`), beside the ladder and for the same
  reason (a deployment replaces the data directory, not the service's home), a header line with
  the time in UTC and the player's name, the context, the text;
- **mails it on** to the game's mailbox, in the background, if the mailbox's login is there.

The mail goes out through `curl`, already on the machine, over SMTP with TLS on port 465 to the
server the login file names. No TLS or mail library in the server: it is cross-compiled from
Windows without a C toolchain, which the mail crates' TLS would need. The login file is a netrc file,
`mail.netrc` beside the ladder (`--mail-login`), written by the user on the machine, so the password
is never on a command line nor in the repository:

```
machine smtp.example.org
login feedback@example.org
password ...
```

Since v1.0.0-rc1 the mail goes to that same mailbox, the login's address, which sends to itself: no
address is written in the repository.

(`chmod 600` it, **owned by the service's user**, `star-racer`: the user's first copy belonged to root,
which the server cannot read.) Its `machine` is the SMTP server and its `login` the sender. Without
it, feedback is only kept, and read from the file.

**Scaleway blocks outbound SMTP** (ports 25, 465 and 587, to any mail server) on its instances by
default, against spam: tried on 2026-11-23 as the service's user, the mail timed out, while the
machine reached the web on 443. Until SMTP is enabled for the instance in Scaleway's console (its
security group), feedback is kept in `feedback.log` and the server logs `cannot mail feedback`.
