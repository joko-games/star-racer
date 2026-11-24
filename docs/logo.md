# The logos

Two logos, both drawn in SVG in `logo/` at the repository's root, both on the user's own mark: a
black disk in a cyan neon ring, its line doubled as if drawn by hand, on nothing (measured on their
128 px original: the ring 8 px wide at the edge, `#72fafc` outside fading to `#62d6d8` inside).

- **`logo/studio.svg`, the studio's**: the ring with a gamepad seen from above inside it, in the
  same neon. The user's word on 2026-11-24: "mon logo de boite de creation de jeux, le rond bleu
  stylisé et tu ajoute une manette vue du dessus au milieu". The studio is to be named
  "<something> Games" in time; the logo carries no name, so it waits for none.
- **`logo/star-racer.svg`, the game's**: the same ring with a racing car seen from the side inside
  it, its rear wing up and two speed lines behind ("Fais le logo de Star Racer, le même cercle mais
  au milieu tu fais mets une voiture vue de coté"). It is the game's first logo: the site's
  favicon, the game page's icons, and the Windows program's icon, which its shortcut and its
  installer show too.

A first try the same day, a ring open on a star in a rounded square, was set aside by the user
for these two.

## Rendering them

Nothing in a build renders SVG: the images are rendered once, when a logo changes, by
`tools/icon`, a small program outside the workspace (so its dependencies stay out of the game's),
on resvg, and committed:

```sh
cargo run --release --manifest-path tools/icon/Cargo.toml --target-dir target/tools
```

| From | To | Size |
| --- | --- | --- |
| `star-racer.svg` | `site/favicon.png`, the site's and the game page's tab icon | 64 px |
| | `client/web/icon.png`, `client/web/icon-192.png`, the page's home screen icons | 512, 192 px |
| | `logo/star-racer.png` | 1024 px |
| | `logo/star-racer.ico`, the Windows program's icon | 16 to 256 px |
| `studio.svg` | `logo/studio.png` | 1024 px |

The `.ico` holds every size Windows asks for, from a list's 16 px to a large tile's 256, so it never
scales one itself. `client/build.rs` compiles it into the Windows executable as a resource
(`winresource`, which calls the Windows SDK's `rc.exe`), with the product name; on any other system
and for the web it does nothing.
