# kobeat user guide

This guide covers installing kobeat on a Kobo, adding music, playing it, and
removing kobeat again. For how kobeat works and how to build it yourself, see
the [README](README.md).

- [What you need](#what-you-need)
- [Installing](#installing)
- [Adding music](#adding-music)
- [Starting the player](#starting-the-player)
- [Using the player](#using-the-player)
- [Starting and stopping kobeatd](#starting-and-stopping-kobeatd)
- [Opening the player from NickelMenu, KFMon or a script](#opening-the-player-from-nickelmenu-kfmon-or-a-script)
- [Hiding or showing the menu entries](#hiding-or-showing-the-menu-entries)
- [Controlling kobeat from a shell (kobeatctl)](#controlling-kobeat-from-a-shell-kobeatctl)
- [Updating](#updating)
- [Uninstalling](#uninstalling)
- [Troubleshooting](#troubleshooting)

## What you need

- A Kobo with Bluetooth audio, such as the Libra Colour, on firmware 4.x.
  kobeat is tested on 4.46.
- Bluetooth headphones or a speaker, paired with the Kobo in its own
  Bluetooth settings. kobeat plays through the Kobo's Bluetooth output.
- A computer to copy files to the Kobo over USB.

You don't need NickelMenu, KFMon or Nickelite. If you already use them, kobeat
works alongside them.

## Installing

1. Download the latest `kobeat-<version>.zip` from the
   [releases page](https://github.com/KiraGreifeneder/kobeat/releases).
2. Connect the Kobo to your computer over USB.
3. Unzip the archive into the root of the Kobo's storage, **merging** it with the
   folders that are already there. Your file manager may need to show hidden
   files, since the zip contains two hidden folders:
   - `.kobo/KoboRoot.tgz`: the plugin that adds kobeat to the Kobo's
     interface.
   - `.adds/kobeat/kobeatd` and `.adds/kobeat/kobeatctl`: the background
     player and its command line tool.
4. Eject the Kobo and unplug it. It installs the plugin and restarts.

After the restart you'll see a **Kobeat** tab in the bottom bar of the home
screen.

## Adding music

Put your music in the `music` folder in the root of the Kobo's storage (that's
`/mnt/onboard/music` if you're using SSH or similar). Subfolders are fine; the player lets you
browse them. kobeat creates the folder itself the first time it runs, so it may
already be there.

Common formats such as MP3, FLAC, Ogg Vorbis, WAV and AAC/M4A play.

## Starting the player

There are two built-in ways to open the player:

- Tap **Kobeat** in the home screen's bottom bar.
- While reading, open the reader menu (tap the middle of the page) and choose
  **Kobeat**.

The player covers the whole screen. Tap the back arrow in its title bar to
return to where you were. Music keeps playing after you close it.

## Using the player

From top to bottom:

- **Status line and Start/Stop kobeatd button.** The status line shows errors
  and confirmations, such as "kobeatd is not running". See
  [Starting and stopping kobeatd](#starting-and-stopping-kobeatd).
- **Now playing.** The track's title, a progress bar and the elapsed and total
  time. Tap anywhere on the progress bar to jump to that point.
- **Previous, Play/Pause, Next.** Previous and Next move through the queue.
- **Music browser.** Shows the current folder:
  - Tap a folder to open it, and **Up** to go back to the parent folder.
  - Tap a file to play it.
  - **Prev page** and **Next page** page through long folders; the number in
    between shows where you are.
- **Play this folder.** Replaces the queue with the files in the current
  folder and starts playing. Tick **Include subfolders** first to also add the
  files from folders inside it (up to three levels deep).

The screen updates once a second while the player is open.

## Starting and stopping kobeatd

`kobeatd` is the background program that actually plays the music. The player
is only a remote control for it, so kobeatd must be running for anything to
play. It isn't started automatically.

- Tap **Start kobeatd** to start it. It takes a moment, and the button is
  disabled for a few seconds so you can't start it twice.
- Tap **Stop kobeatd** to stop playback and shut it down, for example to save
  battery when you're done listening.

The player runs kobeatd from `.adds/kobeat/kobeatd`, or from `kobeatd` in the
root of the storage if that's where you put it. kobeatd's output goes to
`/tmp/kobeatd.log`.

If you'd rather have kobeatd start on its own, run
`/mnt/onboard/.adds/kobeat/kobeatd` from a NickelMenu or KFMon action, or from
a startup script.

## Opening the player from NickelMenu, KFMon or a script

The plugin listens on `/tmp/kobeat-ui.fifo` for the words `open`, `close` and
`toggle`. Anything that can run a shell command can open the player.

NickelMenu (add to a file in `.adds/nm/`):

```
menu_item :main   :Kobeat :cmd_spawn :quiet:echo open > /tmp/kobeat-ui.fifo
menu_item :reader :Kobeat :cmd_spawn :quiet:echo open > /tmp/kobeat-ui.fifo
```

KFMon: point a watch's `action` at a script containing:

```sh
echo open > /tmp/kobeat-ui.fifo
```

If you open the player this way, you may want to hide the built-in entries
(next section).

## Hiding or showing the menu entries

Both built-in entries are shown by default. To hide one, create an empty file
with the matching name in `.adds/kobeat` on the Kobo's storage:

| File                          | Hides                                              |
| ----------------------------- | -------------------------------------------------- |
| `.adds/kobeat/no_nav_tab`     | the **Kobeat** tab in the home screen's bottom bar |
| `.adds/kobeat/no_reader_menu` | the **Kobeat** item in the reader menu             |

To show an entry again, delete its file. The plugin only checks these files
when the Kobo starts, so restart the Kobo after adding or removing one.

## Controlling kobeat from a shell (kobeatctl)

If you have a shell on the Kobo (over SSH or telnet), `kobeatctl` sends the
same commands as the player:

```sh
cd /mnt/onboard/.adds/kobeat
./kobeatctl load-and-play /mnt/onboard/music/song.mp3
./kobeatctl pause
./kobeatctl resume
./kobeatctl seek 90            # jump to 1:30
./kobeatctl play-next
./kobeatctl play-previous
./kobeatctl get-player-state
./kobeatctl shutdown           # stop kobeatd
```

Run `./kobeatctl help` for the full list.

## Updating

To update kobeat, install the new release the same way as the first time,
replacing the old files.

Kobo firmware updates remove the plugin. After a firmware update, unzip the
release again (or just copy `.kobo/KoboRoot.tgz` back) and eject. Your music
and the files in `.adds/kobeat` aren't affected.

## Uninstalling

1. Stop kobeatd with the player's **Stop kobeatd** button.
2. Create an empty file named `uninstall` in `.adds/kobeat` on the Kobo's
   storage, then restart the Kobo. The plugin removes itself and deletes the
   `uninstall` file.
3. Delete the `.adds/kobeat` folder. Delete the `music` folder too if you
   don't want to keep your music on the Kobo.

If Nickel ever crashes while loading the plugin, the plugin uninstalls itself
automatically, so a bad update can't leave the Kobo stuck.

## Troubleshooting

- **The player says "kobeatd is not running".** Tap **Start kobeatd**. If it
  says kobeatd wasn't found, check that `.adds/kobeat/kobeatd` exists.
- **kobeatd starts but nothing plays.** Make sure your headphones or speaker
  are connected in the Kobo's Bluetooth settings, then check
  `/tmp/kobeatd.log` for errors.
- **The music browser says the folder doesn't exist.** kobeat creates the
  `music` folder automatically, so this usually means the storage was still
  mounted over USB. Eject the Kobo and reopen the player.
- **The Kobeat tab doesn't appear.** Check that `.adds/kobeat/no_nav_tab`
  doesn't exist, and that `.kobo/KoboRoot.tgz` was installed: the file
  disappears from `.kobo` once the Kobo has installed it. The plugin's log
  messages start with `Kobeat` (`logread | grep Kobeat` in a shell).
