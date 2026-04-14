---
name: ffmpeg-debugging
description: 'Debug FFmpeg-related issues in the Toaster decoder and exporter plugins. Use for: ffmpeg crash, decode error, av_read_frame, packet interleaving, AVFrame corruption, audio video sync, sws swr context, avformat avcodec, seek issue, timestamp mapping, pts dts, AV_TIME_BASE, export failure, encoder muxer.'
---

> **⚠️ FORWARD-LOOKING**: FFmpeg integration has **not been implemented yet**. No `plugins/` directory or FFmpeg code exists in the current codebase. This skill documents the *intended* patterns for when FFmpeg decoder/exporter plugins are built (see PRD.md Phase 3–4). Do not reference these files as if they exist today.

# FFmpeg Debugging

Diagnose and fix FFmpeg issues in Toaster's decoder and exporter plugins.

## When to Use
- Video/audio decode failures or corruption
- A/V sync problems during playback
- Export produces broken or truncated files
- Seek jumps to wrong position
- Crashes in FFmpeg cleanup

## Key Files
- `plugins/ffmpeg-decoder/ffmpeg-decoder.c` — Decoding pipeline
- `plugins/ffmpeg-exporter/ffmpeg-exporter.c` — Export/muxing pipeline

## Common Pitfalls and Fixes

### 1. AVFrame Sharing (Decode Corruption)
**Symptom**: Video shows audio artifacts or garbled frames.
**Cause**: Single `dec->frame` shared between audio and video decode.
**Fix**: Use separate `dec->video_frame` and `dec->audio_frame`:
```c
dec->video_frame = av_frame_alloc();
dec->audio_frame = av_frame_alloc();
```

### 2. Packet Interleaving (Dropped Frames)
**Symptom**: Audio stutters, video skips frames, or one stream is missing.
**Cause**: `av_read_frame()` returns packets for any stream — discarding non-matching packets loses data.
**Fix**: Queue packets per stream index. Process the target stream's packet, queue others for later.

### 3. Cleanup Order (Crash on Exit)
**Symptom**: Segfault or access violation during shutdown.
**Cause**: Freeing contexts in wrong order.
**Fix**: Always free in this order:
1. `sws_freeContext()` / `swr_free()`
2. `avcodec_free_context()` (video + audio)
3. `avformat_close_input()`

### 4. Seek Errors
**Symptom**: Seek lands on wrong frame or produces glitch frames.
**Cause**: FFmpeg seeks to nearest keyframe; need to flush + decode forward.
**Fix**: After `av_seek_frame()`, call `avcodec_flush_buffers()` on all codec contexts, then decode forward to target PTS.

### 5. Timestamp Mapping with Deletions
**Symptom**: Playback position doesn't match transcript after deleting words.
**Cause**: Deleted segments reduce effective duration.
**Fix**: Sum deleted segment durations before the current position to map edit-time → source-time. All timestamps are in microseconds (`AV_TIME_BASE`).

## Debugging Procedure
1. Identify whether the issue is in decode, export, or both
2. Check stream index handling in packet processing loops
3. Verify AVFrame allocation (separate per stream)
4. Check cleanup order in destroy function
5. For timestamp issues, trace the edit-time → source-time mapping
