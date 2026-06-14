# Onboard seeker-camera clips

Drop your real onboard / FPV-style footage here. The dashboard plays one of
these in the **SEEKER FEED** picture-in-picture window when you click an
in-flight interceptor on the map.

## Expected files

```
seeker-1.mp4
seeker-2.mp4
seeker-3.mp4
seeker-4.mp4
```

Each in-flight interceptor is mapped to one clip deterministically (by its id),
so different interceptors show different feeds. To add more variety, drop more
files and extend the `CLIPS` array in `webui/src/SeekerFeed.tsx`.

## Recommended format

- **Container/codec:** `.mp4` (H.264) — best browser support.
- **Aspect ratio:** 16:9 (the window is `aspect-video`); other ratios are cropped (`object-cover`).
- **Length:** a few seconds is fine — clips loop while the interceptor is in flight.
- **No audio needed** — playback is muted.
- Keep files small (a few MB) so they preload quickly.

If a file is missing or fails to load, the feed gracefully shows procedural
"NO SIGNAL" static instead — so the UI never breaks, even before you add clips.
