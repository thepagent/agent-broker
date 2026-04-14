# OpenAB 0.7.3 Copilot CLI Backend Test Report

**Tester**: @xisx7876 (資管小神)
**Date**: 2026-04-14
**Versions**: OpenAB 0.7.3 / Copilot CLI 1.0.25
**Bot**: COPILX (CopilotNative backend, `copilot --acp`)
**OS**: Windows 11 Pro, 16GB RAM

---

## Test Configuration

```toml
[pool]
max_sessions = 3
session_ttl_hours = 24
```

- LRU eviction: disabled (hard reject on pool full)
- Warmup session: auto-released after model cache populated

---

## Test Steps & Results

| Step | Action | Expected | Result |
|------|--------|----------|--------|
| 1 | Open thread 1: `@COPILX 我的代號是威瑟001` | Session created | ✅ PASS — "收到，威瑟001。" |
| 2 | Open thread 2: `@COPILX 我的代號是喬斯00A` | Session created | ✅ PASS — "收到，喬斯00A。" |
| 3 | Open thread 3: `@COPILX 我的代號是哈奇峰HAHa` | Session created, pool full (3/3) | ✅ PASS — "收到，哈奇峰HAHa。" |
| 4 | Open thread 4: `@COPILX 我的代號是八股` | **No new process**, Service Busy | ✅ PASS — "⚠️ Service Busy: All agent sessions are in use" |
| 5 | Return to thread 1: `你的代號是什麼` | Remembers context | ✅ PASS — "你的代號是威瑟001。" |

**All 4 test conditions met.**

---

## Resource Usage Per Session

| Metric | Baseline (0 sessions) | 1 session | 2 sessions | 3 sessions |
|--------|-----------------------|-----------|------------|------------|
| copilot.exe processes | 0 | 2 | 4 | 6 |
| RAM (copilot only) | 0 MB | ~283 MB | ~565 MB | ~847 MB |

Each session spawns:
- 1x copilot.exe agent (~53 MB)
- 1x copilot.exe worker (~237 MB)
- **Total per session: ~290 MB**

---

## Issues Found

### 1. Warmup Session Occupies Pool Slot (Fixed)

**Severity**: Medium
**Status**: Fixed locally

OpenAB spawns a warmup session (`__warmup__`) on startup to populate the model cache for `/model` autocomplete. This session counted against `max_sessions`, so `max_sessions=3` only allowed 2 user sessions.

**Fix**: Call `pool.drop_session("__warmup__")` after warmup completes.

```
[warmup] model cache populated count=8
[warmup] released warmup session
```

### 2. LRU Eviction Behavior (By Design)

**Severity**: Info
**Status**: Working as intended

When pool is full, OpenAB 0.7.3 doesn't reject — it **suspends the oldest idle session** (LRU eviction) and creates a new one. This is implemented in commit `a021a42` (PR #309: session suspend/resume via `session/load`).

If Copilot CLI supports `session/load`, suspended sessions can be restored with context preserved.

For testing purposes, LRU eviction was disabled to verify hard rejection behavior.

### 3. Intermittent Crash at 3 Concurrent Sessions

**Severity**: High
**Status**: Under investigation

In 2 out of 4 test rounds, openab.exe silently exited 10-75 seconds after the 3rd session was created. The bat loop auto-restarted, but all session context was lost.

**Observations**:
- No panic or error in openab.log (last line is normal INFO)
- No Application Error in Windows Event Log
- No OOM event in System Event Log
- Occurred at ~1 GB total copilot RAM (6 copilot.exe processes)

**Likely cause**: A Copilot CLI child process exits unexpectedly under memory pressure, breaking the stdio pipe, which causes openab to exit.

**Reproduction rate**: ~50% (2/4 rounds)

---

## Summary

| Test Item | Status |
|-----------|--------|
| Session creation (1-3) | ✅ PASS |
| Pool limit enforcement (4th session rejected) | ✅ PASS |
| Context preservation (return to session 1) | ✅ PASS |
| Resource isolation (no extra processes on rejection) | ✅ PASS |
| Stability at 3 concurrent sessions | ⚠️ Intermittent crash |
| Per-session resource cost | ~290 MB / 2 processes |

---

## Recommendations

1. **Default `max_sessions` for Copilot backend**: Consider `max_sessions = 2` for stability, or keep 3 with LRU eviction enabled
2. **Warmup slot fix**: Should be merged upstream — warmup session should not count against pool limit
3. **Crash investigation**: Needs Copilot CLI team input — may be related to multiple `copilot --acp` instances competing for resources
