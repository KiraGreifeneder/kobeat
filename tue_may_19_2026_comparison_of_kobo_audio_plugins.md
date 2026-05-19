# Copilot Chat Conversation Export: Comparison of Kobo audio plugins

**User:** @KiraGreifeneder
**Thread URL:** https://github.com/copilot/c/cc1d7247-bb55-45fd-b428-854dce1a8be4

## @KiraGreifeneder

How does audiobook.koplugin inject audio on a Kobo compared to my still very rudimentary android HAL solution I found in KoboAudio?

## Copilot

`audiobook.koplugin` is mostly **not doing low-level audio injection itself** on Kobo. It rides whatever playback path Kobo already exposes, while your `KoboAudio` README describes a much lower-level **direct HAL/control-socket injection** approach.

**Short version:**
- **audiobook.koplugin:** generate WAV, then hand it off to an existing playback stack:
  - on MTK Kobo: `gst-launch-1.0 ... ! mtkbtmwrpcaudiosink`
  - on BlueZ Kobo: `aplay -D bluealsa ...`
  - on Android: Java `MediaPlayer`
- **KoboAudio:** talks to Kobo’s Android-derived Bluetooth audio plumbing more directly, by sending the A2DP/HAL control opcode to a local UNIX socket and then streaming raw PCM into the spawned data socket. That’s much closer to “real audio injection.”  

That difference is visible in the repos:

- `audiobook.koplugin` selects GStreamer + `mtkbtmwrpcaudiosink` as its primary Kobo BT path in `findAudioPlayer()`. It checks whether the sink exists via `gst-inspect-1.0`, then returns `gst-launch-1.0` as the player command and marks the player type as `"gst-bt"`. ```lua name=ttsengine.lua url=https://github.com/stradichenko/audiobook.koplugin/blob/c4bc9c233f535666aa0212a97b0409d2b816dcf6/ttsengine.lua#L2921-L2940
    -- 1) GStreamer with Kobo Bluetooth A2DP sink (primary on Kobo Libra Colour etc.)
    if self:commandExists("gst-launch-1.0") then
        local handle = io.popen("gst-inspect-1.0 mtkbtmwrpcaudiosink 2>/dev/null | head -1")
        if handle then
            local result = handle:read("*a")
            handle:close()
            if result and result:match("Factory Details") then
                self.audio_player_type = "gst-bt"
                logger.dbg("TTSEngine: Found GStreamer with Bluetooth audio sink")
                return "gst-launch-1.0"
            end
        end
    end
```  
- On non-MTK Kobo devices, the plugin bundles and starts **BlueALSA** so that normal ALSA playback can reach BT headphones through a `bluealsa` PCM device instead of directly speaking HAL/socket protocol. citeturn0commentaryto=multi_tool_use.parallel0
- The plugin documentation explicitly models Kobo audio as shell-command playback pipelines, not direct HAL control. For BlueALSA, it shows `aplay -q -D bluealsa {WAV_FILE}` feeding the BlueALSA ALSA plugin and then `bluetoothd → A2DP sink`. ```markdown name=docs/PLATFORM_AUDIO.md url=https://github.com/stradichenko/audiobook.koplugin/blob/c4bc9c233f535666aa0212a97b0409d2b816dcf6/docs/PLATFORM_AUDIO.md#L124-L166
### Audio Pipeline (BlueALSA)

┌─────────────┐    ┌──────────────┐    ┌─────────────────────────────┐    ┌──────────────┐
│ WAV file    │───>│ aplay -D     │───>│ libasound_module_pcm_       │───>│ bluetoothd   │
│             │    │   bluealsa   │    │   bluealsa.so (ALSA plugin) │    │ → A2DP sink  │
└─────────────┘    └──────────────┘    └─────────────────────────────┘    └──────────────┘
```  
- For MTK Kobo playback, other parts of the plugin likewise force the GStreamer sink `mtkbtmwrpcaudiosink` rather than opening sockets themselves. ```lua name=mediaengine.lua url=https://github.com/stradichenko/audiobook.koplugin/blob/c4bc9c233f535666aa0212a97b0409d2b816dcf6/mediaengine.lua#L511-L571
    if Device:isKobo() and os.execute("gst-inspect-1.0 mtkbtmwrpcaudiosink >/dev/null 2>&1") == 0 then
        sink_arg = " --audiosink=mtkbtmwrpcaudiosink"
    end
```  
- Your `KoboAudio` README, by contrast, says Kobo’s BT pipeline is really backed by the legacy Android Audio HAL and that after sending the HAL “START” opcode `0x02` to the control socket, the daemon spawns a secondary data socket that accepts raw PCM. That is a lower-layer feed point than anything `audiobook.koplugin` appears to use. ```markdown name=README.md url=https://github.com/KiraGreifeneder/KoboAudio/blob/9c4dce89fb95aa6956b677b218c500b0d244a27f/README.md#L1-L8
However, the actual audio data pipeline relies on the legacy Android Audio HAL (`libaudio.a2dp.default.so`). The daemon listens on a local UNIX domain control socket. Once the exact Android Hardware Abstraction Layer OpCode for "START" (`0x02`) is sent to the control socket, the daemon acknowledges it (`0x00`) and dynamically spawns a secondary data socket to ingest raw PCM audio.
```  

So architecturally:

| Aspect | `audiobook.koplugin` | `KoboAudio` |
|---|---|---|
| Level | High-level userland playback | Low-level transport/HAL probing |
| Input format | Usually WAV files | Raw PCM stream |
| Main mechanism | Spawn existing players/sinks (`gst-launch`, `aplay`, Android APIs) | Speak HAL control protocol over UNIX socket |
| Bluetooth routing | Reuses Kobo sink/plugin or BlueALSA | Attempts to drive Kobo’s native BT audio path directly |
| Portability | Higher | Lower, more device/firmware-specific |
| Control | Less precise | More precise/powerful |

A few practical implications:

1. **audiobook.koplugin is safer and more portable.**  
   It adapts to multiple environments: MTK Kobo via GStreamer sink, BlueZ Kobo via bundled BlueALSA, Android via JNI `MediaPlayer`. The docs even show this selection order explicitly. citeturn0commentaryto=multi_tool_use.parallel1

2. **KoboAudio is closer to the real hidden Kobo audio plumbing.**  
   If your goal is “bypass Kobo’s built-in audio stack and let me push PCM myself,” your HAL/socket direction is the more direct one.

3. **On MTK Kobo, audiobook.koplugin likely still depends on the same underlying proprietary stack indirectly.**  
   `mtkbtmwrpcaudiosink` strongly suggests a wrapper around MediaTek’s BT audio RPC path, so the plugin is probably using Kobo’s exposed GStreamer sink as a convenience layer over the same proprietary machinery your repo is trying to hit directly. That’s an inference from naming and usage, not something I found explicitly documented in the repo. citeturn0commentaryto=multi_tool_use.parallel1turn0commentaryto=multi_tool_use.parallel0

4. **Your approach could eventually subsume theirs for Kobo-specific playback.**  
   If you finish the HAL path and make it reliable, you could replace the plugin’s dependency on `gst-launch-1.0`/`mtkbtmwrpcaudiosink` with a custom streamer that feeds PCM directly. That would give tighter control over latency, buffering, pause/resume, and possibly seeking.

One important note: the code search results I used may be incomplete because search responses are capped. If you want to inspect more matches manually, use the GitHub code search UIs for:
- `stradichenko/audiobook.koplugin`: `mtkbtmwrpcaudiosink`, `bluealsa`, `findAudioPlayer`
- `KiraGreifeneder/KoboAudio`: `audio_hw_device`, `A2DP_CTRL_CMD_START`, `out_write`, `set_parameters`

If you want, I can do a **deeper repo-to-repo comparison** next:
1. map the exact playback call chain in `audiobook.koplugin`, and
2. compare it against the concrete files/functions in `KoboAudio` once we identify your HAL implementation files.

## @KiraGreifeneder

yeah please tell me more about audiobook.koplugin. I don't care about supporting kindle, but if there is a better solution to playing audio in the background than my solution, then I'd prefer thta. I need scrubbing, volume control etc.

## Copilot

Yes — `audiobook.koplugin` looks like a **better product-level foundation** for Kobo background playback than a raw HAL socket injector, **if** your goals are things like:

- scrubbing/seek
- pause/resume
- playback surviving UI changes
- Bluetooth routing that mostly “just works”
- less firmware-specific fragility

But it’s better as a **playback orchestration layer**, not because it discovered a magical lower-level audio path.

## The key distinction

Your `KoboAudio` approach is closer to:

- “I know where PCM ultimately enters Kobo’s BT stack”
- “I can probably feed raw audio directly”
- “I own the transport”

`audiobook.koplugin` is closer to:

- “Use whatever stable playback primitive the device already exposes”
- “Wrap it in a robust Lua UI/player lifecycle”
- “Add background control, process monitoring, cleanup, and fallback logic”

For an actual **audiobook/media player**, that second approach is usually the better starting point.

---

## What audiobook.koplugin is really doing on Kobo

From the code you shared earlier, the Kobo side is basically:

### 1. It generates or obtains audio files
Usually WAV/TTS output.

### 2. It picks a playback backend dynamically
The important Kobo-relevant paths appear to be:

- **GStreamer + `mtkbtmwrpcaudiosink`** on MTK-based Kobo BT path
- **BlueALSA + `aplay -D bluealsa`** on BlueZ-style setups
- generic playback helpers in the media engine

The important architectural point is: it is **not manually shoving PCM into a HAL socket**. It is using a higher-level playback stack that already knows how to reach the output device.

---

## Why this is probably better for your use case

If you want **background audiobook playback**, raw injection is only one small piece of the problem.

You also need:

- process ownership
- restart/cleanup behavior
- position tracking
- completion detection
- seek semantics
- pause/resume behavior
- coexistence with UI state changes
- Bluetooth reconnection behavior
- graceful failure handling

That is where `audiobook.koplugin` is likely much more valuable than the low-level HAL work.

---

## Scrubbing / seeking

This is the biggest reason I’d lean toward the plugin-style design.

From the snippets already visible, the media layer is built around **real player control semantics**, not just “stream bytes and hope.”

What stood out:

- there is an `AudiobookPlayer:onSeek(pct)` UI callback
- the media engine has `_seek_offset`
- GStreamer playback uses restart-with-offset logic and URI time fragments like `#t=<seconds>`

That tells me the design already treats seeking as a first-class feature.

### Practical implication
That’s much easier to build on than a raw HAL PCM socket, because with raw streaming you must invent seek behavior yourself:

- decode from target position
- refill buffers
- handle timing discontinuity
- keep UI state synchronized
- deal with compressed formats vs PCM

So if your goal is **book player UX**, the plugin’s playback-control layer is almost certainly the more useful template.

---

## Background playback

Also promising.

From the UI code you showed:

- the player can be **minimized**
- the mini-player remains active
- the widget shrinks so touch passes through elsewhere
- playback state is clearly meant to continue outside the main player view

That suggests the architecture already expects playback to outlive the currently focused UI surface.

For your use case, this is much more important than raw injection purity.

A HAL injector can play audio, sure — but that alone doesn’t give you a real background-capable player model.

---

## Pause/resume/stop lifecycle

This is another area where `audiobook.koplugin` seems stronger.

The Android helper you showed is not Kobo-specific, but it reveals the project’s general design philosophy:

- explicit `playFile`
- `pausePlayback`
- `resumePlayback`
- `stopPlayback`
- completion listeners
- cleanup/release behavior
- audio-focus style management

On Kobo, the same mindset shows up in process-based orchestration:
- launch a backend
- capture PID
- poll position
- watch completion
- clean up orphaned processes
- recover from stuck exclusive sockets

That’s exactly the kind of stuff you need in the real world.

A low-level socket solution gives you none of that for free.

---

## Bluetooth routing

This is where the tradeoff gets interesting.

### audiobook.koplugin advantage
It uses already-available routes:

- `mtkbtmwrpcaudiosink` when present
- BlueALSA when needed

That means:
- less reverse engineering in your app layer
- more likely to survive typical playback use
- less custom protocol burden on you

### Your HAL approach advantage
If Kobo’s exposed higher-level stack is limited, unstable, or missing features, direct HAL control might eventually give you:
- tighter control over buffering
- potentially lower latency
- more deterministic transport
- possibly easier custom DSP/mixing later

### But...
For an audiobook player, **latency is not the hard part**.  
State management is.

So unless Kobo’s higher-level sink path is fundamentally broken, I would usually choose the plugin-style approach.

---

## Volume control

This is the weak spot from what I’ve seen so far.

I haven’t seen clear evidence in the snippets you pasted that `audiobook.koplugin` has a strong Kobo-native volume-control layer.

That suggests a few possibilities:

1. volume is delegated to the underlying player/backend
2. volume is delegated to system mixer state
3. volume UX is incomplete or backend-specific
4. for BT devices, volume may rely partly on headset-side AVRCP absolute volume or device-side controls

For your goals, volume control may be the main area where you’d still need custom work even if you adopt the plugin’s overall design.

### If you build on the plugin approach
I’d think in terms of:
- **seek / pause / resume / background state** from plugin architecture
- **volume** added separately per backend

For example:
- GStreamer backend: maybe set volume in pipeline if using a controllable pipeline
- ALSA backend: mixer/softvol if available
- BT-only scenarios: headset AVRCP or software pre-scale before playback

---

## Where I think the plugin is strongest

Best parts to copy conceptually:

### 1. Backend selection
Choose the best available route at runtime instead of hardcoding one path.

### 2. Separate UI from playback engine
The player widget shouldn’t own audio transport directly.

### 3. Treat playback as a managed session
Keep:
- generation IDs
- PIDs
- completion watchers
- cleanup routines
- restart-safe logic

### 4. Implement seek at the player/orchestration level
Don’t make the transport layer responsible for the whole UX.

### 5. Reuse existing sinks when possible
Especially on Kobo, where the proprietary stack is fragile.

---

## Where your HAL approach is still valuable

Your work is still important if you want to answer questions like:

- can Kobo’s native BT audio path be driven without GStreamer?
- can we get a more direct PCM injection path?
- can we reduce dependencies on `gst-launch` or BlueALSA?
- can we own buffering and stream timing more precisely?

That could become a **backend** under a plugin-like playback manager.

And honestly, that may be the best final design:

- **frontend/control plane from audiobook.koplugin concepts**
- **transport backend from your KoboAudio reverse engineering**

In other words:

> don’t replace the player architecture with HAL injection;
> replace only the lowest playback backend.

That’s the architecture I’d recommend.

---

## My recommendation

If your goal is a **usable Kobo audiobook player with background playback**, I would recommend:

### Prefer this architecture
- **high-level playback manager** like `audiobook.koplugin`
- pluggable backends
- seek/scrub implemented above the transport
- robust lifecycle/process cleanup
- minimized/background UI support

### Use your HAL work as an optional backend
Only if it proves more reliable or capable than:
- `mtkbtmwrpcaudiosink`
- BlueALSA
- other exposed playback routes

That gives you the best of both worlds.

---

## Practical decision rule

Use the plugin-style approach if you care most about:
- scrubbing
- background playback
- UX
- stability
- maintainability

Use the HAL-direct approach if you care most about:
- deepest possible hardware control
- replacing Kobo’s exposed playback tools
- experimenting with native PCM streaming
- future custom audio engine work

For an **audiobook app**, I’d choose the first and keep the second as a specialized backend.

---

## My honest bottom line

If I were building this with your stated priorities, I would **not** make the raw HAL socket path the main architecture.

I would build:

1. a **player/controller layer** modeled after `audiobook.koplugin`
2. a **Kobo backend interface**
3. multiple output backends:
   - GStreamer/MTK sink
   - BlueALSA
   - your direct HAL injector as experimental/advanced backend

That is much more likely to get you:
- seek
- background playback
- pause/resume
- cleanup
- future volume support

without tying the whole app to one brittle proprietary protocol.

If you want, next I can sketch a **clean Kobo playback architecture** for your project — e.g. modules for:
- transport backend
- decoder
- position tracker
- scrubbing controller
- volume abstraction
- background session manager.

## @KiraGreifeneder

Sure, go ahead. I mainly want it as a music player. at the moment, the HAL stuff does let me play audio in the background independent of whatever Kobo's frontend is doing. Also, I don't want to use Koreader, I pretty much need to be able to start it from a script through nickelmenu on its own.

*System: User retried message*

## Copilot

Then I’d recommend this direction:

**Keep your “direct background playback outside Nickel” idea, but stop thinking of the HAL path as the whole app. Make it just the output backend.**  
For a Kobo music player launched from NickelMenu via script, you want a small daemonized player architecture with interchangeable outputs.

## Recommended architecture

### 1. A standalone background player daemon
Launchable from NickelMenu, independent of KOReader/Nickel UI.

Responsibilities:
- own playback state
- keep running in background
- expose commands over a local socket/FIFO
- survive frontend script exits
- report current track, time, duration, volume, state

Example commands:
- `play <file>`
- `pause`
- `resume`
- `stop`
- `seek <seconds>`
- `seek_rel <delta>`
- `set_volume <0..100>`
- `next`
- `prev`
- `status`

Good IPC options:
- UNIX socket
- named pipe + state file
- tiny HTTP server on localhost if you want simplicity

I’d pick a **UNIX socket**.

---

### 2. A thin NickelMenu/script frontend
NickelMenu actions should just send commands.

Examples:
- “Start player daemon”
- “Play/Pause”
- “Next track”
- “Previous track”
- “Volume up/down”
- “Seek forward/back”

That keeps UI glue simple and lets playback continue even if Nickel changes screens.

---

### 3. Separate these layers clearly

#### A. Playlist/session layer
- current queue
- shuffle/repeat
- metadata
- track transitions

#### B. Playback controller
- state machine: stopped / buffering / playing / paused / seeking / error
- elapsed position
- duration
- gapless/prebuffer if you want later

#### C. Decode layer
- decode MP3/FLAC/Ogg/AAC to PCM
- can use ffmpeg, mpg123, or GStreamer

#### D. Output backend
- HAL direct socket backend
- GStreamer MTK sink backend
- maybe ALSA/BlueALSA fallback backend

This is the key design choice:  
**decoder and output should not be fused together** unless you intentionally rely on GStreamer for both.

---

## Best backend strategy for your use case

Since you specifically want:
- script-launched
- background playback
- independence from Kobo frontend
- music player semantics

I’d define 3 possible backends:

### Backend 1: GStreamer full pipeline
Use `gst-play-1.0` or `gst-launch-1.0`.

**Pros**
- easiest path to seeking
- easiest path to volume
- easiest codec support
- less custom code

**Cons**
- weaker control unless you wrap carefully
- process management can be annoying
- maybe firmware/plugin dependency risk
- harder to get precise state unless you use a proper GStreamer app, not shelling out

**Use if:** Kobo’s GStreamer stack is present and reliable.

---

### Backend 2: Your direct HAL PCM backend
Decode externally, feed PCM to the HAL data socket.

**Pros**
- maximum independence from Nickel
- true background playback
- likely closest to the real device path
- full control over buffering and transport

**Cons**
- you must implement seek, volume, underrun handling, pause semantics
- codec support is separate
- device/firmware fragility

**Use if:** you want full control and the HAL path is stable.

---

### Backend 3: Hybrid backend
Use ffmpeg/mpg123 for decode, and your HAL backend for output.

This is probably the sweet spot.

Pipeline:
- file → ffmpeg decode → PCM → your output engine

**Pros**
- your backend stays simple: just PCM sink
- easier multi-format support
- seeking can be done at decoder level
- volume can be software-scaled before output if needed

**Cons**
- more moving parts
- some latency when reseeking/restarting decode

**This is what I would build first.**

---

## What I would actually implement

## Option A: simplest robust v1
A single daemon process that:
- shells out to `ffmpeg` for decode
- reads PCM from stdout
- streams PCM to your HAL data socket
- controls playback with an internal state machine

That gives you:
- MP3/FLAC/Ogg/AAC support
- background playback
- simple script control
- clean separation of decode vs output

### Seek implementation
On seek:
1. stop current decode subprocess
2. tear down/flush current HAL stream
3. restart `ffmpeg -ss <time> -i file ...`
4. reopen/restart output stream
5. continue feeding PCM

Not elegant, but very workable for music.

---

## Volume control options

You said you need volume control. In your architecture, I’d support volume in this order:

### Preferred: software volume in your daemon
Scale PCM samples before writing to the HAL stream.

For 16-bit stereo PCM:
- multiply each sample by gain
- clamp to `[-32768, 32767]`

**Pros**
- works even if Kobo has no usable mixer API
- consistent across BT/internal output
- scriptable

**Cons**
- can reduce dynamic range at low levels
- no “true system volume”

But for a Kobo music player, this is absolutely fine.

### Optional: backend-native volume later
If you later discover:
- HAL supports volume opcode
- AVRCP absolute volume works
- a mixer exists

Then add it as a backend capability.

For now, **software gain is the right answer**.

---

## Scrubbing / seeking

For music playback, seek does not need to be sample-perfect initially.

### Recommended v1 strategy
Keep:
- `track_duration`
- `playback_position_base`
- `monotonic_start_time`
- paused offset

Position = base + elapsed time while playing.

When seeking:
- update base
- restart decoder from target position
- restart output stream

This is enough for:
- seek forward/back in scripts
- crude scrub bar if you later build a GUI
- resume from saved position

---

## Pause/resume

With your direct backend, don’t try to “pause the socket” unless the protocol clearly supports it.

Instead:

### Pause
- stop feeding PCM
- kill/pause decoder subprocess
- record current position
- close or idle output stream cleanly

### Resume
- reopen decoder from saved position
- restart output stream
- continue

This is more like “stop and resume at same position” than a literal transport pause, but for music it’s acceptable.

If later you discover the control socket supports proper suspend/resume opcodes, add them.

---

## Queue/playlist model

For a music player, I’d build these objects:

### Track
- path
- title/artist/album
- duration
- maybe cover path

### Queue
- list of tracks
- current index
- repeat mode
- shuffle seed/order

### PlaybackSession
- current track
- state
- current position
- target output backend
- volume
- decoder PID
- output state
- error

This keeps logic sane.

---

## Background control model

Since you’re launching from NickelMenu, keep commands tiny and idempotent.

Example shell commands:

```sh
koboaudioctl play /mnt/onboard/Music/foo.mp3
koboaudioctl pause
koboaudioctl resume
koboaudioctl seek_rel 15
koboaudioctl seek_rel -15
koboaudioctl set_volume 65
koboaudioctl next
koboaudioctl status
```

Then NickelMenu entries just call these.

That is much better than NickelMenu trying to own process state.

---

## Minimal daemon design

I’d structure it like this:

### `koboaudiod`
Long-running process.

Modules:
- `ipc_server`
- `player_controller`
- `decoder_ffmpeg`
- `output_hal`
- `library/playlist`
- `state_store`

### `koboaudioctl`
Short-lived client command.

Responsibilities:
- connect to daemon socket
- send command
- print response

### `nm` scripts
Just wrappers calling `koboaudioctl`.

---

## Suggested state machine

Use something simple but explicit:

- `IDLE`
- `LOADING`
- `PLAYING`
- `PAUSED`
- `SEEKING`
- `STOPPING`
- `ERROR`

Transitions:
- play → LOADING → PLAYING
- pause → PAUSED
- resume → LOADING → PLAYING
- seek → SEEKING → LOADING → PLAYING
- stop → IDLE
- decode fail / output fail → ERROR

Explicit states matter a lot once you start doing background control from scripts.

---

## Why not make HAL your whole player?

Because HAL injection solves only:

- where PCM goes

It does not solve:

- codecs
- queueing
- seeking
- metadata
- timekeeping
- volume UX
- command/control
- fault recovery

So architecturally, HAL should be:

> `OutputBackend::write_pcm(...)`

not

> “the app”

---

## My recommended v1 stack for KoboAudio

If I were optimizing for your goals, I’d build:

### Decode
- `ffmpeg` CLI subprocess

### Output
- your direct HAL socket backend

### Control
- standalone daemon + UNIX socket client

### Volume
- software PCM gain in daemon

### Seek
- restart decode at offset

### Frontend
- NickelMenu scripts

That gets you:
- background playback
- independence from Nickel frontend
- multi-format playback
- volume control
- seeking/scrubbing foundation
- no KOReader dependency

---

## What to borrow from audiobook.koplugin conceptually

Even if you don’t use KOReader, copy these ideas:

### 1. Backend probing
At startup, detect:
- HAL backend available?
- GStreamer sink available?
- BlueALSA available?

Then choose best backend.

### 2. Process cleanup
Track child PIDs and clean up stale processes.

### 3. Capability-based behavior
Backend reports:
- supports_seek
- supports_native_volume
- supports_pause
- supports_bt
- supports_background

### 4. Failover
If HAL fails, optionally fall back to GStreamer.

### 5. Position polling/watchers
Keep explicit playback monitoring instead of trusting launch success.

---

## If you want “better than my current solution”

Given what you said, “better” probably means:

- not just raw playback
- but controllable, scriptable, resilient playback

So the better solution is **not replacing HAL with KOReader/plugin code**.

The better solution is:

**your HAL path + a real playback daemon architecture**

That’s the upgrade.

---

## Concrete next step I’d recommend

Implement this first:

### Milestone 1
- `koboaudiod` daemon
- `koboaudioctl` client
- one track play/stop/status
- ffmpeg decode to PCM
- PCM -> HAL backend
- software volume
- seek by restart

Once that works, add:
- pause/resume
- next/prev
- queue
- state file for NickelMenu display
- metadata extraction

---

## A practical file/module layout

Something like:

```text
src/
  daemon/
    main.c
    ipc.c
    state.c
    player.c
    playlist.c
    decoder_ffmpeg.c
    output_hal.c
    output_gst.c
    volume.c
    metadata.c
  cli/
    koboaudioctl.c
scripts/
  start-player.sh
  nm-play.sh
  nm-pause.sh
  nm-next.sh
  nm-volup.sh
```

If you prefer Python or shell for orchestration and C only for HAL output, that’s also reasonable:

```text
playerd.py        # daemon, IPC, playlist, ffmpeg process mgmt
halsink.so/.c     # low-level socket writer
koboaudioctl.py   # client
```

Honestly, that hybrid may be faster to iterate.

---

## Bottom line

For your stated goal — **a standalone Kobo music player launched by NickelMenu, playing in the background independent of Kobo’s frontend** — I would build:

- **daemonized player**
- **ffmpeg decode**
- **your HAL backend**
- **software volume**
- **seek by decoder restart**
- **script/IPC control**

And I would treat `audiobook.koplugin` mainly as inspiration for:
- backend abstraction
- cleanup/fallback
- control/state management

not as the literal implementation to reuse.

If you want, next I can sketch a **very concrete command protocol and process model** for `koboaudiod`/`koboaudioctl`, or even propose a **v1 implementation plan in C vs Python**.

## @KiraGreifeneder

Sure, go ahead. I mainly want it as a music player. at the moment, the HAL stuff does let me play audio in the background independent of whatever Kobo's frontend is doing. That doesn't mean I need to use it though, I want to use whatever is best and might work across different kobo devices. Also, I don't want to use Koreader, I pretty much need to be able to start it from a script through nickelmenu on its own.

*System: User retried message*

## Copilot

Then I’d recommend **not** centering the design on KOReader or on raw HAL injection alone.  
For your use case, the best architecture is probably:

**a script-launchable standalone player app/service with pluggable output backends**, where your current HAL path is just one backend.

Given your constraints:

- **music player**, not just TTS
- must **run in background**
- must be **launchable from NickelMenu/script**
- should work across **different Kobo models**
- should be **independent of KOReader**
- wants **scrubbing, volume, pause/resume, track control**

…the right design is less like `audiobook.koplugin` as a whole product, and more like:

- borrow its **backend-detection/fallback mindset**
- ignore its KOReader UI/runtime assumptions
- keep your **direct HAL solution** as an optional MTK backend
- build a **small standalone control plane** around it

## My recommendation in one sentence

Build a **headless playback daemon + tiny CLI/controller**, started by NickelMenu, with output backends in this priority order:

1. **GStreamer backend** if usable
2. **direct HAL/A2DP socket backend** on MTK Kobo if reliable
3. **BlueALSA backend** on BlueZ-based Kobos
4. optional simple ALSA/local-speaker fallback where available

That gives you portability **and** preserves your current “background independent of Nickel” advantage.

---

# Suggested architecture

## 1. Two-process model

You probably want:

### A. `koboaudiod`
Long-running daemon that owns playback state.

Responsibilities:
- current queue/playlist
- current track
- play/pause/stop
- seek position
- volume
- output backend selection
- decoder process/pipeline ownership
- background persistence
- state file / IPC socket

### B. `koboaudioctl`
Small command-line controller invoked from NickelMenu/scripts.

Examples:
```sh
koboaudioctl play /mnt/onboard/Music/foo.mp3
koboaudioctl pause
koboaudioctl resume
koboaudioctl seek +30
koboaudioctl seek 120
koboaudioctl next
koboaudioctl prev
koboaudioctl volume 70
koboaudioctl status
```

NickelMenu should call the controller, **not** the player internals directly.

Why this matters:
- NickelMenu actions are short-lived
- playback must survive menu exit
- you want one persistent owner of the audio session

---

## 2. Backend abstraction

Define a backend interface something like:

```text
init()
start(track, start_pos, volume)
stop()
pause()
resume()
seek(seconds)
set_volume(percent)
get_position()
get_duration()
is_playing()
capabilities()
shutdown()
```

Not every backend will support every feature natively.

So also expose capability flags:

```text
can_seek
can_pause
can_resume
can_volume
reports_position
supports_background
bluetooth_only
```

This lets the daemon adapt behavior per backend.

---

# Recommended backends

## Backend 1: GStreamer
**Best general-purpose candidate if present and stable.**

Why:
- supports many music formats
- seeking is much easier
- volume is easier
- decoding is built in
- one engine can handle MP3/OGG/FLAC/WAV/etc.
- can often target Kobo-specific sink if needed

This is the closest thing to a “real media player backend.”

### Ideal use
On devices where:
- `gst-launch-1.0` or `gst-play-1.0` exists
- `playbin`/decodebin works
- the needed audio sink exists:
  - `mtkbtmwrpcaudiosink`
  - or `autoaudiosink`
  - or ALSA sink

### Why better than raw HAL for music
Your HAL path solves “how do I inject PCM,” but a music player also needs:
- decoding
- demuxing
- seek in compressed formats
- buffering
- state transitions
- track changes

GStreamer already does all of that.

### Best form
If possible, don’t shell out to ad hoc `gst-launch` commands for every action forever.  
Long term, better options are:

- a managed `gst-launch` subprocess with restart-on-seek
- or a small native helper around GStreamer
- or even D-Bus/IPC around a dedicated player wrapper

For a first standalone version, though, a subprocess model is fine.

---

## Backend 2: Direct Kobo HAL/A2DP socket
**Best experimental MTK-specific backend.**

Keep your current solution, because it has an important property:

- it already proves you can play audio in the background independently of Nickel

That’s valuable.

But I would position it as:

- **MTK-specific**
- **lower-level**
- **possibly best for raw PCM streaming**
- **not the default architecture unless it becomes easier than GStreamer**

### Use it when
- GStreamer is unavailable or unstable
- Kobo’s exposed sink path is broken
- you need very direct control over BT output routing
- you are okay owning decode + buffering + timing

### Hard parts you’ll have to own
If this is the main backend, you likely need:
- decoder pipeline before PCM injection
- resampling
- channel conversion
- seek semantics
- pause/resume buffering behavior
- possibly your own software volume

That’s a lot more surface area than “start audio.”

So: very useful backend, but not ideal as your only one.

---

## Backend 3: BlueALSA
**Good cross-device Bluetooth fallback for BlueZ Kobos.**

This is the most interesting thing `audiobook.koplugin` contributes conceptually.

Instead of reverse engineering each device’s BT path, you can sometimes create a more standard route:

- BlueALSA daemon
- ALSA `bluealsa` PCM
- normal playback tools feed ALSA
- BlueZ forwards to BT headphones

That may be more portable across some Kobo models than Kobo’s proprietary MTK internals.

### Why useful
- cleaner than direct HAL poking
- easier to pair with standard decoders/players
- supports “normal Linux audio app” patterns better

### Downside
- setup complexity
- dependency bundling
- may vary by Kobo generation/firmware

Still, it’s worth having as a backend.

---

## Backend 4: simple local-output fallback
If some models have a speaker or standard ALSA output, support that too.

Even if your main goal is BT music, local playback fallback helps with:
- debugging
- device compatibility
- validation on unknown models

---

# What I would not do

## Don’t make NickelMenu directly spawn one-shot players
Bad pattern:
```sh
menu_item :play song:exec:/mnt/onboard/.adds/koboaudio/play.sh /mnt/onboard/Music/foo.mp3
```

Why bad:
- no persistent state
- harder pause/resume
- harder seek
- no clean queue management
- multiple accidental instances

Instead:
```sh
menu_item :play song:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl play /mnt/onboard/Music/foo.mp3
```

The daemon should already be running or auto-start if missing.

---

## Don’t tie architecture to KOReader concepts
Good to borrow:
- backend detection
- fallback hierarchy
- cleanup discipline

Bad to copy:
- UIManager assumptions
- KOReader widget lifecycle
- Lua scheduling model as your core runtime

You want something that works from NickelMenu and shell.

---

## Don’t make raw PCM injection your only interface
For music playback, file/stream abstraction should be above transport.

The daemon should think in terms of:
- tracks
- streams
- decoded PCM source
- transport backend

not just:
- “send bytes to socket”

---

# Control plane design

## IPC
Use a simple local socket or FIFO.

Best options:
- UNIX domain socket
- named pipe pair
- tiny HTTP server on localhost only
- D-Bus if you really want, but probably overkill

I’d use a **UNIX socket**.

Example:
- daemon listens on `/tmp/koboaudio.sock`
- controller sends JSON or simple commands

Example commands:
```json
{"cmd":"play","path":"/mnt/onboard/Music/foo.mp3"}
{"cmd":"pause"}
{"cmd":"resume"}
{"cmd":"seek","pos":120}
{"cmd":"seek_rel","delta":30}
{"cmd":"volume","value":75}
{"cmd":"status"}
{"cmd":"next"}
```

## Persistent state
Keep a small JSON file:
```json
{
  "track": "/mnt/onboard/Music/foo.mp3",
  "position": 183.4,
  "duration": 241.9,
  "paused": false,
  "volume": 72,
  "backend": "gstreamer-mtk",
  "queue": [...]
}
```

That lets you:
- rebuild UI/status
- survive controller restarts
- maybe recover after crashes

---

# Seeking / scrubbing strategy

This is the biggest architecture decision.

## If using GStreamer
Use native seek if possible.

That’s your best case:
- precise-ish scrubbing
- duration reporting
- position queries
- minimal reinvention

## If using HAL direct injection
Treat seeking as:

1. stop playback
2. seek in decoder to target position
3. restart PCM stream from new point
4. re-prime output path
5. update daemon state

This is workable, but you’ll need a separate decoder layer.

## Decoder recommendation
Use an existing decoder frontend, not your own codec parser.

Possible choices:
- `ffmpeg`
- `mpg123`
- `ogg123`
- `flac` decoder
- GStreamer decode pipeline

For maintainability, one general decoder path is better:
- either GStreamer for decode+output
- or ffmpeg for decode-to-PCM + backend-specific output

If your direct HAL backend stays important, **ffmpeg decode → PCM → backend output** is a solid pattern.

---

# Volume control strategy

Volume needs to be abstracted because each backend will differ.

## Best model: software volume in daemon + backend native volume when available

### Option A: backend-native volume
Use if available:
- GStreamer volume property
- ALSA mixer on local device
- AVRCP absolute volume if exposed

### Option B: software pre-scale PCM
Needed for direct HAL backend and as fallback.

Advantages:
- consistent across backends
- independent of device mixer support

Disadvantages:
- possible clipping if naive
- must process PCM stream

For a music player, software volume is acceptable and probably necessary anyway.

So I’d recommend:
- always support software gain
- optionally bypass or combine with native backend volume

---

# Pause/resume

## GStreamer
Likely native and good.

## Direct HAL backend
Probably emulate with either:
- stop while remembering decoder position, then restart on resume
- or keep decoder paused and backend open if protocol permits

For audiobooks, restart-on-resume is okay.  
For music, it’s less elegant but still acceptable if resume latency is short.

---

# Track position / duration

You need this for scrubbing UI and status.

## Best case
Backend reports:
- duration
- current playback position

## Fallback
Estimate from:
- track duration from ffprobe/ffmpeg/metadata
- monotonic start time - paused time - seeks

This fallback is imperfect but often enough for a simple UI.

---

# Queue / playlist model

For a music player, queue management should be in the daemon, not backend.

State:
- current index
- list of paths
- shuffle mode
- repeat mode
- history stack

The backend only plays one resolved track at a time.

---

# Launch / lifecycle with NickelMenu

## Startup model
NickelMenu command should do something like:
```sh
/mnt/onboard/.adds/koboaudio/koboaudioctl ensure-running
/mnt/onboard/.adds/koboaudio/koboaudioctl play "/mnt/onboard/Music/foo.mp3"
```

`ensure-running`:
- checks socket/pid
- starts daemon if absent
- returns quickly

## Daemon start
Could be:
```sh
nohup /mnt/onboard/.adds/koboaudio/koboaudiod >/tmp/koboaudiod.log 2>&1 &
```

The daemon then:
- detects backend
- restores state
- waits for commands

---

# Cross-device backend priority I’d use

For your goals, I’d prioritize:

## 1. GStreamer with Kobo sink if available
Example concept:
- use `playbin` or equivalent
- sink = `mtkbtmwrpcaudiosink` if present
- else default sink

Best for:
- music formats
- seek
- pause
- volume
- fewer custom pieces

## 2. BlueALSA + standard player
Best for devices where BT routing is BlueZ-ish and can be standardized.

## 3. Direct HAL socket backend
Best experimental MTK fallback where proprietary path works but standard plumbing is lacking.

## 4. Local ALSA/speaker fallback
Useful for debugging and broad compatibility.

---

# Concrete modular design

I’d split it like this:

## `daemon/`
- `main`
- IPC server
- state machine
- queue manager

## `backends/`
- `gst_backend`
- `bluealsa_backend`
- `hal_backend`
- `alsa_backend`

## `decoders/`
Maybe none if GStreamer handles decode.  
Otherwise:
- `ffmpeg_decoder`

## `platform/`
- Kobo model detection
- firmware probing
- sink detection
- process cleanup helpers

## `ctl/`
- CLI parser
- socket client

## `ui/optional`
Maybe later a tiny framebuffer UI or web UI, but not required now.

---

# Best implementation stack

Since you need scriptability and Kobo friendliness, I’d likely choose one of these:

## Option A: shell + helper binaries
Good for prototyping, but will get messy for IPC/state/seek.

## Option B: Lua
Possible, especially if you like the KOReader ecosystem style, but without KOReader runtime it’s less attractive unless you already have a Lua environment packaged.

## Option C: Python
Convenient, but maybe heavier / less ideal depending on Kobo availability.

## Option D: small C/C++ daemon + shell controller
Best for robustness, but more work.

## Option E: hybrid
- shell controller
- Python or C daemon
- helper subprocesses for playback

For your current stage, I’d pick **hybrid**:
- simple shell `koboaudioctl`
- one real daemon in **C or Python**, depending on what you can package comfortably

If Python isn’t consistently available or convenient on Kobo, then C is the safer long-term route.

---

# What to borrow from audiobook.koplugin specifically

Borrow these ideas:

## 1. Runtime probing
At startup, detect:
- `gst-launch-1.0`
- `gst-play-1.0`
- `gst-inspect-1.0 mtkbtmwrpcaudiosink`
- `aplay`
- BlueALSA presence
- your HAL socket path viability

## 2. Fallback order
Don’t assume one backend.

## 3. Aggressive cleanup
Their notes about orphan `gst-launch` processes and exclusive sockets are very relevant.  
Your daemon should always:
- track child PIDs
- kill stale wrappers
- reclaim sockets/resources on restart

## 4. Bluetooth-aware backend choice
Backend choice should depend on:
- BT connected?
- sink available?
- backend actually routable?

## 5. Separate transport from UI
Very important for your standalone use.

---

# My actual recommendation

If I were choosing what to build next for **KoboAudio as a music player**, I would do this:

### Phase 1
- create `koboaudiod`
- create `koboaudioctl`
- implement `status/play/pause/resume/stop`
- implement **GStreamer backend first**
- keep your current **HAL backend** behind a backend interface

### Phase 2
- add seek/scrub
- add software volume
- add queue/playlist
- add state persistence

### Phase 3
- add BlueALSA backend
- improve cross-device probing
- add better track metadata and duration reporting

### Phase 4
- optional simple UI / external remote control

---

# Bottom line

For a **script-launched Kobo music player**, the best design is:

- **standalone daemon**
- **CLI controller**
- **backend abstraction**
- **GStreamer-first when available**
- **your HAL path as a device-specific fallback/backend**
- **KOReader not required at all**

So yes: your current HAL solution is valuable, but I would treat it as **one output module**, not the entire player architecture.

If you want, next I can sketch:
1. a **minimal command set and IPC protocol**, or  
2. a **backend interface plus fallback detection logic** in concrete pseudocode.

## @KiraGreifeneder

how do I know if any of these already exist?

gst-launch-1.0
gst-play-1.0
gst-inspect-1.0 mtkbtmwrpcaudiosink
aplay
BlueALSA presence
your HAL socket path viability

## Copilot

Use a single probe script from NickelMenu or over SSH. For each item, check existence plus a quick viability test.

```sh name=probe_audio_stack.sh
#!/bin/sh
set -u

echo "=== Kobo audio stack probe ==="
echo "Date: $(date)"
echo

have() { command -v "$1" >/dev/null 2>&1; }

section() {
  echo
  echo "--- $1 ---"
}

section "Basic binaries"

for cmd in gst-launch-1.0 gst-play-1.0 gst-inspect-1.0 aplay; do
  if have "$cmd"; then
    echo "$cmd: YES ($(command -v "$cmd"))"
  else
    echo "$cmd: NO"
  fi
done

section "GStreamer MTK sink"

if have gst-inspect-1.0; then
  if gst-inspect-1.0 mtkbtmwrpcaudiosink >/tmp/mtk_sink_probe.txt 2>&1; then
    echo "mtkbtmwrpcaudiosink: YES"
    head -20 /tmp/mtk_sink_probe.txt
  else
    echo "mtkbtmwrpcaudiosink: NO"
    head -20 /tmp/mtk_sink_probe.txt
  fi
else
  echo "gst-inspect-1.0 missing, cannot probe sink"
fi

section "ALSA / aplay"

if have aplay; then
  echo "[aplay -l]"
  aplay -l 2>&1
  echo
  echo "[aplay -L]"
  aplay -L 2>&1
else
  echo "aplay missing"
fi

section "BlueALSA"

BLUEALSA_FOUND=0

if pgrep -a bluealsa >/tmp/bluealsa_ps.txt 2>/dev/null; then
  echo "bluealsa daemon running:"
  cat /tmp/bluealsa_ps.txt
  BLUEALSA_FOUND=1
else
  echo "bluealsa daemon not running"
fi

for p in \
  /usr/bin/bluealsa \
  /usr/sbin/bluealsa \
  /usr/local/bin/bluealsa \
  /mnt/onboard/.adds/*/bluealsa/bin/bluealsa
do
  for f in $p; do
    [ -x "$f" ] || continue
    echo "bluealsa binary found: $f"
    BLUEALSA_FOUND=1
  done
done

if have aplay; then
  if aplay -L 2>/dev/null | grep -i '^bluealsa\>' >/tmp/bluealsa_pcm.txt 2>/dev/null; then
    echo "bluealsa PCM advertised by ALSA:"
    cat /tmp/bluealsa_pcm.txt
    BLUEALSA_FOUND=1
  else
    echo "no bluealsa PCM in aplay -L"
  fi
fi

if [ -f /etc/asound.conf ] && grep -i bluealsa /etc/asound.conf >/tmp/bluealsa_conf.txt 2>/dev/null; then
  echo "/etc/asound.conf mentions bluealsa:"
  cat /tmp/bluealsa_conf.txt
  BLUEALSA_FOUND=1
else
  echo "/etc/asound.conf has no bluealsa stanza (or file missing)"
fi

if [ "$BLUEALSA_FOUND" -eq 1 ]; then
  echo "BlueALSA presence: LIKELY"
else
  echo "BlueALSA presence: NO EVIDENCE"
fi

section "Bluetooth / MTK stack clues"

if command -v dbus-send >/dev/null 2>&1; then
  echo "[D-Bus managed objects grep]"
  dbus-send --system --print-reply \
    --dest=com.kobo.mtk.bluedroid / \
    org.freedesktop.DBus.ObjectManager.GetManagedObjects 2>&1 \
    | grep -i 'bluez\|media\|player\|transport\|avrcp\|device' | head -50
else
  echo "dbus-send missing"
fi

echo
echo "[/proc/net/unix grep for audio/control sockets]"
grep -i 'mtk\|a2dp\|audio\|btmw\|rpc' /proc/net/unix 2>/dev/null || echo "no obvious socket names"

section "HAL socket path viability"

# This is heuristic. Adjust once you know the exact socket name(s).
SOCKET_MATCHES=$(grep -i 'mtk\|a2dp\|audio\|btmw\|rpc' /proc/net/unix 2>/dev/null | wc -l | tr -d ' ')
echo "candidate socket entries: $SOCKET_MATCHES"

if [ "$SOCKET_MATCHES" -gt 0 ]; then
  echo "HAL/socket path viability: POSSIBLE"
  echo "You still need the exact control socket protocol/opcodes."
else
  echo "HAL/socket path viability: NO OBVIOUS SOCKET EXPOSED"
fi

section "Recommendation"

if have gst-inspect-1.0 && gst-inspect-1.0 mtkbtmwrpcaudiosink >/dev/null 2>&1; then
  echo "Best current candidate: GStreamer MTK sink backend"
elif have aplay && aplay -L 2>/dev/null | grep -iq '^bluealsa\>'; then
  echo "Best current candidate: BlueALSA backend"
elif [ "$SOCKET_MATCHES" -gt 0 ]; then
  echo "Best current candidate: direct HAL/socket backend"
elif have aplay; then
  echo "Best current candidate: plain ALSA/local output backend"
else
  echo "No obvious playback backend found"
fi

rm -f \
  /tmp/mtk_sink_probe.txt \
  /tmp/bluealsa_ps.txt \
  /tmp/bluealsa_pcm.txt \
  /tmp/bluealsa_conf.txt
```

## What each test means

### `gst-launch-1.0`
Checks whether you can run GStreamer pipelines at all.

Quick manual test:
```sh
command -v gst-launch-1.0
```

If present, that’s promising.

---

### `gst-play-1.0`
Checks whether you have the higher-level GStreamer player helper.

Quick manual test:
```sh
command -v gst-play-1.0
```

Nice to have, not essential.

---

### `gst-inspect-1.0 mtkbtmwrpcaudiosink`
This is the most important MTK/Kobo probe.

Manual:
```sh
gst-inspect-1.0 mtkbtmwrpcaudiosink
```

Interpretation:
- **success**: Kobo likely exposes the MediaTek BT audio sink through GStreamer
- **failure**: you probably need BlueALSA, ALSA, or your HAL backend

You can also do the shortest check:
```sh
gst-inspect-1.0 mtkbtmwrpcaudiosink >/dev/null 2>&1 && echo yes || echo no
```

---

### `aplay`
Checks whether ALSA userland is present.

Manual:
```sh
command -v aplay
aplay -l
aplay -L
```

Interpretation:
- `aplay` exists + useful devices listed = ALSA route may exist
- `aplay` exists but no devices = probably not enough by itself
- `bluealsa` appears in `aplay -L` = strong BlueALSA sign

---

### BlueALSA presence
BlueALSA is not just “is there a command”; it’s usually a combination of:

- `bluealsa` daemon exists or runs
- ALSA advertises a `bluealsa` PCM
- `/etc/asound.conf` contains `bluealsa`
- BT stack is BlueZ-based enough for it to matter

Manual checks:
```sh
pgrep -a bluealsa
aplay -L | grep -i '^bluealsa'
grep -i bluealsa /etc/asound.conf
find / -name bluealsa 2>/dev/null
```

Interpretation:
- all 3 positive = BlueALSA probably usable
- only binary found = not enough
- PCM listed in `aplay -L` = best sign

---

### HAL socket path viability
This one is different: there usually isn’t a single “yes/no” command unless you already know the exact socket name and protocol.

You’re really checking:
1. does the relevant daemon/socket exist?
2. does it appear when BT is enabled/connected?
3. can you talk to it with the expected control opcodes?

The best quick heuristic is:
```sh
grep -i 'mtk\|a2dp\|audio\|btmw\|rpc' /proc/net/unix
```

Also:
```sh
ps | grep -i 'bluetooth\|audio\|mtk\|bt'
```

Interpretation:
- visible abstract/local sockets with promising names = likely viable
- no such sockets = either wrong time/state or wrong stack

For your specific KoboAudio-style path, the real proof is not just socket presence, but:

- BT device connected
- control socket accepts START opcode
- returns success/ack
- spawns or enables data socket
- data stream produces audible playback

That’s the only definitive viability test.

---

## Practical order I’d use

On a Kobo, I’d check in this order:

```sh
gst-inspect-1.0 mtkbtmwrpcaudiosink >/dev/null 2>&1 && echo "use gstreamer-mtk"
```

If no:

```sh
aplay -L | grep -iq '^bluealsa' && echo "use bluealsa"
```

If no:

```sh
grep -i 'mtk\|a2dp\|audio\|btmw\|rpc' /proc/net/unix
```

If that looks promising:
- test your HAL code

If not:
- you likely only have plain ALSA/local or no usable route discovered yet.

---

## Best real-world test
Presence checks are good, but **a 2-second playback test** is better.

For example:

### GStreamer sink test
```sh
gst-launch-1.0 filesrc location=/path/to/test.wav ! wavparse ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

### BlueALSA test
```sh
aplay -D bluealsa /path/to/test.wav
```

### HAL test
Use your current direct injector with a known-good PCM sample.

That gives you “installed” vs “actually works.”

---

## My recommendation
Make one probe script like above and save its output per device. Then you can classify each Kobo model into:

- `gstreamer-mtk`
- `bluealsa`
- `hal-direct`
- `alsa-local`
- `unknown`

That classification can directly drive backend selection in your future daemon.

If you want, I can next turn this into:
1. a **NickelMenu-friendly one-liner set**, or  
2. a **backend auto-detection shell script** that outputs just one backend name.

## @KiraGreifeneder

```
[root@kobo onboard]# ./probe_audio_stack.sh
=== Kobo audio stack probe ===
Date: Tue May 19 17:18:23 CEST 2026


--- Basic binaries ---
gst-launch-1.0: YES (/bin/gst-launch-1.0)
gst-play-1.0: YES (/bin/gst-play-1.0)
gst-inspect-1.0: YES (/bin/gst-inspect-1.0)
aplay: YES (/bin/aplay)

--- GStreamer MTK sink ---
mtkbtmwrpcaudiosink: YES
Factory Details:
  Rank                     none (0)
  Long-name                Bluedroid Audio Sink
  Klass                    Audio/Sink
  Description              Output audio to MTK Bluetooth Middleware RPC API
  Author                   George Talusan <george.talusan@rakuten.com>

Plugin Details:
  Name                     mtkbtmwrpc
  Description              Kobo's MTK Bluetooth Middleware RPC API plugin
  Filename                 /lib/gstreamer-1.0/libgstmtkbtmwrpc.so
  Version                  1.0.0
  License                  Proprietary
  Source module            gst-mtkbtmwrpc
  Binary package           Kobo
  Origin URL               https://www.kobo.com/

GObject
 +----GInitiallyUnowned
       +----GstObject

--- ALSA / aplay ---
[aplay -l]
aplay: device_list:272: no soundcards found...

[aplay -L]
null
    Discard all samples (playback) or generate zero samples (capture)

--- BlueALSA ---
bluealsa daemon not running
no bluealsa PCM in aplay -L
/etc/asound.conf has no bluealsa stanza (or file missing)
BlueALSA presence: NO EVIDENCE

--- Bluetooth / MTK stack clues ---
[D-Bus managed objects grep]
         object path "/org/bluez/hci0"
               string "org.bluez.Device1"

[/proc/net/unix grep for audio/control sockets]
ef2eb9c0: 00000002 00000000 00010000 0001 01  5490 /tmp//mtk_bt_service
ee43a680: 00000003 00000000 00000000 0001 03  4004 /tmp//mtk_bt_service
ee43a3c0: 00000003 00000000 00000000 0001 03  3993 /tmp//mtk_bt_service

--- HAL socket path viability ---
candidate socket entries: 3
HAL/socket path viability: POSSIBLE
You still need the exact control socket protocol/opcodes.

--- Recommendation ---
Best current candidate: GStreamer MTK sink backend
```
here's my output, so I guess i can work with gstreamer? How? can I test whether it even works with my headphones

## Copilot

Yes — **you should absolutely try GStreamer first** on this device.

Your probe result is unusually good:

- `gst-launch-1.0` exists
- `gst-play-1.0` exists
- `gst-inspect-1.0` exists
- `mtkbtmwrpcaudiosink` exists and is clearly Kobo’s proprietary MTK Bluetooth sink
- ALSA is basically useless here (`no soundcards found`)
- no BlueALSA evidence
- your HAL socket path is still possible, but **GStreamer is the best exposed integration point right now**

So the next question is not “can I work with GStreamer?”  
It’s: **does `mtkbtmwrpcaudiosink` actually send audible audio to your connected headphones?**

## Yes: test it directly

You want a **known-good short WAV test**.

---

# Fastest test if you already have a WAV file

If you have a WAV file on the Kobo, run:

```sh
gst-launch-1.0 filesrc location="/path/to/test.wav" ! wavparse ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

If that works, you should hear audio in the headphones.

---

# If you do not already have a test WAV file

You already showed a script in `audiobook.koplugin` that generates a test tone and plays it through `mtkbtmwrpcaudiosink`. That is actually a very good sign, because it is almost exactly the test you need.

From the earlier repo snippet, their script does this:

- generate a 3-second WAV tone
- run:
  ```sh
  gst-launch-1.0 filesrc location="$TESTFILE" \
      ! wavparse \
      ! audioconvert ! audioresample \
      ! "audio/x-raw,format=S16LE,rate=48000,channels=2" \
      ! mtkbtmwrpcaudiosink
  ```

That is probably the best first test on your device.

---

# I’d use this exact test script

```sh name=test_mtk_bt_audio.sh
#!/bin/sh
set -u

echo "=== MTK BT audio sink test ==="

if ! command -v gst-launch-1.0 >/dev/null 2>&1; then
  echo "gst-launch-1.0 missing"
  exit 1
fi

if ! gst-inspect-1.0 mtkbtmwrpcaudiosink >/dev/null 2>&1; then
  echo "mtkbtmwrpcaudiosink missing"
  exit 1
fi

TESTFILE=/tmp/bt_test_tone.wav

echo "Generating 3-second 440 Hz WAV tone..."

if command -v python3 >/dev/null 2>&1; then
python3 - <<'PY'
import struct, math
sr = 16000
dur = 3
freq = 440
amp = 16000
n = sr * dur
path = "/tmp/bt_test_tone.wav"
data_size = n * 2
hdr = struct.pack('<4sI4s4sIHHIIHH4sI',
    b'RIFF', 36 + data_size, b'WAVE',
    b'fmt ', 16, 1, 1, sr, sr * 2, 2, 16,
    b'data', data_size)
with open(path, 'wb') as f:
    f.write(hdr)
    for i in range(n):
        s = int(amp * math.sin(2 * math.pi * freq * i / sr))
        f.write(struct.pack('<h', s))
print(path)
PY
else
  echo "python3 missing; cannot generate test WAV automatically"
  exit 1
fi

echo "Playing test tone..."
echo "If your headphones are connected and this sink works, you should hear a 3-second tone."

gst-launch-1.0 \
  filesrc location="$TESTFILE" \
  ! wavparse \
  ! audioconvert \
  ! audioresample \
  ! "audio/x-raw,format=S16LE,rate=48000,channels=2" \
  ! mtkbtmwrpcaudiosink

RET=$?

echo "gst-launch exit code: $RET"

rm -f "$TESTFILE"

if [ "$RET" -eq 0 ]; then
  echo "Pipeline completed."
else
  echo "Pipeline failed."
fi
```

Run:
```sh
sh ./test_mtk_bt_audio.sh
```

---

# How to interpret results

## Case 1: You hear the tone
Best-case outcome.

That means:
- your headphones are connected
- the sink works
- GStreamer can be your primary backend
- you probably do **not** need direct HAL injection for basic playback on this device

That would strongly push your architecture toward:
- **GStreamer-first backend**
- HAL as fallback/experimental

## Case 2: Pipeline succeeds, but no sound
That usually means one of:
- BT device connected but not actually selected for audio
- sink/plugin accepts pipeline but routing is not active
- volume issue
- BT middleware state issue
- headset profile issue

Then we’d want to test:
- reconnect headphones
- kill old `gst-launch` processes
- inspect BT middleware state
- maybe retry after power-cycling Bluetooth

## Case 3: Pipeline hangs forever
Could mean:
- sink is waiting on a connection/state
- exclusive socket is wedged
- middleware is stuck

Then:
```sh
killall -9 gst-launch-1.0
```
and retry.

## Case 4: Immediate pipeline error
Then we inspect the GStreamer error output. That will tell us whether:
- `wavparse` failed
- sink negotiation failed
- sink could not open underlying transport

---

# Before testing: verify the headphones are actually connected

Your probe only showed some DBus objects, not necessarily that the device is actively connected for audio.

If you know the MAC address of your headphones, test:

```sh
dbus-send --system --print-reply \
  --dest=com.kobo.mtk.bluedroid \
  /org/bluez/hci0/dev_XX_XX_XX_XX_XX_XX \
  org.freedesktop.DBus.Properties.Get \
  string:"org.bluez.Device1" string:"Connected"
```

Replace `XX_...` with underscores instead of colons.

If you don’t know the path, dump managed objects:

```sh
dbus-send --system --print-reply \
  --dest=com.kobo.mtk.bluedroid / \
  org.freedesktop.DBus.ObjectManager.GetManagedObjects 2>/dev/null | less
```

Look for your device and `Connected`.

If you already know your HAL test works with the same headphones, then this is probably okay and you can skip straight to audio testing.

---

# Even easier test: use gst-play-1.0

Since `gst-play-1.0` exists, you can also try:

```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink /path/to/test.wav
```

That’s nice because `gst-play-1.0` is closer to how a real music player would behave.

If that works, it’s a very strong sign you can base your standalone music player on GStreamer.

---

# Best next tests in order

I’d do exactly this:

## 1. Tone test with `gst-launch-1.0`
Use the script above.

## 2. Real file playback with `gst-play-1.0`
If you have an MP3/OGG/FLAC:
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/mnt/onboard/Music/test.mp3"
```

This matters because for a music player, decode support is important, not just WAV output.

## 3. Seek test
If `gst-play-1.0` supports it on your build, try interactive controls or restart from a position later. Even if the CLI is limited, successful playback already tells us a lot.

## 4. Backgrounding test
Start playback, return to Nickel, and see whether audio continues.

This is especially important for your use case.

---

# What success would mean architecturally

If these tests work, the best architecture for your project becomes much clearer:

## Recommended backend order on this device
1. **GStreamer + `mtkbtmwrpcaudiosink`**
2. direct HAL backend as fallback / experimental backend

Why:
- GStreamer already solves decode
- likely easier seeking
- likely easier playlist/music handling
- much less custom transport code
- still uses Kobo’s exposed MTK audio path

That would mean your daemon can think at the level of:
- play file
- pause
- resume
- seek
- stop
- query state

instead of:
- decode file
- resample PCM
- negotiate sink
- manage transport buffer

---

# Important subtle point

Using GStreamer here does **not** mean abandoning your HAL work completely.

It probably means:

- Kobo exposes a **supported-enough userland wrapper** around the same proprietary MTK Bluetooth path
- your HAL work is still useful as:
  - fallback
  - reverse-engineering reference
  - lower-level recovery path if GStreamer is flaky

So your HAL project was still absolutely worthwhile — it helped prove the platform can do background audio independently.

But if `mtkbtmwrpcaudiosink` works, it may be the better practical interface.

---

# What I’d like you to try next

Please run one or both of these:

## Test 1
```sh
sh ./test_mtk_bt_audio.sh
```

## Test 2
If you have a real music file:
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/path/to/song.mp3"
```

Then tell me:
- whether you heard audio
- whether playback continued in background
- whether the process exited cleanly or hung
- any stderr output

If you want, after that I can help you build a **minimal standalone GStreamer-based Kobo music daemon** that you can launch from NickelMenu.

## @KiraGreifeneder

I dont want to use python, I'd rather keep the project rust-based and don't want to install python onto my kobo rn

## Copilot

That’s fine — **you don’t need Python at all**.  
And yes, if your project is Rust-based, I’d keep it that way.

For now, you only need a **shell-only probe/playback test**, and longer term a **Rust daemon/controller** is a very good fit for this project.

## Shell-only audio test

If you don’t want Python on the Kobo, the easiest test is just to use an existing audio file you already have.

### Test with a real file
If you have any WAV/MP3/OGG/FLAC on the device:

```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/mnt/onboard/path/to/file.mp3"
```

or for a WAV:

```sh
gst-launch-1.0 filesrc location="/mnt/onboard/path/to/file.wav" ! wavparse ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

That’s enough to answer the important question:
**does Kobo’s GStreamer BT sink actually play to your headphones?**

---

## If you don’t already have a WAV file

You can generate a simple WAV **without Python** using shell tools, but it’s annoying and less reliable.  
So the best no-Python option is just:

- copy one short test MP3/WAV to the Kobo
- try `gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink ...`

That is actually a better real-world test for your music-player goal anyway.

---

## Best no-Python tests to run

### 1. Real file playback
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/mnt/onboard/Music/test.mp3"
```

### 2. Background behavior
Start the command, then leave it running and return to Nickel.  
See whether:
- audio keeps playing
- BT stays connected
- process survives normally

### 3. Kill stale pipelines if needed
If it gets stuck:
```sh
killall -9 gst-play-1.0 gst-launch-1.0
```

### 4. Optional WAV-specific low-level test
```sh
gst-launch-1.0 filesrc location="/mnt/onboard/Music/test.wav" ! wavparse ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

---

# Rust is actually a good choice here

For your intended architecture, Rust fits very well:

- daemon/background service
- Unix socket IPC
- process management
- state machine
- backend abstraction
- less messy than shell for playback lifecycle
- smaller/cleaner than dragging in Python

## What I’d build in Rust

### `koboaudiod`
A long-running daemon:
- backend detection
- playback control
- queue
- seek state
- volume
- status reporting
- child process ownership

### `koboaudioctl`
A tiny CLI:
- `play`
- `pause`
- `resume`
- `seek`
- `status`
- `volume`
- `next`
- `prev`

### Backends
Rust traits like:

```rust
trait AudioBackend {
    fn start(&mut self, path: &str, start_pos: Option<f64>) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn seek(&mut self, pos: f64) -> Result<()>;
    fn set_volume(&mut self, volume: u8) -> Result<()>;
    fn status(&self) -> PlaybackStatus;
}
```

Implementations:
- `GStreamerMtkBackend`
- `HalDirectBackend`
- later maybe `BlueAlsaBackend`

---

# Practical recommendation now

Since you don’t want Python, do this next:

## Use an existing file and test:
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/mnt/onboard/Music/test.mp3"
```

If that works, then your likely plan becomes:

### Primary backend
- Rust wrapper around `gst-play-1.0` or `gst-launch-1.0`

### Secondary backend
- your direct HAL backend

That is probably the best near-term path.

---

# Near-term architecture choice

You have two realistic Rust approaches:

## Option A: spawn system GStreamer tools
Rust daemon launches:
- `gst-play-1.0`
- or `gst-launch-1.0`

Pros:
- fastest path
- no Rust GStreamer cross-compilation pain initially
- leverages Kobo’s built-in binaries

Cons:
- less precise control
- seeking/status may require subprocess tricks
- pause/resume may be awkward depending on tool mode

## Option B: Rust + GStreamer library bindings
Rust links to GStreamer libs directly.

Pros:
- better control
- true seek/pause/volume APIs
- cleaner player implementation

Cons:
- much harder on Kobo
- cross-compilation/integration pain
- depends on available dev libs/ABI compatibility

### My advice:
Start with **Option A**.  
Don’t overengineer yet.

If Kobo’s built-in `gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink` works well, your first working player can absolutely just be a Rust daemon supervising subprocesses.

---

# So: no Python needed

Your next step is just:

```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "/mnt/onboard/Music/test.mp3"
```

or with another format you already have.

If you want, after you test that, I can help you design a **minimal Rust crate structure** for:

- `koboaudiod`
- `koboaudioctl`
- backend trait
- GStreamer subprocess backend
- HAL fallback backend

## @KiraGreifeneder

ok, you can design a minimal rust architecture for me if you also tell me how seeking would work, because i don't see gst play or gst launch having any options for that. Ands yes, gst-play-1.0 did successfully play an audio file through my headphones

## Copilot

Great — **make GStreamer your primary backend**.  
Since `gst-play-1.0` successfully played through your headphones, that is the best practical playback path on this Kobo right now.

And you’re right: if you use `gst-play-1.0` or `gst-launch-1.0` as plain subprocesses, **seeking is the awkward part**. That is the main architectural issue.

# The important conclusion

You now have two distinct backend categories:

## 1. GStreamer subprocess backend
Best for:
- reliable playback
- codec support
- background playback
- easier initial implementation

Weak at:
- precise control if treated as an opaque CLI process

## 2. Your direct HAL backend
Best for:
- owning transport
- possibly easier custom seek semantics if *you* also own the decoder path

Weak at:
- lots of custom decode/buffer/control work

So for a **minimal Rust architecture**, I’d recommend:

- build the player as a daemon/controller system
- define a backend trait
- implement **GStreamer subprocess backend first**
- design seek as a backend capability
- initially support seek via **restart-at-position**
- later upgrade to better GStreamer control if needed

---

# Minimal Rust architecture

## Binaries

### `koboaudiod`
Long-running daemon.

Responsibilities:
- detect backend
- manage one playback session
- maintain queue/state
- own the child process/backend
- expose local control socket

### `koboaudioctl`
CLI client.

Used by NickelMenu/scripts:
- `koboaudioctl play /mnt/onboard/Music/foo.mp3`
- `koboaudioctl pause`
- `koboaudioctl resume`
- `koboaudioctl seek 120`
- `koboaudioctl seek +30`
- `koboaudioctl volume 70`
- `koboaudioctl status`

---

# High-level module layout

```text
src/
  bin/
    koboaudiod.rs
    koboaudioctl.rs
  backend/
    mod.rs
    gstreamer_cli.rs
    hal_direct.rs
  daemon/
    mod.rs
    state.rs
    ipc.rs
    player.rs
  model/
    mod.rs
    command.rs
    status.rs
    track.rs
  platform/
    mod.rs
    detect.rs
  util/
    mod.rs
    process.rs
    time.rs
```

---

# Core data model

## `Track`
```rust
pub struct Track {
    pub path: String,
    pub duration_secs: Option<f64>,
    pub title: Option<String>,
}
```

## `PlaybackState`
```rust
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}
```

## `PlayerStatus`
```rust
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub track: Option<Track>,
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub volume: u8,
    pub backend: String,
    pub can_seek: bool,
    pub can_pause: bool,
    pub can_resume: bool,
}
```

---

# IPC command model

Use a Unix socket with simple JSON messages.

## Commands
```rust
pub enum Command {
    Play { path: String },
    Pause,
    Resume,
    Stop,
    SeekAbs { position_secs: f64 },
    SeekRel { delta_secs: f64 },
    Volume { percent: u8 },
    Status,
    Shutdown,
}
```

This is a very clean fit for NickelMenu because `koboaudioctl` can just serialize commands to the daemon.

---

# Backend abstraction

Define a trait like this:

```rust
pub trait AudioBackend {
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> BackendCapabilities;

    fn play(&mut self, path: &str, start_pos_secs: f64) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn pause(&mut self) -> anyhow::Result<()>;
    fn resume(&mut self) -> anyhow::Result<()>;
    fn set_volume(&mut self, percent: u8) -> anyhow::Result<()>;

    fn poll_position(&mut self) -> anyhow::Result<Option<f64>>;
    fn poll_finished(&mut self) -> anyhow::Result<bool>;
}
```

## Capabilities
```rust
pub struct BackendCapabilities {
    pub can_seek: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_volume: bool,
    pub reports_position: bool,
}
```

Notice I did **not** put `seek()` directly in the trait.

That is deliberate.

Because in your first version, seeking may be implemented by the **player layer**, not by the backend itself.

---

# How seeking should work

This is the main part you asked about.

## Short answer
For the first version, **seeking should be implemented as “stop and restart at new offset.”**

That sounds crude, but it’s actually the right first move.

Why:
- it works across backends
- it avoids needing interactive control of a running subprocess
- it keeps the architecture clean
- for music players, this is acceptable if restart latency is small

---

## Seek strategy by layer

### Player layer owns logical seek
When user requests seek:
1. compute target position
2. stop current backend playback
3. restart backend at target position
4. update daemon state

So seek is a **session restart operation**.

### Backend layer handles “play from offset”
Each backend decides how to honor `start_pos_secs`.

That’s the key abstraction.

---

# How GStreamer seek would work in v1

You correctly noticed:
- `gst-play-1.0` is not a nice remote-control API
- `gst-launch-1.0` is even more of a fire-and-forget pipeline tool

So in **v1**, your GStreamer backend should not attempt runtime seek commands.  
Instead, it should use **start-at-offset playback**.

## Option A: `gst-play-1.0` with URI time fragment
This is the same idea `audiobook.koplugin` was using in places:
- `file:///path/to/file#t=123`

That may work depending on pipeline/format support.

Conceptually:
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "file:///mnt/onboard/Music/foo.mp3#t=123"
```

If Kobo’s build honors it, great.

## Option B: GStreamer pipeline using `playbin`/`uridecodebin`
If URI fragment seeking works more reliably with `uridecodebin`, you can launch a pipeline from Rust using `gst-launch-1.0` and a `file://...#t=` URI.

Example concept:
```sh
gst-launch-1.0 playbin uri="file:///mnt/onboard/Music/foo.mp3#t=123" audio-sink=mtkbtmwrpcaudiosink
```

Whether `playbin` CLI syntax works on-device must be tested, but the point is: **seek becomes restart with a URI offset**.

## Option C: ffmpeg-assisted seeking
If GStreamer URI fragment behavior is unreliable, fallback approach:
- decode/transcode from offset using ffmpeg
- pipe output into sink/backend

But that is more complexity than you want for v1.

---

# Practical v1 seeking algorithm

Let’s say current track is 300 seconds long, and user wants +30 seconds from 75.

### Player logic:
```rust
target = clamp(current_position + 30.0, 0.0, duration)
backend.stop()
backend.play(track.path, target)
state.position_secs = target
state.state = Playing
```

That’s it.

---

# How do you know current position?

This is the second half of seeking.

Since the CLI tools may not report live position nicely, you need a daemon-side estimate.

## Position tracking in v1 = wall clock model

When playback starts:
- store `started_at_monotonic`
- store `base_position_secs`

Then current position is:

```text
base_position_secs + (now - started_at_monotonic)
```

unless paused/stopped.

This is enough for:
- status display
- relative seek
- progress bar
- next/prev logic

It’s not sample-accurate, but it’s good enough for a first music player.

---

## State fields needed

```rust
pub struct SessionState {
    pub track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
    pub started_at_monotonic: Option<Instant>,
    pub paused_position_secs: Option<f64>,
    pub volume: u8,
}
```

### To compute current position:
- if `Playing`: `base + elapsed`
- if `Paused`: paused position
- if `Stopped`: base position or 0

---

# Pause/resume in v1

This depends on what the subprocess can do well.

## Simplest good solution
Treat pause/resume similarly to seek:

### Pause
- compute current estimated position
- stop backend
- mark paused at that position

### Resume
- restart backend with `start_pos_secs = paused_position`

This is not a “true paused pipeline,” but it is often perfectly acceptable for a Kobo music player.

Advantages:
- works even if GStreamer CLI lacks runtime pause control
- consistent with seek
- easy to reason about

If later you find a reliable way to pause/resume `gst-play-1.0` interactively, you can optimize it.

---

# Volume control in v1

You said you need volume control. With CLI GStreamer, that may also be awkward.

So treat volume similarly:

## Option 1: If GStreamer CLI can expose volume
Use it.

## Option 2: If not, restart with volume parameter
If your chosen pipeline can include a `volume` element or property, rebuild pipeline with new volume.

Example idea:
```sh
gst-launch-1.0 playbin uri="file:///..." audio-filter="volume volume=0.5" ...
```

or:
- use a pipeline with `volume ! audioconvert ! ... ! mtkbtmwrpcaudiosink`

If runtime mutation is hard, just:
- remember current position
- restart playback with new volume pipeline

For v1, that’s okay.

---

# Why this restart-based control model is okay

Because your immediate goal is:
- standalone Kobo music player
- background playback
- cross-device-ish design
- not perfect desktop-media-player polish

Restart-based seek/pause/volume sounds a bit brute-force, but it gives you:

- clean architecture
- backend independence
- easy fallback to HAL backend
- no dependency on complicated interactive subprocess control

And because the MTK GStreamer sink already works, restart latency may be acceptable.

---

# Backend detection logic

Your current device suggests this order:

```rust
if has_gst_play() && has_mtk_sink() {
    use GStreamerMtkBackend
} else if hal_socket_looks_viable() {
    use HalDirectBackend
} else {
    no backend
}
```

The detection functions can simply run:
- `command -v gst-play-1.0`
- `gst-inspect-1.0 mtkbtmwrpcaudiosink`
- maybe inspect `/proc/net/unix`

---

# Suggested concrete backend behavior

## `GStreamerCliBackend`
Fields:
```rust
pub struct GStreamerCliBackend {
    child: Option<std::process::Child>,
    current_path: Option<String>,
    current_start_pos: f64,
    volume: u8,
}
```

### `play(path, start_pos_secs)`
- stop existing child if any
- construct command
- spawn child
- store child handle/path/start position

### `stop()`
- kill child
- reap child
- clear state

### `pause()`
In v1:
- return `Unsupported` and let player layer implement pause via stop + saved position

or just let player never call backend pause.

### `resume()`
Same: handled by player layer as restart.

### `set_volume(percent)`
- store volume
- if playing, player layer can restart at current position with new volume

### `poll_finished()`
- `try_wait()` on child

### `poll_position()`
- return `None`
- player layer uses clock estimate

That is a very workable first implementation.

---

# Player engine flow

## Play command
1. detect metadata if possible
2. backend.play(path, 0.0)
3. state.track = track
4. state.base_position_secs = 0
5. state.started_at_monotonic = now
6. state = Playing

## SeekAbs
1. compute target
2. backend.stop()
3. backend.play(path, target)
4. state.base_position_secs = target
5. state.started_at_monotonic = now
6. state = Playing

## Pause
1. current = estimated_position()
2. backend.stop()
3. state.paused_position = current
4. state = Paused

## Resume
1. pos = paused_position
2. backend.play(path, pos)
3. state.base_position_secs = pos
4. state.started_at_monotonic = now
5. state = Playing

---

# Why not use the HAL backend as primary for seek?

You could, but only if you also want to own:
- decoding
- track parsing
- offset seeking in compressed formats
- resampling
- gap handling

That’s much heavier.

GStreamer already proved it can play the file. So use it.

---

# My recommended v1 milestones

## Milestone 1
- Rust daemon with Unix socket
- commands: play, stop, status
- GStreamer MTK backend only
- position estimated by clock

## Milestone 2
- pause/resume using stop/restart
- seek abs/rel using restart-at-offset
- persistent status file

## Milestone 3
- volume control via pipeline reconstruction
- queue/playlist
- automatic next track

## Milestone 4
- HAL fallback backend
- cross-device probing
- smarter metadata/duration extraction

---

# Important experiment you should do next

Before locking in the seek design, test whether Kobo’s GStreamer supports URI fragment seek.

Try this with a real file:

```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink "file:///mnt/onboard/Music/test.mp3#t=30"
```

If that starts around 30 seconds in, you’re in great shape.

Also test:

```sh
gst-launch-1.0 playbin uri="file:///mnt/onboard/Music/test.mp3#t=30" audio-sink=mtkbtmwrpcaudiosink
```

If either works, your v1 seek story is solid:
- **seek = restart at URI offset**

If neither works, then we can design a more explicit fallback strategy.

---

# Bottom line

Because `gst-play-1.0` already works with your headphones, the minimal Rust architecture should be:

- **daemon + CLI**
- **GStreamer MTK subprocess backend first**
- **seek/pause/resume implemented initially as stop-and-restart**
- **position tracked in daemon using monotonic time**
- **HAL direct backend kept as fallback/experimental**

If you want, next I can write a **concrete Rust crate/module skeleton** with:
- trait definitions
- command enums
- daemon state struct
- GStreamer backend stub
- seek/pause logic in pseudocode or actual Rust.

## @KiraGreifeneder

yeahh no it doesn't start 30 seconds in  :|

## Copilot

Then don’t rely on `gst-play`/`gst-launch` URI fragments for seek.  
That means the cleanest **v1** answer is:

**Use GStreamer only for “play this source now”, but use a separate decoder/segmenter path for real seeking.**

Since you’re staying Rust-based, I’d structure it like this:

## Recommended architecture now

### Primary design
- **Daemon/controller in Rust**
- **Backend abstraction in Rust**
- **Two playback strategies**
  1. **GStreamer direct-file backend** for simple play
  2. **Decoded-from-offset backend** for seek-capable play

Because URI `#t=` didn’t work, seeking has to happen **before** the sink sees the audio.

That means:

- either use a player that supports seek natively
- or start decoding at the requested offset yourself and feed that into the sink

On Kobo, given what you’ve already discovered, the second option is more realistic if you want to keep using the working MTK sink.

---

# What this means in practice

## For normal playback
You can still do:
```sh
gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink /path/to/file.mp3
```

## For seek-capable playback
You likely want a pipeline more like:

```text
decoder starting at offset N  ->  PCM stream  ->  GStreamer sink path
```

So your player becomes:

- **track/queue/state manager in Rust**
- **decoder process or library**
- **sink backend process or library**

---

# Best practical v1 choice

Given your constraints, I would not jump straight to full Rust GStreamer bindings.

Instead I’d do:

## Rust daemon + subprocess backends
with two backends:

### A. `GstPlayBackend`
- simplest
- best for initial playback
- no real seek

### B. `SeekableTranscodeBackend`
- uses `ffmpeg`/`ffprobe` if available, or another decoder
- starts decoding at offset
- outputs WAV/PCM
- feeds a GStreamer pipeline targeting `mtkbtmwrpcaudiosink`

This sounds heavier, but architecturally it’s straightforward.

---

# The real issue: you need a decoder that can start mid-file

Since `mtkbtmwrpcaudiosink` is just the sink, it does not solve:
- MP3 seeking
- OGG seeking
- FLAC seeking
- duration parsing

You need one of:

1. **GStreamer library API** with real seek support
2. **Another player engine with seek support**
3. **A decoder/transcoder that starts from offset**

## My honest recommendation
For a minimal system that actually works:

### Short term
Use **external decode tools** if available.

For example:
- `ffmpeg -ss 30 -i file.mp3 ...`
- pipe decoded audio into GStreamer sink pipeline

### Long term
Move to a **native Rust playback core** with proper library control.

---

# The architecture I’d recommend now

## Layers

### 1. Controller layer
Rust daemon handles:
- commands
- queue
- playback state
- current position estimate
- backend selection

### 2. Decode layer
Responsible for:
- open file
- start from offset
- output PCM stream

### 3. Output layer
Responsible for:
- sending PCM/audio to actual device
- here: Kobo MTK BT via GStreamer sink

This is more robust than treating `gst-play` as a full player.

---

# Concrete backend model

Instead of one backend trait, split it into two concepts:

## Source/decoder
```rust
trait AudioSource {
    fn start(&mut self, path: &str, offset_secs: f64) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
}
```

## Output sink
```rust
trait AudioSink {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn write_mode(&self) -> SinkWriteMode;
}
```

But because you’re likely using subprocesses, a more realistic v1 is:

## Combined pipeline backend
```rust
trait PlaybackBackend {
    fn play(&mut self, path: &str, offset_secs: f64, volume: u8) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn poll_finished(&mut self) -> Result<bool>;
}
```

And then implement:

### `GstDirectFileBackend`
- `gst-play-1.0 ... file.mp3`
- only for offset=0 or no-seek mode

### `TranscodeToMtkBackend`
- decode from offset
- feed MTK sink

---

# How seek would work now

Since `#t=` failed, seek becomes:

1. user requests new position
2. daemon computes `target_secs`
3. daemon stops current playback
4. backend launches a **new decode pipeline beginning at `target_secs`**
5. daemon resets timing base

That’s still restart-based seek, but the **offset is enforced by the decoder**, not by GStreamer URI handling.

---

# What could provide the offset decode?

You need to check what tools already exist on the Kobo.

The big ones to probe are:

- `ffmpeg`
- `ffplay`
- `ffprobe`
- `mplayer`
- `mpg123`
- `madplay`
- `sox`
- `ogg123`
- `flac`

If `ffmpeg` exists, your life gets much easier.

For example, conceptually:

```sh
ffmpeg -ss 30 -i file.mp3 -f wav - | gst-launch-1.0 fdsrc ! wavparse ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

Or for raw PCM:
```sh
ffmpeg -ss 30 -i file.mp3 -f s16le -ac 2 -ar 48000 - | gst-launch-1.0 fdsrc ! audio/x-raw,format=S16LE,rate=48000,channels=2 ! mtkbtmwrpcaudiosink
```

That gives you real seek because `ffmpeg` starts decoding from 30 seconds.

---

# If ffmpeg is not installed

Then you have three paths:

## Path A: bundle ffmpeg
Heavy, but powerful.

## Path B: use your HAL backend for seek-capable PCM playback
You still need a decoder though.

## Path C: use GStreamer libraries directly from Rust
This is probably the “cleanest” technically, but more effort.

Given your “minimal Rust architecture” request, I’d probably do:

- **subprocess-first design**
- later replace subprocess implementation with library-backed implementation

---

# Updated minimal Rust architecture

## `PlayerEngine`
Owns:
- state
- backend
- clock-based position estimate
- restart-based seek/pause/resume

## `BackendKind`
```rust
enum BackendKind {
    GstDirect,
    DecodeToMtk,
    HalDirect,
}
```

## `PlaybackBackend`
```rust
trait PlaybackBackend {
    fn name(&self) -> &'static str;
    fn play(&mut self, path: &str, offset_secs: f64, volume: u8) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn poll_finished(&mut self) -> anyhow::Result<bool>;
    fn supports_seek(&self) -> bool;
}
```

### `GstDirectBackend`
- supports seek: false
- play with `gst-play-1.0 --audiosink=mtkbtmwrpcaudiosink`

### `DecodeToMtkBackend`
- supports seek: true
- if `offset_secs == 0`, can still use decode path
- launches:
  - decoder process
  - GStreamer sink process

---

# Pause/resume under this model

Still simple:

## Pause
- compute current position
- stop playback
- remember paused position

## Resume
- backend.play(path, paused_position, volume)

This works perfectly fine if the seekable backend can start from an offset.

---

# Volume control under this model

Much easier if you have a decode pipeline:

- either scale in decoder/output pipeline
- or software volume in PCM path

For example:
- ffmpeg filter volume
- GStreamer `volume` element before sink
- custom Rust PCM scaling later

So ironically, once you stop relying on `gst-play` as a monolithic player, **volume becomes easier**.

---

# What I’d do next, concretely

## 1. Probe for decoder tools
Run:

```sh
command -v ffmpeg
command -v ffprobe
command -v mplayer
command -v mpg123
command -v ogg123
command -v flac
command -v sox
```

## 2. If `ffmpeg` exists
Your best architecture is immediately:

- Rust daemon
- decode-from-offset with ffmpeg
- output via GStreamer MTK sink

## 3. If no decoder tools exist
Then we should decide between:
- bundling one
- using Rust audio decode crates for limited formats
- moving toward direct GStreamer API usage

---

# Best likely architecture if you want it to scale

If you want this to become a serious music player, the best long-term design is probably:

- Rust daemon
- abstract playback engine
- backend implementations:
  - `GstMtkBackend` using GStreamer API if you can eventually manage it
  - `HalBackend`
  - maybe `BlueAlsaBackend`
- metadata/duration extraction layer
- queue/state/IPC

But for **now**, since you asked for minimal and practical:

## Minimal viable design
- daemon + ctl in Rust
- restart-based seek
- seek implemented by decoder starting from offset
- GStreamer only as sink/output transport
- `gst-play` used only for quick proof-of-concept, not final seekable engine

---

# My advice in one sentence

Because `gst-play` works but **does not seek**, treat it as evidence that **the MTK sink works**, not that `gst-play` should be your final player engine.

If you want, next I can do one of these:

1. give you a **Rust architecture assuming `ffmpeg` is available**, or  
2. give you a **Rust architecture assuming no external decoder tools are available**.

## @KiraGreifeneder

give me the rust architecture, but I'd like to use mpg123

## Copilot

Use **Rust daemon + `mpg123` decoder + GStreamer MTK sink**. That’s a solid architecture for your Kobo music player.

## Big picture

Split the system into:

1. **control plane** in Rust
2. **decode pipeline** using `mpg123`
3. **output pipeline** using `gst-launch-1.0 ... mtkbtmwrpcaudiosink`

So:

```text
MP3 file
  -> mpg123 (decode, seek, PCM out)
  -> gst-launch fdsrc ! raw-audio caps ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
  -> Bluetooth headphones
```

That gives you:
- background playback
- MP3 support
- seek by restarting decoder at offset
- clean Rust orchestration
- no KOReader dependency

## Binaries

### `koboaudiod`
Long-running daemon:
- owns playback session
- tracks current file/position/state
- listens on Unix socket

### `koboaudioctl`
CLI:
- sends commands to daemon
- called by NickelMenu/scripts

## Suggested crate layout

```text
src/
  bin/
    koboaudiod.rs
    koboaudioctl.rs
  backend/
    mod.rs
    mpg123_gst.rs
  daemon/
    mod.rs
    engine.rs
    ipc.rs
    state.rs
  model/
    command.rs
    status.rs
    track.rs
  platform/
    detect.rs
  util/
    process.rs
```

## Core model

### `Track`
```rust
pub struct Track {
    pub path: String,
    pub duration_secs: Option<f64>,
    pub title: Option<String>,
}
```

### `PlaybackState`
```rust
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}
```

### `PlayerStatus`
```rust
pub struct PlayerStatus {
    pub state: PlaybackState,
    pub track: Option<Track>,
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub volume_percent: u8,
    pub backend: String,
}
```

## IPC commands

Use a Unix socket with JSON.

```rust
pub enum Command {
    Play { path: String },
    Stop,
    Pause,
    Resume,
    SeekAbs { position_secs: f64 },
    SeekRel { delta_secs: f64 },
    Volume { percent: u8 },
    Status,
    Shutdown,
}
```

## Backend trait

Keep it simple:

```rust
pub trait PlaybackBackend {
    fn name(&self) -> &'static str;
    fn play(&mut self, path: &str, offset_secs: f64, volume_percent: u8) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn poll_finished(&mut self) -> anyhow::Result<bool>;
}
```

That’s enough for v1.

## Concrete backend: `Mpg123GstBackend`

This backend owns two child processes:

- `mpg123`
- `gst-launch-1.0`

### Why two children?
Because:
- `mpg123` handles MP3 decode and seek offset
- GStreamer handles Kobo-specific sink output

## Seek model

This is the key part.

Because `gst-play` didn’t support useful seeking for you, **seek happens by restarting `mpg123` at a new offset**.

### Algorithm
On seek:
1. calculate target position
2. kill current `mpg123` + `gst-launch`
3. spawn new pipeline with new start offset
4. reset daemon timing base

This is restart-based seek, but it’s perfectly reasonable for a Kobo music player.

## How `mpg123` fits in

`mpg123` should:
- read file
- seek to requested offset
- output PCM to stdout

Then GStreamer reads from stdin via `fdsrc`.

Conceptually:

```sh
mpg123 -s -k <frame-offset> file.mp3 | \
gst-launch-1.0 fdsrc fd=0 ! audio/x-raw,format=S16LE,rate=44100,channels=2,layout=interleaved ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

You may need to tune exact `mpg123` flags depending on the build, but that is the architecture.

## Important note about seeking with `mpg123`

`mpg123` seeking is usually easiest in **frame/sample terms**, not directly seconds.

So your Rust side should likely maintain:

- duration
- sample rate
- estimated position in seconds

and convert requested seconds into a `mpg123` seek parameter.

Depending on what `mpg123` supports on your Kobo build, you may use:
- frame-based skip
- sample-based skip
- or a time/seek flag if available

You should first inspect:
```sh
mpg123 --help
```

because exact options vary.

## Position tracking

In v1, do it in Rust with a monotonic clock.

When playback starts:
- `base_position_secs = requested_offset`
- `started_at = now`

Current position while playing:
```rust
base_position_secs + started_at.elapsed().as_secs_f64()
```

When paused:
- snapshot current position
- stop backend
- set state to `Paused`

When resumed:
- restart backend from saved paused position

## Pause/resume

Also restart-based.

### Pause
1. compute current estimated position
2. stop both child processes
3. save paused position

### Resume
1. restart backend from saved paused position

This avoids complicated runtime control.

## Volume control

For v1, do software volume in the GStreamer pipeline if possible.

Add a `volume` element before the sink.

Conceptually:

```sh
gst-launch-1.0 fdsrc fd=0 \
  ! audio/x-raw,format=S16LE,rate=44100,channels=2,layout=interleaved \
  ! audioconvert \
  ! audioresample \
  ! volume volume=0.70 \
  ! mtkbtmwrpcaudiosink
```

Then changing volume works like seek:
1. compute current position
2. restart pipeline with new `volume` value

That’s good enough for v1.

## Daemon state

```rust
pub struct SessionState {
    pub track: Option<Track>,
    pub playback_state: PlaybackState,
    pub base_position_secs: f64,
    pub started_at: Option<std::time::Instant>,
    pub paused_position_secs: Option<f64>,
    pub volume_percent: u8,
    pub backend_name: String,
}
```

## Engine responsibilities

### `PlayerEngine`
Methods:
- `play(path)`
- `stop()`
- `pause()`
- `resume()`
- `seek_abs(pos)`
- `seek_rel(delta)`
- `set_volume(percent)`
- `status()`

This layer owns logic; backend only executes.

## Example flow

### Play
```rust
backend.play(path, 0.0, volume)?;
state.track = Some(track);
state.base_position_secs = 0.0;
state.started_at = Some(Instant::now());
state.playback_state = PlaybackState::Playing;
```

### Seek
```rust
let target = clamp(current_position + delta, 0.0, duration);
backend.stop()?;
backend.play(path, target, state.volume_percent)?;
state.base_position_secs = target;
state.started_at = Some(Instant::now());
state.playback_state = PlaybackState::Playing;
```

### Pause
```rust
let pos = self.current_position();
backend.stop()?;
state.paused_position_secs = Some(pos);
state.started_at = None;
state.playback_state = PlaybackState::Paused;
```

### Resume
```rust
let pos = state.paused_position_secs.unwrap_or(0.0);
backend.play(path, pos, state.volume_percent)?;
state.base_position_secs = pos;
state.started_at = Some(Instant::now());
state.playback_state = PlaybackState::Playing;
```

## Process wiring in Rust

Your backend likely needs:
- one pipe from `mpg123.stdout` to `gst-launch.stdin`

Sketch:

```rust
pub struct Mpg123GstBackend {
    mpg123_child: Option<std::process::Child>,
    gst_child: Option<std::process::Child>,
}
```

### `play()`
1. stop existing children
2. spawn `mpg123` with `stdout(Stdio::piped())`
3. take `mpg123.stdout`
4. spawn `gst-launch-1.0` with `stdin` connected to that pipe
5. store both children

Depending on Rust ownership ergonomics, the easiest implementation may be to use a shell pipeline first, then later replace it with explicit pipe wiring.

### Minimal pragmatic version
Spawn:
```sh
sh -c 'mpg123 ... | gst-launch-1.0 ...'
```

That’s less elegant, but very fast to get working.

Then later you can improve it to explicit child management.

## Recommended v1 implementation choice

### Start with shell-wrapped pipeline in Rust
Because it is much simpler.

Example command string:
```sh
mpg123 -q -s /mnt/onboard/Music/test.mp3 | \
gst-launch-1.0 fdsrc fd=0 ! audio/x-raw,format=S16LE,rate=44100,channels=2,layout=interleaved ! audioconvert ! audioresample ! volume volume=1.0 ! mtkbtmwrpcaudiosink
```

For seek, you adjust the `mpg123` arguments.

Rust just launches and kills the wrapper shell PID tree.

That is enough for v1.

## Platform detection

At daemon startup:
- verify `gst-launch-1.0`
- verify `gst-inspect-1.0 mtkbtmwrpcaudiosink`
- verify `mpg123`

If all present, choose `Mpg123GstBackend`.

## Recommended NickelMenu integration

NickelMenu should invoke only the CLI:

```sh
menu_item :Play Music:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl play "/mnt/onboard/Music/song.mp3"
menu_item :Pause Music:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl pause
menu_item :Resume Music:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl resume
menu_item :Seek +30:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl seek +30
menu_item :Seek -10:exec:/mnt/onboard/.adds/koboaudio/koboaudioctl seek -10
```

## What to test before coding too much

You should manually verify `mpg123` exists and supports raw PCM output + seek:

```sh
command -v mpg123
mpg123 --help
```

Then test raw decode:

```sh
mpg123 -q -s "/mnt/onboard/Music/test.mp3" | \
gst-launch-1.0 fdsrc fd=0 ! audio/x-raw,format=S16LE,rate=44100,channels=2,layout=interleaved ! audioconvert ! audioresample ! mtkbtmwrpcaudiosink
```

If that works, your architecture is validated.

Then test whatever seek flag your `mpg123` build supports.

## Bottom line

Your minimal Rust architecture should be:

- **Rust daemon** for playback state + IPC
- **Rust CLI** for NickelMenu/script control
- **`Mpg123GstBackend`** as the first real backend
- **seek/pause/resume/volume implemented by restarting pipeline**
- **position tracked in Rust using a monotonic clock**

If you want, I can next write:
1. the actual Rust trait and struct skeletons, or
2. the exact shell test commands to validate `mpg123` raw output and seeking on your Kobo.
