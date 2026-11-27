#!/usr/bin/env bash
# Starts your own copy of the title on Linux, through Proton (or Wine), with the settings scripts/play-nvidia.ps1
# (play-foxhound.ps1) starts it with on Windows. The program is the Windows victory.exe (scripts/build-linux.sh
# builds it); Proton's vkd3d-proton, DXVK and dxvk-nvapi run it. docs/linux-proton.md has the details.
#
#   scripts/play-nvidia.sh --exe ~/Games/MyDump/game_scarlett_master.exe
#   scripts/play-nvidia.sh --exe ... --series-s            Series S profile (1280x720 inside, faster, softer)
#   scripts/play-nvidia.sh --exe ... --no-smooth-motion    without the driver's Smooth Motion frame generation
#   scripts/play-nvidia.sh --exe ... --debug               Proton, vkd3d-proton, DXVK and Vulkan loader logs too
#   scripts/play-nvidia.sh shaders [folder]                translate every shader into the cache (once a build)
#   scripts/play-nvidia.sh warm --used                     make the pipelines earlier runs drew with (once a build)
#   scripts/play-nvidia.sh hlsl-check                      can this Proton compile the layer's helper shaders?
#                                    (also upscaler-check [--device] and config, passed on to victory.exe as these)
#   scripts/play-nvidia.sh winetricks d3dcompiler_47      winetricks in the same prefix, with the same Proton
#   scripts/play-nvidia.sh --exe ... -- %command%          from Steam's launch options (non-Steam game; see the doc)
#
# Options (as the PowerShell launcher's): --series-x (the default) | --series-s, --res 1440|1080to1440|4k,
# --dlss off|dlaa|quality|balanced|performance, --framegen [2-6], --no-framegen-ready, --fg-type auto|dlss|fsr,
# --fps N, --preload, --build release|diag, --set NAME=VALUE (any switch, as often as needed).
# Linux only: --no-smooth-motion, --debug, --victory PATH (the victory.exe to start), --prefix DIR (the Proton
# prefix; default ~/.local/share/victory/proton), --dry-run (print what would run, run nothing).
#
# How it runs, the first that is there: what follows `--` (Steam's %command%); umu-run (PROTONPATH picks the
# Proton, else $PROTON or a proton-cachyos/GE-Proton found on this PC, else umu's own); the Proton in $PROTON (or
# one found); plain wine (a warning: it needs vkd3d-proton, DXVK and dxvk-nvapi installed into its prefix).
#
# In the game, F1 opens the settings menu. What it saves goes to victory-settings.txt in the repository's folder
# and is applied here at every start, over the options above; SAVE AND RESTART ends the run and this script
# starts it again. Each run writes a log to ~/.local/state/victory/logs. Every VICTORY_... switch is described
# in a comment where the code reads it.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
script="$repo/scripts/$(basename "${BASH_SOURCE[0]}")"

# Exit codes the F1 menu ends a run with to be started again (VICTORY_RESTART_EXIT_CODE; +1: preload first).
restart_code=75

say() { printf '%s\n' "$*" >&2; }
die() {
    say "play-nvidia.sh: $*"
    exit 1
}
usage() {
    sed -n '2,33p' "$script" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

# `Z:\...` for an absolute Unix path (Wine and Proton map Z: to /); anything else as it is. This is what the
# Windows program reads; see docs/linux-proton.md for why this and not winepath.
dos() {
    local p=$1
    if [[ $p == /* && $p != //* ]]; then
        p=${p//\//\\}
        printf 'Z:%s' "$p"
    else
        printf '%s' "$p"
    fi
}
# The Unix path of a `Z:\...` path (to check it exists); other drive letters as they are.
unix() {
    local p=$1
    if [[ $p == [Zz]:\\* ]]; then
        p=${p:2}
        printf '%s' "${p//\\//}"
    else
        printf '%s' "$p"
    fi
}
inside_repo() {
    local p
    p=$(realpath -m -- "$1")
    [[ $p == "$repo" || $p == "$repo"/* ]]
}

exe=""
victory=""
build=release
series_s=0
res=1440
dlss=quality
framegen=0
times=2
no_fg_ready=0
fg_type=auto
fps=1000
preload=0
smooth=1
debug=0
dry_run=0
prefix=""
command=run
more=()
pass=()
steam=()

value() {
    [[ $# -ge 2 && -n $2 ]] || die "$1 needs a value (see --help)"
}
while (($#)); do
    case $1 in
        --exe) value "$@"; exe=$2; shift ;;
        --exe=*) exe=${1#*=} ;;
        --victory) value "$@"; victory=$2; shift ;;
        --build) value "$@"; build=$2; shift ;;
        --series-s) series_s=1 ;;
        --series-x) series_s=0 ;;
        --res) value "$@"; res=$2; shift ;;
        --dlss) value "$@"; dlss=$2; shift ;;
        --framegen)
            framegen=1
            if [[ ${2:-} =~ ^[2-6]$ ]]; then
                times=$2
                shift
            fi
            ;;
        --no-framegen-ready) no_fg_ready=1 ;;
        --fg-type) value "$@"; fg_type=$2; shift ;;
        --fps) value "$@"; fps=$2; shift ;;
        --preload) preload=1 ;;
        --set) value "$@"; more+=("$2"); shift ;;
        --no-smooth-motion) smooth=0 ;;
        --debug) debug=1 ;;
        --dry-run) dry_run=1 ;;
        --prefix) value "$@"; prefix=$2; shift ;;
        -h | --help) usage 0 ;;
        shaders | warm | hlsl-check | upscaler-check | config | winetricks)
            command=$1
            shift
            pass=("$@")
            break
            ;;
        --)
            shift
            steam=("$@")
            break
            ;;
        *) die "unknown option $1 (see --help)" ;;
    esac
    shift
done

[[ $build == release || $build == diag ]] || die "--build is release or diag"
[[ $res =~ ^(1440|1080to1440|4k)$ ]] || die "--res is 1440, 1080to1440 or 4k"
[[ $dlss =~ ^(off|dlaa|quality|balanced|performance)$ ]] || die "--dlss is off, dlaa, quality, balanced or performance"
[[ $fg_type =~ ^(auto|dlss|fsr)$ ]] || die "--fg-type is auto, dlss or fsr"
if ! [[ $fps =~ ^[0-9]+$ ]] || ((fps < 1 || fps > 1000)); then die "--fps is 1 to 1000"; fi
# The F1 menu's PRELOAD SHADERS (and a restart that asks for one) sets VICTORY_PRELOAD=1.
[[ ${VICTORY_PRELOAD:-} == 1 ]] && preload=1

# What the F1 menu saved: its own choices (VICTORY_MENU_...) stand in for the options above, and its switches go
# over the settings below. An empty value unsets a switch.
menu_file="$repo/victory-settings.txt"
declare -A menu=()
menu_keys=()
if [[ -f $menu_file ]]; then
    while IFS= read -r line || [[ -n $line ]]; do
        line=${line%$'\r'}
        if [[ $line =~ ^[[:space:]]*(VICTORY_[A-Z0-9_]+)[[:space:]]*=(.*)$ ]]; then
            key=${BASH_REMATCH[1]}
            val=${BASH_REMATCH[2]}
            val=${val#"${val%%[![:space:]]*}"}
            val=${val%"${val##*[![:space:]]}"}
            [[ -v menu[$key] ]] || menu_keys+=("$key")
            menu[$key]=$val
        fi
    done <"$menu_file"
fi
[[ -v menu[VICTORY_MENU_CONSOLE] ]] && { [[ ${menu[VICTORY_MENU_CONSOLE]} == series-s ]] && series_s=1 || series_s=0; }
[[ ${menu[VICTORY_MENU_RES]:-} =~ ^(1440|1080to1440|4k)$ ]] && res=${menu[VICTORY_MENU_RES]}
[[ ${menu[VICTORY_MENU_DLSS]:-} =~ ^(off|dlaa|quality|balanced|performance)$ ]] && dlss=${menu[VICTORY_MENU_DLSS]}
if [[ -v menu[VICTORY_MENU_FRAMEGEN] ]]; then
    # 0: off; 2 to 6: that many frames shown for each one rendered (1, from an older file: 2).
    [[ ${menu[VICTORY_MENU_FRAMEGEN]} =~ ^[1-6]$ ]] && framegen=1 || framegen=0
    [[ ${menu[VICTORY_MENU_FRAMEGEN]} =~ ^[2-6]$ ]] && times=${menu[VICTORY_MENU_FRAMEGEN]}
fi
[[ ${menu[VICTORY_MENU_FG_TYPE]:-} =~ ^(auto|dlss|fsr)$ ]] && fg_type=${menu[VICTORY_MENU_FG_TYPE]}
if [[ ${menu[VICTORY_MENU_FPS]:-} =~ ^[0-9]+$ ]] && ((menu[VICTORY_MENU_FPS] >= 1 && menu[VICTORY_MENU_FPS] <= 1000)); then
    fps=${menu[VICTORY_MENU_FPS]}
fi

# The settings tested on Windows (play-foxhound.ps1 has the same, with what each is for; keep the two alike).
declare -A s=(
    [VICTORY_THIN]=1 [VICTORY_JOIN_PARTS]=8 [VICTORY_RELAY]=2 [VICTORY_RELAY_COPY]=1 [VICTORY_WIDE_BACK]=1
    [VICTORY_CARD_REHASH_RUNS]=1 [VICTORY_JOIN_COPIES]=1 [VICTORY_CARD_STEPS]=2 [VICTORY_CARD_AWAKE]=1
    [VICTORY_CARD_BUFFERS_MIN_KB]=64 [VICTORY_INDIRECT_WINDOW]=1 [VICTORY_INDIRECT_BATCH]=2
    [VICTORY_UPLOAD_AHEAD]=1 [VICTORY_SHOW_AFTER]=1 [VICTORY_MAP_THREADS]=1
    [VICTORY_WC_NO_BACK]=1 [VICTORY_WC_NO_BACK_MIN_KB]=1024
    [VICTORY_SOFT_VIEWS]=copies
    [VICTORY_TEXTURE_POOL]=1 [VICTORY_RIPE_BLOCKS]=0 [VICTORY_TEXTURE_NO_WAIT]=1 [VICTORY_TEXTURE_CATCHUP]=0
    [VICTORY_TEXTURE_STAGE_MS]=16 [VICTORY_LEVELS_ON_DEMAND]=0
    [VICTORY_PRELUDE_UPLOADS]=1 [VICTORY_SHOW_NEW_LATER]=1 [VICTORY_POOL_SMALL]=0 [VICTORY_TABLED_REREAD]=1
    [VICTORY_QUEUE_HOST_TEXTURES]=0
    [VICTORY_HEAP_SYNC_FIRST]=1 [VICTORY_SYNC_IN_PLACE]=1 [VICTORY_SPANS_QUICK]=1 [VICTORY_SYNC_FINE]=1
    [VICTORY_HEAP_SYNC_AT_SUBMIT]=1
    [VICTORY_AMBIENT]=0.8 [VICTORY_AMBIENT_AIR]=1.5 [VICTORY_AMBIENT_SPECULAR]=1
    [VICTORY_EARLY_DEPTH]=1
    [VICTORY_LEAVE_OUT]="02384a60fda6f2d0,731a39171b568298"
    [VICTORY_PIPELINE_BOOK]=1
)
if ((!series_s)); then
    if [[ $res == 1440 ]]; then
        s[VICTORY_NATIVE_1440P]=1
    elif [[ $res == 1080to1440 ]]; then
        s[VICTORY_RENDER_1080P]=1
    fi
fi
if [[ $dlss != off ]]; then
    # NVIDIA's files: where VICTORY_STREAMLINE_DIR says, else where Get-NVIDIA-DLSS.cmd puts them.
    sl=${VICTORY_STREAMLINE_DIR:-$repo/nvidia-sdk/Streamline/bin/x64}
    # Or wherever NVIDIA's SDK was unpacked under nvidia-sdk/ (not its development build).
    if [[ -z ${VICTORY_STREAMLINE_DIR:-} && ! -f $sl/sl.interposer.dll && -d $repo/nvidia-sdk ]]; then
        found=$(find "$repo/nvidia-sdk" -path '*/bin/x64/sl.interposer.dll' ! -path '*development*' -print -quit 2>/dev/null)
        [[ -n $found ]] && sl=$(dirname "$found")
    fi
    if [[ -f $(unix "$sl")/sl.interposer.dll ]]; then
        s[VICTORY_DLSS]=1
        s[VICTORY_DLSS_MODE]=$dlss
        s[VICTORY_STREAMLINE_DIR]=$(dos "$(unix "$sl")")
        if ((framegen)); then
            s[VICTORY_FRAMEGEN]=1
            s[VICTORY_FRAMEGEN_X]=$times
        elif ((!no_fg_ready)); then
            s[VICTORY_FRAMEGEN]=1
            s[VICTORY_FRAMEGEN_X]=0 # ready, off
        fi
        # Frame generation takes 1.5 to 2 GB of the card's memory: on from the start it is left a fifth of the
        # budget; ready but off, a little less is held back.
        if [[ ${s[VICTORY_FRAMEGEN]:-} == 1 ]]; then
            ((framegen)) && s[VICTORY_TEXTURE_ROOM_PERCENT]=80 || s[VICTORY_TEXTURE_ROOM_PERCENT]=86
        fi
    else
        say "No NVIDIA files (sl.interposer.dll) under $repo/nvidia-sdk: running without DLSS and frame"
        say "generation. Unpack NVIDIA's Streamline SDK there (README, Linux, step 7) or set VICTORY_STREAMLINE_DIR."
    fi
fi
# AMD FSR 3 frame generation: made ready as DLSS Frame Generation is. auto: the layer uses it only where DLSS
# Frame Generation does not run; fsr: DLSS Frame Generation is not asked.
if [[ $fg_type != dlss ]] && ((framegen || !no_fg_ready)); then
    fx=${VICTORY_FIDELITYFX_DIR:-$repo/amd-sdk/FidelityFX/bin}
    s[VICTORY_FSR_FRAMEGEN]=1
    s[VICTORY_FIDELITYFX_DIR]=$(dos "$(unix "$fx")")
    if [[ ! -f $(unix "$fx")/amd_fidelityfx_framegeneration_dx12.dll ]] && ((framegen)); then
        say "No AMD FSR 3 files in $fx (needed only without DLSS frame generation, or with --fg-type fsr)."
    fi
    [[ $fg_type == fsr ]] && unset 's[VICTORY_FRAMEGEN]'
    if [[ ! -v s[VICTORY_FRAMEGEN_X] ]]; then
        ((framegen)) && s[VICTORY_FRAMEGEN_X]=$times || s[VICTORY_FRAMEGEN_X]=0
    fi
    if [[ ! -v s[VICTORY_TEXTURE_ROOM_PERCENT] ]]; then
        ((framegen)) && s[VICTORY_TEXTURE_ROOM_PERCENT]=80 || s[VICTORY_TEXTURE_ROOM_PERCENT]=86
    fi
fi
s[VICTORY_FRAME_INTERVAL_US]=$(((1000000 + fps / 2) / fps))
for k in "${menu_keys[@]}"; do
    [[ $k == VICTORY_MENU_* ]] && continue
    if [[ -n ${menu[$k]} ]]; then s[$k]=${menu[$k]}; else unset "s[$k]"; fi
done
for m in "${more[@]}"; do
    [[ $m == VICTORY_*=* ]] || die "--set takes NAME=VALUE with a VICTORY_ name: $m"
    s[${m%%=*}]=${m#*=}
done
console=series-x
((series_s)) && console=series-s
# For the menu: what it shows as the current choices, where it saves, and that this script starts the game again.
s[VICTORY_MENU_CONSOLE]=$console
s[VICTORY_MENU_RES]=$res
[[ ${s[VICTORY_DLSS]:-} == 1 ]] && s[VICTORY_MENU_DLSS]=$dlss || s[VICTORY_MENU_DLSS]=off
if [[ ${s[VICTORY_FRAMEGEN]:-} == 1 || ${s[VICTORY_FSR_FRAMEGEN]:-} == 1 ]]; then
    s[VICTORY_MENU_FRAMEGEN]=${s[VICTORY_FRAMEGEN_X]:-0}
else
    s[VICTORY_MENU_FRAMEGEN]=0
fi
s[VICTORY_MENU_FG_TYPE]=$fg_type
s[VICTORY_MENU_FPS]=$fps
s[VICTORY_MENU_FILE]=$(dos "$menu_file")
s[VICTORY_RESTART_EXIT_CODE]=$restart_code

# Switches left over in this shell are cleared; path settings and the Proton switches stay, paths in Z: form.
kept=(VICTORY_CONFIG VICTORY_GAME_DIR VICTORY_CACHE_DIR VICTORY_DATA_DIR VICTORY_HOME VICTORY_STREAMLINE_DIR
    VICTORY_FIDELITYFX_DIR VICTORY_LOG VICTORY_PROTON VICTORY_GPU_VENDOR VICTORY_WINE_TRUST_UNVERIFIED)
while IFS= read -r name; do
    keep=0
    for k in "${kept[@]}"; do [[ $name == "$k" ]] && keep=1; done
    if ((keep)); then
        [[ $name == VICTORY_LOG || $name == VICTORY_PROTON || $name == VICTORY_GPU_VENDOR ]] || export "$name=$(dos "${!name}")"
    else
        unset "$name"
    fi
done < <(compgen -e | grep '^VICTORY_' || true)
for k in "${!s[@]}"; do export "$k=${s[$k]}"; done

# Proton: NVAPI on, so DXVK reports the card as NVIDIA (not AMD) and DLSS/Reflex can load.
export PROTON_ENABLE_NVAPI=1 DXVK_ENABLE_NVAPI=1
# The driver's Smooth Motion (its Vulkan present layer, RTX 40 and later) in place of the Windows driver profile.
# Not with the game's own frame generation on: two frame generators on one another.
if ((smooth)) && ((framegen)); then
    say "Smooth Motion left off: the game's own frame generation is on (--framegen)."
    smooth=0
fi
if ((smooth)); then
    export NVPRESENT_ENABLE_SMOOTH_MOTION=1
else
    unset NVPRESENT_ENABLE_SMOOTH_MOTION
fi

state=${XDG_STATE_HOME:-$HOME/.local/state}/victory
cache=${XDG_CACHE_HOME:-$HOME/.cache}/victory
logs=$state/logs
# vkd3d-proton keeps its DXIL->SPIR-V cache in the current folder unless told: here that is the repository's,
# and those are translations of the game's shaders (docs/asset-policy.md). The path is read by Windows code.
export VKD3D_SHADER_CACHE_PATH=${VKD3D_SHADER_CACHE_PATH:-$(dos "$cache/vkd3d-proton")}
inside_repo "$(unix "$VKD3D_SHADER_CACHE_PATH")" && die "VKD3D_SHADER_CACHE_PATH is inside the repository; keep caches outside it"
# The NVIDIA driver's own pipeline cache: kept whole (it otherwise trims to its size limit).
export __GL_SHADER_DISK_CACHE_SKIP_CLEANUP=${__GL_SHADER_DISK_CACHE_SKIP_CLEANUP:-1}
stamp=$(date +%Y%m%d-%H%M%S)
if ((debug)); then
    export PROTON_LOG=1 PROTON_LOG_DIR=$logs VKD3D_DEBUG=warn NVPRESENT_LOG_LEVEL=4 VK_LOADER_DEBUG=layer
    export VKD3D_LOG_FILE
    VKD3D_LOG_FILE=$(dos "$logs/vkd3d-proton-$stamp.log")
    export DXVK_LOG_LEVEL=info DXVK_LOG_PATH
    DXVK_LOG_PATH=$(dos "$logs")
fi

# Linux refuses mappings past vm.max_map_count; the title's memory views and guards take many.
max_maps=$(cat /proc/sys/vm/max_map_count 2>/dev/null || echo 0)
if ((max_maps < 1048576)); then
    say "vm.max_map_count is $max_maps: below 1048576 the game can run out of memory mappings. Raise it:"
    say "  sudo sysctl -w vm.max_map_count=1048576   (and in /etc/sysctl.d/ to keep it)"
fi

# The victory.exe to start (not needed from Steam, whose shortcut names it, nor for winetricks).
if ((${#steam[@]} == 0)) && [[ $command != winetricks ]]; then
    if [[ -z $victory ]]; then
        victory=${VICTORY_EXE:-$repo/target/x86_64-pc-windows-msvc/$build/victory.exe}
    fi
    if [[ ! -f $victory ]]; then
        command -v cargo >/dev/null 2>&1 || die "no $victory, and no cargo to build it with (docs/linux-proton.md)"
        say "Building the layer (a few minutes the first time) ..."
        "$repo/scripts/build-linux.sh" "$build"
        [[ -f $victory ]] || die "the build did not make $victory"
    fi
fi

# The arguments victory.exe gets.
if [[ $command == run ]]; then
    [[ -n $exe ]] || die "--exe <your copy's game_scarlett_master.exe> is needed (see --help)"
    [[ -f $exe ]] || die "no $exe"
    exe=$(realpath -- "$exe")
    args=(run "$(dos "$exe")" --console "$console" --execute)
elif [[ $command == winetricks ]]; then
    command -v winetricks >/dev/null 2>&1 || die "winetricks is not installed (sudo pacman -S winetricks)"
    args=("${pass[@]}")
else
    args=("$command")
    for a in "${pass[@]}"; do
        if [[ $a == /* && -e $a ]]; then args+=("$(dos "$a")"); else args+=("$a"); fi
    done
fi

# How it runs.
find_proton() {
    local d found=()
    for d in /usr/share/steam/compatibilitytools.d/proton-cachyos* \
        "$HOME"/.steam/root/compatibilitytools.d/GE-Proton* \
        "$HOME"/.local/share/Steam/compatibilitytools.d/GE-Proton* \
        "$HOME"/.local/share/Steam/compatibilitytools.d/proton-cachyos*; do
        [[ -x $d/proton ]] && found+=("$d")
    done
    ((${#found[@]})) && printf '%s\n' "${found[@]}" | sort -V | tail -n 1
    return 0
}
prefix=${prefix:-${WINEPREFIX:-${STEAM_COMPAT_DATA_PATH:-${XDG_DATA_HOME:-$HOME/.local/share}/victory/proton}}}
proton=${PROTON:-}
runner=()
how=""
if ((${#steam[@]})); then
    [[ $command != winetricks ]] || die "from Steam, use protontricks (protontricks -l lists the shortcut's id)"
    how="Steam's command"
    runner=("${steam[@]}")
elif command -v umu-run >/dev/null 2>&1; then
    if [[ -z ${PROTONPATH:-} ]]; then
        p=${proton:-$(find_proton)}
        [[ -n $p ]] && export PROTONPATH=$p
    fi
    export GAMEID=${GAMEID:-umu-victory} WINEPREFIX=$prefix
    how="umu-run (PROTONPATH=${PROTONPATH:-the default of umu})"
    runner=(umu-run "$victory")
    [[ $command == winetricks ]] && runner=(umu-run winetricks)
else
    [[ -n $proton ]] || proton=$(find_proton)
    if [[ -n $proton ]]; then
        [[ -x $proton/proton ]] || die "PROTON=$proton has no proton script"
        export STEAM_COMPAT_DATA_PATH=$prefix
        if [[ -z ${STEAM_COMPAT_CLIENT_INSTALL_PATH:-} ]]; then
            for d in "$HOME/.local/share/Steam" "$HOME/.steam/steam" "$HOME/.steam/root"; do
                [[ -d $d ]] && STEAM_COMPAT_CLIENT_INSTALL_PATH=$d && break
            done
            export STEAM_COMPAT_CLIENT_INSTALL_PATH=${STEAM_COMPAT_CLIENT_INSTALL_PATH:-$HOME/.local/share/Steam}
        fi
        how="Proton $proton"
        runner=("$proton/proton" run "$victory")
        if [[ $command == winetricks ]]; then
            [[ -d $prefix/pfx ]] || die "no prefix yet in $prefix: start the game (or hlsl-check) once first"
            runner=(env "WINE=$proton/files/bin/wine" "WINESERVER=$proton/files/bin/wineserver" "WINEPREFIX=$prefix/pfx" winetricks)
        fi
    elif command -v wine >/dev/null 2>&1; then
        say "No umu-run and no Proton (set PROTON=/path/to/proton-dir): plain wine. Direct3D 12 then needs"
        say "vkd3d-proton, DXGI needs DXVK and NVAPI needs dxvk-nvapi, installed into WINEPREFIX=$prefix."
        export WINEPREFIX=$prefix
        how="wine (WINEPREFIX=$prefix)"
        runner=(wine "$victory")
        [[ $command == winetricks ]] && runner=(winetricks)
    else
        die "nothing to run victory.exe with: install umu-launcher, set PROTON, or install wine"
    fi
fi
# The prefix holds the caches and saves (the game's data): never inside the repository.
if ((${#steam[@]} == 0)) && inside_repo "$prefix"; then
    die "the prefix $prefix is inside the repository; use one outside it (--prefix)"
fi

# The game's own log: victory.exe writes everything it prints there (VICTORY_OUTPUT_FILE), since under Proton
# its console output reaches no terminal; this script shows the file as it grows. Proton's and umu's own lines
# go to a log of their own.
log=$logs/victory-nvidia-$stamp.log
proton_log=$logs/proton-$stamp.log
[[ $command == winetricks ]] || export VICTORY_OUTPUT_FILE
VICTORY_OUTPUT_FILE=$(dos "$log")
switches=$(for k in "${!s[@]}"; do printf '%s=%s\n' "$k" "${s[$k]}"; done | sort | tr '\n' ' ')
header="# console=$console build=$build preload=$preload smooth_motion=$smooth runner: $how switches: $switches"
if ((dry_run)); then
    printf '%s\n' "$header"
    printf 'cd %q\n' "$repo"
    printf 'run:'
    printf ' %q' "${runner[@]}" "${args[@]}"
    printf '\n'
    env | grep -E '^(PROTON|DXVK|VKD3D|NVPRESENT|VK_LOADER|STEAM_COMPAT|WINEPREFIX|GAMEID|PROTONPATH|__GL)' | sort
    exit 0
fi
((${#steam[@]})) || mkdir -p "$prefix"
mkdir -p "$logs"
printf '%s\n' "$header" >"$log"
say "Log: $log"
say "Proton's own output: $proton_log"

cd "$repo" # victory.toml is read from here
run_victory() {
    local shown=""
    if [[ $command != winetricks ]]; then
        tail -n 0 -F "$log" 2>/dev/null &
        shown=$!
    fi
    set +e
    "${runner[@]}" "$@" 2>&1 | tee -a "$proton_log"
    local rc=${PIPESTATUS[0]}
    set -e
    if [[ -n $shown ]]; then
        sleep 1 # the last lines
        kill "$shown" 2>/dev/null || true
        wait "$shown" 2>/dev/null || true
    fi
    return "$rc"
}
if [[ $command != run ]]; then
    run_victory "${args[@]}"
    exit $?
fi
while :; do
    if ((preload)); then
        say "Preloading shaders before the game starts. This takes a while."
        run_victory preload "$(dos "$(dirname "$exe")")" || say "The preload stopped; what it did is kept."
        preload=0
    fi
    rc=0
    run_victory "${args[@]}" || rc=$?
    if ((rc == restart_code || rc == restart_code + 1)); then
        ((rc == restart_code + 1)) && preload=1
        printf '# restarted by the F1 menu (exit code %s)\n' "$rc" >>"$log"
        say "Starting the game again (F1 menu) ..."
        continue
    fi
    exit "$rc"
done
