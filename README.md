# VIctory on NVIDIA

> **Reporting a problem?** Please attach **both** logs from the run that went
> wrong. Without them we can only guess. Paste `%LOCALAPPDATA%` into the address
> bar of File Explorer to get there.
>
> 1. **The play log**, in `%LOCALAPPDATA%\victory\logs\`: the newest
>    `victory-nvidia-play-<date>-<time>-<number>.log` (from `victory.exe`) or
>    `victory-nvidia-<date>-<time>.log` (from the script launcher). The window
>    shows its path at the start, on the line `Log: ...`.
> 2. **The crash log**: `%LOCALAPPDATA%\VIctory\data\7952987A\crashes.log`, and,
>    if there is one from that run, the newest `victory-<date>-<time>-<number>.crash.log`
>    in `%LOCALAPPDATA%\VIctory\data\7952987A\temp\`.
>
> Also say which GPU, Windows version (10 or 11) and release you used.

## Frame generation: what your card supports

| Your card | Frame generation | Limits |
| --- | --- | --- |
| GeForce RTX 40 and RTX 50 | **DLSS frame generation** | 2x on RTX 40; more than 2x (up to 6x) needs an RTX 50 |
| GeForce RTX 20 and RTX 30 | **AMD FSR 3 frame generation** (DLSS frame generation needs an RTX 40 or later) | **2x at most**. DLSS upscaling still works on these cards, so the Series X profile is fine |
| Other cards (no DLSS) | **AMD FSR 3 frame generation** | **2x at most, Series S profile only** (F1 menu, CONSOLE): without DLSS the game's own upscaler, which FSR 3 needs, only takes the Series S sizes. Not tested here |

- **At every start the game tells you** which one your card runs, and why
  not if it can't, at the top of the picture for the first 30 seconds (and
  in the console window).
- **F1 menu, FRAME GEN TYPE**: AUTO (DLSS where the card has it, else FSR 3),
  DLSS, or FSR 3 (also on an RTX 40/50, if you prefer it). FRAME GENERATION
  switches it off or on.
- FSR 3 needs AMD's FidelityFX DLLs (`amd_fidelityfx_loader_dx12.dll` and
  `amd_fidelityfx_framegeneration_dx12.dll`, about 30 MB). They are AMD's and
  not part of this program. On a card without DLSS frame generation (or with
  FSR 3 chosen in F1) the first start of `victory.exe` asks once whether to
  download them from AMD's own GitHub (the FidelityFX SDK);
  `victory.exe get-fsr` does it later, and `Get-AMD-FSR.cmd` does it for the
  script launcher. Each file is checked against the copy this fork was tested
  with, and only loaded with AMD's valid signature. A `fidelityfx` folder next
  to `victory.exe`, or `VICTORY_FIDELITYFX_DIR`, works too.

## Quick start: how to run it

**The easy way: one file, nothing to build**

1. Download `victory.exe` from the
   [latest release](https://github.com/trustno1-coder/VIctory-NVIDIA/releases/latest).
2. Put it into the folder of **your own dump** of the title's Xbox Series
   build v1.0.2.1, next to `game_scarlett_master.exe`.
3. Double-click it. Windows may say the publisher is unknown (the file is not
   signed): "More info", then "Run anyway".

- The first start asks once whether to download NVIDIA's DLSS and frame
  generation files (about 70 MB) from NVIDIA's own GitHub. They are NVIDIA's
  and are not part of this program; say no and the game still runs, with its
  own upscaling and no frame generation.
- Settings, saves, caches, logs and NVIDIA's and AMD's files are kept under
  `%LOCALAPPDATA%\victory`. The one exception is on Windows 10. Its own Direct3D 12
  stops at shader model 6.5, and the game's translated shaders need 6.6. Without
  6.6 the game shows a black screen and closes within seconds. So the first start
  there offers to fetch Microsoft's newer Direct3D 12 runtime (the D3D12 Agility
  SDK, `D3D12Core.dll`, about 22 MB from nuget.org). It goes into a `D3D12`
  folder beside `victory.exe`. `victory.exe get-d3d12` does the same by hand.
  The file is checked against Microsoft's signature and the tested SHA-256.
  Windows 11 keeps using its own runtime. A `D3D12` folder that is already
  there, for example from a repack or another guide (1.618, 1.620), is used
  at the version of its `D3D12Core.dll`. Before v2026.11.27.4 that version
  had to be 1.614, or Windows refused Direct3D 12 with `0x887E0003`.
- C: drive full? Set `VICTORY_HOME` to another folder, once, in a command
  prompt: `setx VICTORY_HOME D:\victory`. Then start victory.exe from a new
  window. Settings, saves, caches, logs and the downloads all go there
  instead of `%LOCALAPPDATA%\victory`. To keep what you have, move that
  folder's contents there first.
- You need Windows 10 or 11, 32 GB of RAM or more, an SSD and an NVIDIA
  GeForce RTX card with a recent driver. This repository and its releases
  contain no game files.
- **The first start of each new version restarts by itself once.** A new
  `victory.exe` runs at about 14 fps on its first start, and normally from then
  on. So the first start plays about 20 seconds, says so at the top of the
  picture, and starts the game again. It doesn't happen again for that version
  (a mark is kept in `%LOCALAPPDATA%\victory\warmed`).
- RTX 20 and 30 cards can try DLSS frame generation with the
  [MFG Unlocker](https://github.com/dashdogy/MFG-Unlocker) mod (not ours, and
  not tested with this fork, which was only run on an RTX 50). It is an option,
  off by default: **F1, MFG UNLOCKER, ON**, then restart. On, the next start
  downloads it by itself from the mod's own GitHub release (v1.7.2, about 5 MB,
  checked against the SHA-256 its author published) into
  `%LOCALAPPDATA%\victory\mfg-unlocker`, and every start loads it. Nothing is
  renamed or copied. On an RTX 20 or 30 the first start also asks once (No unless
  you type y). `victory.exe get-mfg` downloads it by hand. The mod writes its own
  `RTXMFG-...` logs and settings beside `victory.exe`, and Backspace opens its
  menu. The start's notice says "UNLOCKED BY A MOD" and the log says it was
  loaded. If the game crashes or frame generation fails, turn it off in F1 (or
  add `VICTORY_MFG_UNLOCKER=` to `%LOCALAPPDATA%\victory\victory-settings.txt`).
  FSR 3 frame generation (2x) is the supported way on those cards. A
  `winmm.dll` copy of the mod beside `victory.exe` still works too.

**In the game**

- **F1** brings up the settings menu (arrow keys pick and change, Enter does
  it): DLSS, frame generation, frame rate cap and more.
- **F3** shows the stats panel (frame rate, frame generation, draws, GPU and
  memory numbers).

**Building it yourself instead**

Install [Rust](https://rustup.rs) and the
[Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
with "Desktop development with C++", then in PowerShell:

```powershell
git clone https://github.com/trustno1-coder/VIctory-NVIDIA
cd VIctory-NVIDIA
.\Get-NVIDIA-DLSS.cmd
powershell -ExecutionPolicy Bypass -File scripts\play-nvidia.ps1 -Exe 'D:\Games\MyDump\game_scarlett_master.exe'
```

Put the path to **your** `game_scarlett_master.exe` in the last line. The
first start builds the layer, which takes a few minutes.

Options, requirements in full and the details are under
[How to run it](#how-to-run-it).

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files **you** supply,
tuned for **NVIDIA RTX** cards.

> **This repository contains no game files, code or assets.** You need your
> own legally obtained copy of the title. It is not affiliated with or endorsed
> by the title's developer or publisher, nor by NVIDIA. See
> [docs/asset-policy.md](docs/asset-policy.md).

## Built from many projects, and from running every one of them

This repository did not come from one place, and the base it stands on today
is the end of a long comparison, not the start of the work. Over several weeks
we built, ran and measured many projects on the same NVIDIA PC: Leonida,
[Marvisak's VIctory](https://github.com/Marvisak/VIctory),
[fxceless's fork](https://github.com/fxceless/victory-fork),
[the 7900 XTX fork](https://github.com/avatarmanz/VIctory-7900XTX),
[Belobug's](https://github.com/Belobug/VIctory-Belobug-AMD),
[DexNom's](https://github.com/DexNom/VIctory),
[CGBJ's cool_fox](https://github.com/codec-d/cool_fox), Kelvaroq's, Neptune,
xcl (another layer for the same title) and then
[foxhound](https://github.com/readmet3xt-lab/foxhound). On each we found what
stopped it on an RTX card, fixed or worked around it, measured the result, and
carried what we learned to the next. Every pull request sent here was built
and run on the same card before it was merged. No single project got here
alone, and choosing the base that runs best was only one result of that work.

### What this fork brings

Not one fix but a long line of them: days of work, much of it around the clock,
across every one of those codebases, more than 300 measured runs on the same
RTX PC, and each fault chased towards its cause before the next build. In rendering, in
speed, in the picture and in simply being able to play it:

**Rendering fixes**

- **Draws no longer skipped on NVIDIA.** NVIDIA caps the view heap below what
  the title asks for; thousands of draws a frame were dropped, the screen
  rarely updated and the character jumped from place to place. The pool of
  descriptor slots was reworked until almost none were.
- **Menus, loading screens and subtitles on Series S**, black until then (the
  interface plane laid over the game), and **faces that move** (their network
  carried through as integers): ported from fxceless's pull requests 1189 and
  927 and made to work on this card.
- **The intro video without flicker**: its frames came out of order.
- **Black blobs gone**: a single bad number from a shader, spread by blur and
  the upscaler into patches on faces and in the air, is caught where it is
  written.
- **White, hazy rooms explained and tamed**: the title's ray tracing was
  running with its structure builder off, so every ray missed. Which passes
  can be left out without filling the picture with rainbow glitter was worked
  out pass by pass.
- **The ray-tracing structure builder running on NVIDIA**, on CGBJ's renderer:
  the draws dropped for it went from thousands a frame to one.
- **White-outs with black holes further into the game** followed up in frames
  captured in play: thousands of draws a frame were being dropped there for
  that same missing structure.

**Speed**

- **Memory compares moved to the card**: 11 to about 22 frames a second on the
  base of the time.
- **Settings measured, not guessed**: one switch at a time, on the same scene,
  run after run.
- **The base itself chosen by measurement**: the same rooms went from 15-17
  frames a second to 26-30, at a higher resolution.
- **The 30 frames a second were the title's own frame interval**, not the
  layer's limit: found, and lifted.

**Picture and frames**

- **DLSS in place of the title's upscaler**, with its model and mode to choose.
- **DLSS frame generation with up to six frames shown for each one rendered**,
  switched and changed while playing, without a restart (foxhound's is 2x, set
  at the start). By Streamline's own count it shows exactly 2, 4 and 6.
- **Why frame generation showed nothing on a full 12 GB card**, found and
  fixed: the card's memory is now shared out so that it has room.
- **The frame rate at the top left of the picture**: frames rendered a second
  and, with frame generation, frames shown (`FPS: 28.4  FG: 170.4`).

**Playing it**

- **The F1 settings menu**, in every build: DLSS, frame generation, frame rate
  cap, console profile, picture size, the light estimates and the main speed
  switches, saved to `victory-settings.txt` and applied at the next start
  ("SAVE AND RESTART NOW" does that for you; lines marked LIVE change at once).
- **The F3 stats panel**, in every build: frame rate and frame time, frames
  shown with frame generation, draws, how busy the GPU and the submitting
  thread are, copies, RAM and VRAM. The menu switches it too.
- **Open water without dark blocks.** Two of the title's water light programs
  run on 32-lane cards only in a form that left whole tiles of the sea dark
  (around boats, along shores, under bridges) and cost about 15 ms of the
  card a frame. They are left out there: no blocks and 30 fps instead of 25
  on the first boat ride (RTX 5070), with water that is darker and bluer.
  WATER LIGHT in the menu (`VICTORY_WATER_LIGHT=1`) runs them again.
- **One command to play**: the launcher builds the layer the first time and
  applies what the menu saved.
- **`Get-NVIDIA-DLSS.cmd`**: the eight NVIDIA files that are loaded, about
  70 MB, taken out of NVIDIA's 263 MB release without downloading the rest.
  NVIDIA's licence does not let a repository carry them, so they come from
  NVIDIA.

**Tested, not assumed**

- Every pull request sent here was read line by line and, where it touched the
  game, built and run on this card before it was merged: ten so far.

Where each of these is today: this fork's own code in this tree is the F1
menu, the stats panel in every build, the water light switch, frame
generation from 2x to 6x with its live switching and memory handling, the
lifted frame rate cap, the frame rate line, the launcher and the NVIDIA file
fetcher. The foxhound line carries the descriptor slots, the
compares on the card, the interface plane and the face network in its own
form. The black-blob scrub, the graphics card picker and the ray-tracing
builder path are in this repository's history, and come over as this base
turns out to need them.

### The base since 2026-11-26: foxhound

On 2026-11-26 this fork moved onto
[readmet3xt-lab/foxhound](https://github.com/readmet3xt-lab/foxhound)
(`02fc18a`, GPL-3.0-or-later). On the same PC it ran the same rooms at close to
twice the frame rate of what this fork had, at a higher resolution, so its
tree is now this one's, with its history kept, and this fork's own additions
are on top. What foxhound is and how it got there is in its own README, kept as
[docs/README-foxhound.md](docs/README-foxhound.md); its measurements are in
[docs/findings.md](docs/findings.md).

What foxhound brings:

- the Series X profile at a native 2560x1440;
- DLSS Super Resolution and DLSS Frame Generation through NVIDIA Streamline,
  loaded when the game runs (nothing of NVIDIA's is needed to build);
- an estimate of indirect light and of reflections in place of the title's ray
  tracing, and fewer stalls at scene cuts.

What this fork has on top of it is under
[What this fork brings](#what-this-fork-brings) above.

## Where it stands

Tested on one PC: Windows 11, Ryzen 7 9800X3D, 64 GB RAM, **GeForce RTX 5070
12 GB**; the title's Xbox Series build v1.0.2.1, Series X profile, 2560x1440
output, DLSS Quality. Frame rates are the layer's own count, in the opening
rooms.

| | |
|---|---|
| Opening rooms | mostly 26-30 frames a second rendered (33-38 ms a frame) |
| Frame generation | 2x to 6x on top of that: by Streamline's own count, exactly 2, 4 and 6 frames shown for each one rendered |
| Device losses, crashes | none in the measured runs |
| Before this base (Series S, 720p to 1080p by DLSS) | 15-17 frames a second in the same rooms |

foxhound's own measurements, on an RTX 4070 SUPER, are in its README.

What is known to be wrong:

- **No ray tracing.** The title's own is off (`VICTORY_BVH_BUILDER=1`, or the
  menu's RAY TRACING, turns it on: slow and with a broken picture on this
  base). An estimate stands in for indirect light and reflections.
- **Frame generation needs room in the card's memory.** The layer fills the
  card with textures up to its budget; with frame generation on from the start
  the launcher holds a fifth of the budget back (a little less while it is only
  ready), so a few more textures can come late. Switched on from the menu on a
  full 12 GB card it can still say that memory is past the budget; it then
  runs, perhaps unevenly.
- **Frames are only generated while the game's window has the focus**, and
  about a second passes after a change before they come.
- **The first run of every new build stutters** while the driver compiles
  pipelines (foxhound's README, *Warming up*, says how to make them ahead).
- "SAVE AND RESTART NOW" can leave the old game's process behind without a
  window; it goes when the PC restarts.
- What foxhound lists as wrong (the heat-haze pass left out, hitches in
  cutscenes) applies here too.

## How to run it

You need Windows 10 or 11 (x64), a CPU with AVX2, 32 GB of RAM or more, an
SSD, an NVIDIA GeForce RTX card with a recent driver (RTX 40 or later for frame
generation, with hardware-accelerated GPU scheduling on; more than 2x needs an
RTX 50), [Rust](https://rustup.rs) (the toolchain is pinned), the Visual Studio
Build Tools with "Desktop development with C++", and **your own dump** of the
title's Xbox Series build v1.0.2.1 (the folder with
`game_scarlett_master.exe`).

```powershell
git clone https://github.com/trustno1-coder/VIctory-NVIDIA
cd VIctory-NVIDIA
copy victory.example.toml victory.toml      # then set game_dir to your dump's folder, outside this one
.\Get-NVIDIA-DLSS.cmd                        # once: NVIDIA's DLSS and frame generation files, about 70 MB
scripts\play-nvidia.ps1 -Exe 'D:\Games\MyDump\game_scarlett_master.exe'
```

The first start builds the layer (a few minutes). `Get-NVIDIA-DLSS.cmd` gets
NVIDIA's files from
[NVIDIA-RTX/Streamline](https://github.com/NVIDIA-RTX/Streamline) into
`nvidia-sdk\` (git-ignored). They are NVIDIA's, under NVIDIA's licences, which
are saved beside them; this repository does not carry them and may not. Nothing
is downloaded unless you run that file; `VICTORY_STREAMLINE_DIR` names another
folder if you have the Streamline SDK already.

`scripts\play-nvidia.ps1` is `scripts\play-foxhound.ps1` under the name this
fork has always had; its options:

| Option | What |
|---|---|
| *(none)* | Series X profile, 2560x1440, DLSS Quality, no frame rate cap, frame generation ready but off |
| `-FrameGen`, `-FrameGenX 4` | frame generation on from the start, 2 to 6 frames shown for each one rendered |
| `-Dlss off` / `dlaa` / `balanced` / `performance` | the title's own upscaling, or another DLSS mode |
| `-Fps 60` | a frame rate cap (30 is the title's own) |
| `-SeriesS` | the Series S profile: 1280x720 inside, faster, softer |
| `-Res 1080to1440` / `4k` | scene at 1080p upscaled by the title / the stock 3840x2160 output |
| `-NoFrameGenReady` | frame generation only with `-FrameGen` (it is otherwise made ready at every start, so the menu can switch it) |
| `-FgType auto` / `dlss` / `fsr` | which frame generation (see the top of this page); FSR 3's DLLs from `VICTORY_FIDELITYFX_DIR`, else `amd-sdk\FidelityFX\bin` here |
| `-Build diag` | the diagnostics build (slower; captures, timelines, recording time in the F3 panel) |
| `-More @{VICTORY_X='1'}` | add or override any switch |

What the F1 menu saved (`victory-settings.txt`) goes over these at every start;
delete that file to go back to them. Full screen: start the game, then run
`.\scripts\fullscreen.ps1` in another window. Logs go to
`%LOCALAPPDATA%\victory\logs`, saves to `%LOCALAPPDATA%\victory\data`.

## Linux (Proton, NVIDIA)

The same Windows `victory.exe` runs on Linux through Proton (vkd3d-proton, DXVK, dxvk-nvapi). It
is not a native port, and Windows behaves as before. It is written for CachyOS or Arch with an
NVIDIA RTX card and **has not been tested on a GPU yet**. Please report what you find (logs:
step 11). The details are in [docs/linux-proton.md](docs/linux-proton.md); what differs under
Wine, and why, is in [docs/proton-audit.md](docs/proton-audit.md).

### Install on CachyOS or Arch Linux

Every command runs in a normal terminal (bash or fish, CachyOS's default), not as root unless it
starts with `sudo`. Your own dump of the title goes in a folder **outside** this repository, e.g.
`~/Games/MyDump` with `game_scarlett_master.exe` in it.

**1. The NVIDIA driver.** CachyOS installs it for you. Check that it is running:

```bash
nvidia-smi
```

On plain Arch, if that fails: enable the `[multilib]` repository in `/etc/pacman.conf` (the
[Arch wiki](https://wiki.archlinux.org/title/Official_repositories#multilib) shows how), install
the driver, then reboot:

```bash
sudo pacman -S --needed nvidia-open nvidia-utils lib32-nvidia-utils
```

**2. The packages.** On CachyOS (this brings proton-cachyos, umu-launcher and winetricks):

```bash
sudo pacman -S --needed cachyos-gaming-meta git rustup clang lld llvm unzip
```

On Arch (umu-launcher downloads a Proton by itself the first time):

```bash
sudo pacman -S --needed umu-launcher winetricks git rustup clang lld llvm unzip
```

**3. This repository and its Rust toolchain.** Then stay in the `VIctory-NVIDIA-Proton` folder
for every step below.

```bash
git clone https://github.com/oreo1298/VIctory-NVIDIA-Proton
```

```bash
cd VIctory-NVIDIA-Proton
```

```bash
rustup toolchain install
```

```bash
cargo install cargo-xwin --locked
```

**4. Build** (a few minutes; the first build also downloads Microsoft's build libraries, once):

```bash
scripts/build-linux.sh
```

**5. Check the memory-map limit.** This should print `1048576` or more (the Arch and CachyOS
default):

```bash
cat /proc/sys/vm/max_map_count
```

If it prints less, raise it for good:

```bash
echo 'vm.max_map_count = 1048576' | sudo tee /etc/sysctl.d/80-victory.conf
```

```bash
sudo sysctl --system
```

**6. Check that Proton can build the layer's own shaders.** The first run sets up the Proton
prefix (in `~/.local/share/victory/proton`) and downloads umu's Steam Linux Runtime (on Arch a
Proton too), which takes a few minutes, once:

```bash
scripts/play-nvidia.sh hlsl-check
```

If any line shows `NO` under "compiles", install Microsoft's shader compiler into the same prefix
(winetricks downloads it), then run `hlsl-check` again:

```bash
scripts/play-nvidia.sh winetricks d3dcompiler_47
```

**7. DLSS (optional, recommended).** NVIDIA's files are NVIDIA's, under NVIDIA's licence, and
not part of this repository. Download Streamline (about 263 MB) and unpack it into `nvidia-sdk`
in this folder; that folder is git-ignored, and the launcher finds the files there by itself:

```bash
curl -L -o /tmp/streamline.zip https://github.com/NVIDIA-RTX/Streamline/releases/download/v2.14.16/streamline-sdk-v2.14.16.zip
```

```bash
unzip -q /tmp/streamline.zip -d nvidia-sdk
```

**8. Translate the shaders ahead** (a few minutes; again after every update). Put your own dump's
folder in place of `~/Games/MyDump`:

```bash
scripts/play-nvidia.sh shaders ~/Games/MyDump
```

**9. Play:**

```bash
scripts/play-nvidia.sh --exe ~/Games/MyDump/game_scarlett_master.exe
```

F1 is the settings menu, F3 the stats panel. The driver's Smooth Motion (RTX 40 and later) is on.
Add `--no-smooth-motion` to turn it off, or `--series-s` for the Series S profile. After the
first session, make the pipelines it used ahead, so the next start stutters less:

```bash
scripts/play-nvidia.sh warm --used
```

**10. Updating:** fetch the new version and build again, then repeat steps 8 and 9 (and
`warm --used`). After a Proton update, repeat step 6 too:

```bash
git pull
```

```bash
scripts/build-linux.sh
```

**11. If something goes wrong,** attach these to the report:

- the game's log: the newest `victory-nvidia-<date>-<time>.log` in `~/.local/state/victory/logs/`,
  and the `proton-<date>-<time>.log` of the same time beside it (Proton's own lines);
- the crash log, `crashes.log`, and the newest `.crash.log` from the `temp` folder beside it. This
  lists both:

```bash
ls -t ~/.local/share/victory/proton/drive_c/users/steamuser/AppData/Local/victory/data/*/crashes.log ~/.local/share/victory/proton/drive_c/users/steamuser/AppData/Local/victory/data/*/temp/*.crash.log
```

- what `scripts/play-nvidia.sh hlsl-check` prints.

Run with `--debug` for Proton's, vkd3d-proton's and DXVK's own logs (they go to the same
folder). Starting it from Steam as a non-Steam game, every launcher option and a troubleshooting
table are in [docs/linux-proton.md](docs/linux-proton.md).

## How it works, and the findings

- [docs/README-foxhound.md](docs/README-foxhound.md): foxhound's own README
  (set-up by hand, warming up, its switches).
- [docs/architecture.md](docs/architecture.md): the layer's parts.
- [docs/findings.md](docs/findings.md): measurements behind the choices.
- [docs/dlss.md](docs/dlss.md): DLSS and Frame Generation.
- [docs/fork-ray-tracing.md](docs/fork-ray-tracing.md): the title's ray tracing
  and the light estimate.
- [docs/asset-policy.md](docs/asset-policy.md): what may and may not be in this
  repository.
- [docs/linux-proton.md](docs/linux-proton.md) and
  [docs/proton-audit.md](docs/proton-audit.md): Linux with Proton.

## Credits

A tree of other people's work with a little on top; the history is kept.

- **[foxhound](https://github.com/readmet3xt-lab/foxhound)** by
  readmet3xt-lab: the base of this fork since 2026-11-26, and through it:
- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**,
  **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**,
  **[codec-d/cool_fox](https://github.com/codec-d/cool_fox)**,
  **[DexNom/VIctory](https://github.com/DexNom/VIctory)**,
  **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**
  and the others foxhound's README names.
- Everyone who sent pull requests to this fork before the move; that work is
  in the history, on the branch that was `main` until then.
- NVIDIA Streamline, DLSS and Reflex are NVIDIA's; they are loaded at run time
  from a folder you supply.
- Much of this fork's code was written with
  [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory and foxhound. See [LICENSE](LICENSE); the one file
that is not covered is named in [docs/asset-policy.md](docs/asset-policy.md).
