# Bazarr

Subtitle automation. Sources subtitles for Sonarr and Radarr libraries.

- **Port**: 6767
- **Image**: `lscr.io/linuxserver/bazarr`
- **Compose**: [`compose.yml`](../compose.yml)
- **Network**: `media` (external)

## Volumes

| Container Path | Description |
|----------------|-------------|
| `/config` | Bazarr config (`${BAZARR_CONFIG_PATH}` on the host) |
| `/data/media` | Media library, matching Sonarr/Radarr (`${MEDIA_PATH}` on the host) |

Mount the media library at the same in-container path Sonarr and Radarr use so
Bazarr can resolve the paths they report.

## Connected Apps

| App | Default Port | Role |
|-----|--------------|------|
| Sonarr | 8989 | TV library source |
| Radarr | 7878 | Movie library source |
| Whisper ASR (optional) | 10300 | AI transcription fallback |

Point Bazarr at your Sonarr/Radarr instances after first start. An optional
[Whisper ASR](https://github.com/ahmetoner/whisper-asr-webservice) service can be
configured as a last-resort provider that transcribes audio when no external
provider has a matching subtitle.

## Subtitle Automation Policy

A typical end state: **every video has a subtitle in the target language**, each
subtitle is synced to the audio track, and the library carries no unwanted
foreign-language sidecars.

### Providers

Configured in priority order (Bazarr falls back down the list). A common setup:

1. A human-transcript provider such as **opensubtitles.com** — preferred, scored
   against the release name.
2. **whisperai** — last-resort AI transcription. Generates a subtitle when no
   provider hit clears the configured `minimum_score`. Runs as a separate ASR
   service, optionally GPU-accelerated.

> **GPU sharing**: if the Whisper ASR service shares a GPU with other workloads
> (e.g. a local LLM runtime), account for the resident VRAM of the Whisper model
> when sizing the co-tenant. Use a smaller Whisper model, or stop the ASR service
> while the other workload runs, if VRAM is tight.

Whisper-generated subs are **not pinned**. `upgrade_subs=true` re-checks providers
periodically; any later fetch that beats the current sub's score replaces it
automatically.

### Scheduled tasks (Bazarr built-in)

| Task | Cadence | Purpose |
|------|---------|---------|
| `update_series` / `update_movies` | 60 min | Sync library from Sonarr/Radarr |
| `wanted_search_missing_subtitles_series` | 6 h | Search subs for episodes that have none (Whisper fills gaps) |
| `wanted_search_missing_subtitles_movies` | 6 h | Same, for movies |
| `series_full_scan_subtitles` / `movies_full_scan_subtitles` | daily | Reindex on-disk sub files |
| `upgrade_subtitles` | 12 h | Re-score existing subs; replace if a better one is available. **Also re-runs ffsubsync** when the sub is out of sync threshold. `days_to_upgrade_subs=7`. |

### Sync behavior (ffsubsync)

Settings live under the `subsync.*` section of `system/settings` (NOT flat `general.*`):

- `subsync.use_subsync=true`, `subsync.force_audio=true`, `subsync.gss=true` (golden-section search — handles progressive drift, not just constant offset)
- `subsync.max_offset_seconds=60`
- `subsync.no_fix_framerate=false` — i.e. framerate correction is **on** (double negative), which is what fixes NTSC↔PAL drift
- `subsync.use_subsync_threshold=false` / `subsync.use_subsync_movie_threshold=false` — sync **every** sub on upgrade pass, not just below-threshold ones
- Audio track is the reference (no transcript dependency)

> Note: the automatic `upgrade_subtitles` task only re-syncs subs within the `days_to_upgrade_subs` window (7 days). **Old subs never get re-synced automatically** — resync them explicitly via the per-subtitle PATCH below (see "Per-subtitle sync"). The PATCH enqueues into Bazarr's internal sync worker queue (concurrency 4) and returns 204 immediately; the actual ffsubsync work drains in the background.

#### Diagnosing out-of-sync subs

Two distinct failure modes, different fixes:

- **Constant offset** — sub is early/late by the same amount for the whole
  episode. ffsubsync fixes this cleanly as long as the shift is within
  `max_offset_seconds` (60s).
- **Progressive drift** — fine at the start, worse toward the end. Almost
  always a **framerate mismatch** (23.976 fps film vs 25 fps PAL). Common on
  older / DVD-era and animated shows that were rebroadcast at a different
  cadence. Audio-referenced ffsubsync handles this too (it stretches, not
  just shifts) — *provided the sub was timed for the same edit of the episode*.

When a resync **won't take**, the sub is almost always timed for a **different
release** than the on-disk video (e.g. a BluRay-timed sub over a WEB-DL rip, or
a differently-edited broadcast master). The true offset then exceeds
`max_offset_seconds=60` and ffsubsync bails. Fix: **blacklist** the bad sub so
Bazarr won't re-grab it (wrench → blacklist in the UI, or the blacklist API),
then re-search — a release-matched sub syncs correctly. Bumping
`max_offset_seconds` past ~60s is not a fix; it just lets a wrong sub snap to a
false alignment.

#### Per-subtitle sync

The bulk `upgrade_subtitles` task re-syncs everything on a schedule, but a
single-pane-of-glass tool also needs **on-demand sync of one subtitle** — the
UI's wrench → **Sync** action. That is *not* a `system/tasks` trigger; it's:

```bash
# Sync one existing subtitle against the video's audio track.
# id = sonarrEpisodeId (episodes) or radarrMovieId (movies)
curl -s -X PATCH -H "X-API-KEY: $K" \
  --data-urlencode 'action=sync' \
  --data-urlencode 'language=en' \
  --data-urlencode 'path=/data/media/tv/.../episode.en.srt' \
  --data-urlencode 'type=episode' \
  --data-urlencode 'id=<episodeId>' \
  "$B/subtitles"
```

The orca `bazarr.*` sync verb should wrap this per-subtitle PATCH (targeted
resync) in addition to the bulk task trigger (fleet-wide sweep).

### Sub-file slimming

A common local convention when keeping the library to a single language:

**Keep**, per video:
- `<base>.en.<ext>` — primary English
- `<base>.en.forced.<ext>` — forced (foreign-dialog captions)
- `<base>.en.hi.<ext>` / `<base>.hi.<ext>` — English HI/SDH
- `<base>.<ext>` — unlabeled (assumed primary English)

**Delete**:
- Any sidecar in another language (fr/es/de/zh/…)
- Duplicate English variants in the same slot (keep largest)
- `*.bazarr.*` scratch files

Embedded subtitle tracks inside the container are **never touched** — leaving
baked-in foreign subs in place is fine.

Subtitle extensions handled: `.srt .ass .ssa .sub .idx .vtt .sup`

### Operational runbook

```bash
# Bazarr API key (also in the container at /config/config/config.yaml)
K=$(docker exec bazarr grep "^  apikey:" /config/config/config.yaml | head -1 | awk '{print $2}')
B=http://localhost:6767/api

# Trigger Bazarr scheduled jobs ad-hoc
for jid in wanted_search_missing_subtitles_series \
          wanted_search_missing_subtitles_movies \
          upgrade_subtitles; do
  curl -s -X POST -H "X-API-KEY: $K" "$B/system/tasks?taskid=$jid"
done

# Update a Bazarr setting via API (form-encoded, lowercase bools)
curl -s -X POST -H "X-API-KEY: $K" \
  --data-urlencode 'settings-general-upgrade_subs=true' \
  "$B/system/settings"
```

### Future: orca arr-management plugin

Bazarr stays as the execution layer. A future orca *arr management plugin
(bazarr/sonarr/radarr/prowlarr/lidarr) will expose this same policy surface —
providers, schedules, bulk ops, **per-subtitle resync** (see "Per-subtitle sync"
above), sub-file hygiene — through orca's single pane of glass, so it's driven
from CLI/MCP/REST/UI instead of the per-arr web UIs.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TZ` | `Etc/UTC` | Timezone |
| `PUID` / `PGID` | `99` / `100` | User/group ID |
| `BAZARR_IMAGE_TAG` | `latest` | Image tag |
| `BAZARR_CONFIG_PATH` | `/opt/appdata/bazarr` | Config directory |
| `MEDIA_PATH` | `/mnt/pool/data/media` | Media library |

## Troubleshooting

```bash
docker logs bazarr
# Config file lives at ${BAZARR_CONFIG_PATH}/config/config.yaml
```
