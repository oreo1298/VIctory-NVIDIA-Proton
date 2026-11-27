# Playing on Linux with Proton (NVIDIA)

This runs the same Windows `victory.exe` on Linux, through Proton: Wine, vkd3d-proton (Direct3D 12
on Vulkan), DXVK's DXGI and dxvk-nvapi. It is not a native Linux port. Everything that must
behave differently under Wine is switched at run time, and Windows behaves as before. What can go
wrong under Wine, and why, is in [proton-audit.md](proton-audit.md).

Written for CachyOS (KDE Plasma), an RTX 4090 and the current proprietary NVIDIA driver, with
**proton-cachyos** or **GE-Proton**. Proton Experimental and plain Wine work as fallbacks. As of
this writing nothing here has been run on such a machine: see
[What was tested](#what-was-tested-and-what-was-not).

## What you need

| | |
|---|---|
| GPU and driver | GeForce RTX (RTX 40 or later for Smooth Motion and DLSS frame generation) with the current proprietary driver. vkd3d-proton recommends 595-series drivers or later. |
| Proton | proton-cachyos or GE-Proton (both on Valve's Proton 10 tree), or Proton Experimental. Each bundles vkd3d-proton, DXVK and dxvk-nvapi. |
| Kernel | Linux 6.7 or later, so Wine's kernel write watches work (see [Write watches](#write-watches)). Linux 6.14 or later for ntsync. Current CachyOS kernels have both. |
| `vm.max_map_count` | 1048576 or more (Arch and CachyOS ship 1048576). The launcher warns when it is lower. |
| Memory | 32 GB or more, as on Windows. |
| Launcher | `umu-launcher` (recommended outside Steam), or Steam, or a Proton folder in `$PROTON`. |
| To build | rustup, `cargo-xwin` (`cargo install cargo-xwin --locked`), clang and lld. |
| Your dump | your own dump of the title's Xbox Series build v1.0.2.1, outside the repository. |

vkd3d-proton features the layer relies on, all present on NVIDIA with a current driver:
Vulkan 1.3; descriptor indexing with 1,000,000 or more update-after-bind descriptors; shader
model 6.6 with `ResourceDescriptorHeap` indexing; mesh shaders (`VK_EXT_mesh_shader`); 32-lane
waves; 64-bit atomics; enhanced barriers; sparse (tiled) resources; `VK_EXT_external_memory_host`
(only for the in-place paths, which are off by default). The title's ray tracing is off by
default, as on Windows. No `VKD3D_CONFIG` option is needed or set.

## Build

```bash
cargo install cargo-xwin --locked
```

```bash
scripts/build-linux.sh
```

This makes `target/x86_64-pc-windows-msvc/release/victory.exe`. `scripts/build-linux.sh diag`
makes the diagnostics build in `target/x86_64-pc-windows-msvc/diag/`. The first build downloads
Microsoft's CRT and Windows SDK import libraries into `~/.cache/cargo-xwin`; they are Microsoft's,
under Microsoft's licence. The build uses `--locked`. A release `victory.exe` built on Windows
runs the same way.

`scripts/build-linux.sh release --gnu` builds `x86_64-pc-windows-gnu` with mingw-w64 instead.
It builds and its CPU-only commands run under Wine, but it is not the supported path:

- `build.rs` adds the `D3D12SDKVersion`/`D3D12SDKPath` exports and the delay-load of
  `d3d12.dll` only for MSVC. That matters on Windows (Agility SDK) and not under vkd3d-proton.
- It links mingw's `msvcrt.dll` instead of the UCRT.
- It carries DWARF debug information (about 78 MB).

## Set up

1. `victory.toml` (copy `victory.example.toml`) can hold Unix paths. Under Wine an absolute
   Unix path is read as its `Z:` form, so `game_dir = '/home/me/Games/MyDump'` works, and so do
   `VICTORY_GAME_DIR` and the other path variables.
2. **DLSS (optional).** `Get-NVIDIA-DLSS.cmd` is PowerShell and does not run here. Instead,
   download NVIDIA's Streamline SDK release `v2.14.16` (the version the Windows fetcher uses)
   from <https://github.com/NVIDIA-RTX/Streamline/releases>, under NVIDIA's licence, which you
   accept yourself. Unpack it **outside** the repository and point the layer at its `bin/x64`:

   ```bash
   export VICTORY_STREAMLINE_DIR=$HOME/sdk/streamline-sdk-v2.14.16/bin/x64
   ```

   Or unpack the archive anywhere under `nvidia-sdk/` in the repository folder (git-ignored;
   `unzip -q streamline-sdk-v2.14.16.zip -d nvidia-sdk`).
   The launcher finds `bin/x64/sl.interposer.dll` there by itself, skipping the SDK's
   development build. DLSS under Proton needs NVAPI, which the launcher turns on.
3. **Check the helper shaders once per Proton version:**

   ```bash
   scripts/play-nvidia.sh hlsl-check
   ```

   See [Helper shaders and d3dcompiler_47](#helper-shaders-and-d3dcompiler_47). The launcher
   passes `hlsl-check`, `upscaler-check`, `config`, `shaders` and `warm` on to `victory.exe`,
   with the same runner and environment as a run, and `winetricks` to winetricks in the same
   prefix.

## Play

```bash
scripts/play-nvidia.sh --exe ~/Games/MyDump/game_scarlett_master.exe
```

| Option | What |
|---|---|
| *(none)* | Series X profile, 2560x1440, DLSS Quality (with NVIDIA's files), Smooth Motion on, in-game frame generation ready but off |
| `--series-s` / `--series-x` | the console profile (Series X is the default) |
| `--res 1080to1440` / `4k`, `--dlss off` / `dlaa` / `balanced` / `performance`, `--framegen [2-6]`, `--no-framegen-ready`, `--fg-type auto` / `dlss` / `fsr`, `--fps N`, `--preload`, `--build diag`, `--set VICTORY_X=1` | as `play-nvidia.ps1`'s `-Res`, `-Dlss`, `-FrameGen`/`-FrameGenX`, `-NoFrameGenReady`, `-FgType`, `-Fps`, `-Preload`, `-Build`, `-More` |
| `--no-smooth-motion` | without the driver's Smooth Motion |
| `--debug` | also `PROTON_LOG=1`, `VKD3D_DEBUG=warn` with `VKD3D_LOG_FILE`, `DXVK_LOG_LEVEL=info`, `NVPRESENT_LOG_LEVEL=4`, `VK_LOADER_DEBUG=layer` (logs in `~/.local/state/victory/logs`) |
| `--victory PATH` | another `victory.exe` (e.g. a release download); `VICTORY_EXE` does the same |
| `--prefix DIR` | the Proton prefix (default `~/.local/share/victory/proton`; never inside the repository) |
| `--dry-run` | print the runner, the arguments and the environment, and run nothing |
| `winetricks VERB...` | winetricks in the game's prefix, with the same Proton (not from Steam: use protontricks there) |

How it starts `victory.exe`, the first that is there:

1. what follows `--` (Steam's `%command%`, below);
2. **umu-run**. `PROTONPATH` picks the Proton; else `$PROTON`; else the newest proton-cachyos
   (`/usr/share/steam/compatibilitytools.d/`) or GE-Proton (Steam's `compatibilitytools.d`) found;
   else umu's own. `GAMEID` defaults to `umu-victory`, and `WINEPREFIX` is the prefix above;
3. the Proton folder in **`$PROTON`**, or one found as above. It runs `"$PROTON/proton" run`,
   with `STEAM_COMPAT_DATA_PATH` set to the prefix and `STEAM_COMPAT_CLIENT_INSTALL_PATH` to your
   Steam folder;
4. plain **wine**, with a warning: vkd3d-proton, DXVK and dxvk-nvapi must then be installed into
   its prefix.

The launcher always sets `PROTON_ENABLE_NVAPI=1` and `DXVK_ENABLE_NVAPI=1`. Without them DXVK
reports the RTX 4090 as an AMD card (vendor 0x1002), and DLSS and Reflex cannot load. It sets
`VKD3D_SHADER_CACHE_PATH` to `~/.cache/victory/vkd3d-proton`, because vkd3d-proton otherwise
writes its cache into the current folder, which is the repository's. It also keeps the NVIDIA
driver's shader cache from being trimmed (`__GL_SHADER_DISK_CACHE_SKIP_CLEANUP=1`). The F1 menu
works as on Windows: SAVE AND RESTART and PRELOAD SHADERS end the run with exit code 75 (76 to
preload first), and the launcher starts the game again.

### Paths: `Z:\` rather than winepath

The launcher passes the title's executable and every path setting in their `Z:\...` form. Wine and
Proton always map `Z:` to `/`. It is exact, instant and needs nothing running. `winepath` would
need a Wine of the same prefix, which under umu or Steam means starting a whole Proton session per
path. For paths that do not come from the launcher (`victory.toml`, variables you export yourself),
`victory.exe` does the same conversion under Wine (`victory_core::wine::windows_path`).

### As a non-Steam game in Steam

1. Steam: *Games → Add a Non-Steam Game*, and choose `victory.exe`
   (`target/x86_64-pc-windows-msvc/release/victory.exe`).
2. *Properties → Compatibility*: force proton-cachyos or GE-Proton.
3. *Properties → Launch options*:

   ```bash
   /path/to/VIctory-NVIDIA-Proton/scripts/play-nvidia.sh --exe /path/to/MyDump/game_scarlett_master.exe -- %command%
   ```

The launcher sets everything above, then runs Steam's command with `run <game> --console ...
--execute` after it. The prefix is then Steam's, in `steamapps/compatdata/<shortcut id>/pfx`. Add
`--no-smooth-motion`, `--series-s` and the other options before `--`.

## Smooth Motion

On Windows this fork has no Smooth Motion code; the Windows driver applies it from a profile. On
Linux the driver's Vulkan present layer (`VK_LAYER_NV_present`, RTX 40 and later) turns it on with
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`. The launcher sets it by default. `--no-smooth-motion` leaves it
unset. It is also left off when you ask for the game's own frame generation (`--framegen`), so two
frame generators never stack. To check that the layer loaded, run with `--debug` and look for
`VK_LAYER_NV_present` in the Vulkan loader's lines and for `NVPRESENT` lines in the launcher's log.

## Shaders and pipelines ahead

```bash
scripts/play-nvidia.sh shaders ~/Games/MyDump
```

```bash
scripts/play-nvidia.sh warm --used
```

`shaders` translates every shader into the layer's cache. `warm --used` makes the pipelines that
earlier runs drew with, so the driver has them compiled. Both get the same switches as a run, since
some of them are part of the translation's key. Run both after each new build, and `warm --used`
again after a vkd3d-proton (Proton) or driver update.

Under vkd3d-proton the application's `ID3D12PipelineLibrary` is a stand-in while vkd3d-proton's own
cache is on (the default). The pipelines `warm` makes land in vkd3d-proton's `vkd3d-proton.cache`
and the NVIDIA driver's cache instead, which is where the time goes anyway. A pipeline library from
another vkd3d-proton or driver is refused, and that counts as a miss, as on Windows.

## Where things are

`<prefix>` is `~/.local/share/victory/proton` (umu: the prefix itself; `$PROTON`: `<prefix>/pfx`)
or, from Steam, `steamapps/compatdata/<id>/pfx`. `%LOCALAPPDATA%` there is
`drive_c/users/steamuser/AppData/Local`.

| What | Where |
|---|---|
| Translated shaders, pipeline book and library | `<prefix>/drive_c/users/steamuser/AppData/Local/victory/cache` |
| Saves | `.../AppData/Local/victory/data` |
| Crash log | `.../AppData/Local/victory/data/7952987A/crashes.log` (and `temp/*.crash.log` beside it; the folder name is lower case) |
| vkd3d-proton's cache | `~/.cache/victory/vkd3d-proton/vkd3d-proton.cache` |
| NVIDIA's cache | `~/.cache/nvidia/GLCache` (or `__GL_SHADER_DISK_CACHE_PATH`) |
| The game's log (`victory-nvidia-*.log`), Proton's output (`proton-*.log`), `--debug` logs | `~/.local/state/victory/logs` |
| F1 menu settings | `victory-settings.txt` in the repository folder (git-ignored) |

## Helper shaders and d3dcompiler_47

The layer compiles a few dozen small helper shaders of its own when it starts: the
indirect-argument clamp, the interface plane, the light estimate and others. Under Proton the
system's HLSL compiler is Wine's own `d3dcompiler_47`, which is vkd3d-shader's compiler.

- It ignores the root signatures written into the HLSL. The layer then makes them from the
  source's `#define RS "..."` string itself (`victory-gpu/src/rootsig.rs`; logged once as "made
  from their sources' strings instead").
- It may not compile every helper. `hlsl-check` lists them. A helper that does not compile is
  switched off. For the indirect clamp that means "no argument clamp: indirect execution dropped",
  and the game draws almost nothing.

If `hlsl-check` shows any `NO`, put Microsoft's `d3dcompiler_47.dll` into the prefix. The
launcher runs winetricks with the same runner, Proton and prefix as the game (umu-run, or the
`$PROTON` folder's own wine):

```bash
scripts/play-nvidia.sh winetricks d3dcompiler_47
```

From Steam, `protontricks <id> d3dcompiler_47` (`protontricks -l` lists the shortcut's id).
Afterwards `hlsl-check` should show `yes` in both of its first columns.

## Write watches

The layer notices which of the title's GPU memory the title wrote through Windows' write watching.
Wine has two ways to provide it, and every run logs which one it got and what a first write cost:

- `Wine write watches: in the kernel (userfaultfd), cheap`. Linux 6.7 or later and a Wine that has
  them (upstream since 10.7; expected in current Proton-10-based builds).
- `... by write protection and a signal per first write`, with a warning. Every first write is a
  signal round trip, measured here at about 2.2 µs; with several thousand a frame that is about
  16 ms of CPU a frame. Update Proton or the kernel.

## Troubleshooting

| What you see | Where to look | What to do |
|---|---|---|
| Next to nothing drawn; `no argument clamp: indirect execution dropped` in the log | `hlsl-check` | Microsoft's `d3dcompiler_47` in the prefix (above) |
| `DXGI reports this NVIDIA card as AMD` | the log's `GPU vendor` line | `PROTON_ENABLE_NVAPI=1 DXVK_ENABLE_NVAPI=1` (the launcher sets them; from Steam, through the launcher) |
| No DLSS: `not validly signed as Wine checks it` | the log | Wine could not verify NVIDIA's signature. If the files are NVIDIA's own, `--set VICTORY_WINE_TRUST_UNVERIFIED=1` |
| No DLSS: Streamline loads, DLSS not supported | `scripts/play-nvidia.sh upscaler-check --device` | NVAPI on; Proton's `nvngx.dll` in `<prefix>/drive_c/windows/system32` |
| `the process uses half of vm.max_map_count` | the log | `sudo sysctl -w vm.max_map_count=2147483642`, and in `/etc/sysctl.d/` |
| `write watches: by write protection and a signal per first write` | the log | a newer Proton (proton-cachyos, GE-Proton) and kernel |
| Mouse look stutters or stops (KDE Wayland) | | the X11 session, gamescope, or `PROTON_ENABLE_WAYLAND=1` with proton-cachyos/GE |
| Stutter in the first minutes of a new build | | `shaders` and `warm --used` (above) |
| A 5 s pause when quitting | | expected: the exit watchdog ends the process when vkd3d-proton's threads do not detach |
| The game does not start again after F1's SAVE AND RESTART | the launcher's window | only the launcher restarts it: start through `play-nvidia.sh` |
| `Proton: Error: unable to use parent for game drive`, `pressure-vessel-wrap: W: Not sharing path ... /usr`, ProtonFixes' `fsr4` lines | `proton-*.log` | harmless: Proton's game-drive mapping, the Steam runtime not mounting `/usr` (proton-cachyos-slr lives there and still runs), and proton-cachyos fetching AMD's FSR 4 DLL into the prefix |

When something fails, keep these:

1. the game's log `~/.local/state/victory/logs/victory-nvidia-<date>-<time>.log`, and
   `proton-<date>-<time>.log` beside it;
2. the crash log `<prefix>/drive_c/users/steamuser/AppData/Local/victory/data/7952987A/crashes.log`,
   and the newest `.crash.log` in `temp/` beside it;
3. with `--debug`: `vkd3d-proton-<date>-<time>.log`, `steam-*.log` (Proton), and DXVK's
   `victory_dxgi.log`, all in `~/.local/state/victory/logs`;
4. the output of `scripts/play-nvidia.sh hlsl-check`, `nvidia-smi`, and `uname -r`.

## Known differences from Windows

- Direct3D 12 is vkd3d-proton, so the Agility SDK (`D3D12\D3D12Core.dll`, `get-d3d12`) does not
  apply, and `victory play` does not offer it under Wine.
- No PowerShell: `get-dlss`, `get-fsr`, `get-mfg` and `get-d3d12` do not run. Fetch on Windows,
  or as under *Set up*.
- `SetThreadSelectedCpuSets` is a no-op in Wine, so `VICTORY_FAST_CORES` asks for nothing there.
  To keep the game on the 9950X3D's cache CCD, use `taskset`, or the kernel's `amd_3d_vcache`
  driver (`amd_x3d_mode` set to `cache`).
- Raised thread priorities take effect only where Wine may raise them (`RLIMIT_NICE`).
- Device-removed data (`VICTORY_D3D12_DRED`) is mostly empty under vkd3d-proton. Use
  `VKD3D_CONFIG=breadcrumbs` with a debug build of vkd3d-proton instead.
- Crash stack walks stop at the Windows/Linux boundary (inside the driver).
- The first run of a build is not "about 14 fps until restarted" as on Windows: that is the
  Windows driver's per-program cache. `victory play`'s warm-up restart does not apply to this
  launcher.

## Switches added for Proton

| Switch | What |
|---|---|
| `VICTORY_PROTON=0\|1` | Proton paths off or on whatever the detection says (detection: ntdll's `wine_get_version`) |
| `VICTORY_GPU_VENDOR=auto\|nvidia\|amd\|intel\|other` | the vendor the layer takes the card for; `auto` uses DXGI on Windows and the Vulkan device under Wine |
| `VICTORY_WINE_TRUST_UNVERIFIED=1` | under Wine only, a DLL whose signature Wine cannot verify is loaded anyway, with a warning |
| `VICTORY_OUTPUT_FILE=<path>` | everything `victory.exe` prints, its log included, goes to that file (under Proton the console reaches no terminal); set by `play-nvidia.sh`, which shows the file as it grows |
| `VICTORY_RESTART_EXIT_CODE=<n>` | the F1 menu ends the run with exit code n (n+1: preload first) instead of starting PowerShell; set by `play-nvidia.sh` |

## What was tested, and what was not

Tested in a Linux container without a GPU:

- the cross-compiled build;
- `victory.exe --help`, `config` and `hlsl-check` under Wine 9.0;
- the Wine detection, its log line and the write-watch test;
- the Unix-to-`Z:` path conversion;
- the parser of the helpers' root signatures (unit tests under Wine). On Wine's own D3D12 on Mesa
  lavapipe, 16 of 19 strings became root signatures; the 3 others need more than lavapipe's 8
  Vulkan descriptor sets, a limit of upstream vkd3d there, not of vkd3d-proton;
- the launcher's choice of runner, its arguments and environment (`--dry-run`), its restart loop
  and shellcheck.

Not tested: anything on an NVIDIA GPU under Proton, including whether Proton's vkd3d-shader
compiles every helper.
