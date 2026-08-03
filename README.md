<p align="center">
  <img src="assets/icon-256.png" width="120" alt="bazarr" />
</p>

# bazarr

[Bazarr](https://www.bazarr.media/) manages and sources subtitles for the
movies and TV series handled by Radarr and Sonarr.

First-party [orca](https://github.com/argyle-labs/orca) plugin (service-backend).

> **Status:** deploy config + docs only. Bazarr is not Servarr-shaped (it has
> its own API, not the shared `X-Api-Key` client), so it lives in its own repo
> rather than the [`arr`](https://github.com/argyle-labs/arr) workspace. The
> orca service-backend implementation is planned; today this repo carries the
> self-contained deploy config so it can be stood up by hand or via orca.

| App | Port | Role |
|-----|------|------|
| **Bazarr** | `6767` | Subtitles for Radarr/Sonarr libraries |

## Deploy

```bash
docker compose up -d
```

See [`compose.yml`](compose.yml) for volumes and environment (upstream
LinuxServer image, `${MEDIA_PATH}` + config paths parameterized). Point Bazarr
at the same media library and at your Radarr/Sonarr instances after first start.

Operator notes: [`docs/bazarr.md`](docs/bazarr.md).
