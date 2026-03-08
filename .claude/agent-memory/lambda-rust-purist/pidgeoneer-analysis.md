# Pidgeoneer Dashboard Analysis (2026-03-08)

## Architecture
- Leptos 0.7 SSR+hydrate app with Axum server
- Data pipeline: PID example -> Iggy.rs -> Axum server (consumer) -> broadcast channel -> WebSocket -> Browser
- Browser shows numeric cards only; Chart.js loaded but unused

## Bugs Found
1. **Timestamp type mismatch**: `ControllerDebugData.timestamp` is `u128`, `PidControllerData.timestamp` is `u64`
2. **O(n) insertion**: `data_vec.insert(0, data)` in iggy_client.rs line 55 - should use VecDeque::push_back
3. **Memory leaks**: Four `.forget()` calls on wasm_bindgen Closures in iggy_client.rs
4. **Reconnect = page reload**: Line 114 iggy_client.rs does `window.location().reload()`, losing all data

## Naming Issues
- `IggyClient` (iggy_client.rs) is actually a WebSocket client to the Axum server, not to Iggy
- The module name `iggy_client.rs` is equally misleading for the browser side

## Missing Features (priority order)
1. **Setpoint & process variable** not in data model (only error = setpoint - PV)
2. **Time-series charts** (Chart.js loaded but zero integration)
3. **Controller config display** (Kp, Ki, Kd, limits)
4. **Live statistics** (overshoot, settling time, etc.)
5. **Iggy health surfaced to UI** (dashboard says "Connected" even when Iggy is down)

## Cleanup Opportunities
- Redundant `#[cfg(feature = "ssr")]` in main.rs line 13 (already inside ssr-gated fn)
- `WebSocketState` struct is unnecessary indirection over `broadcast::Sender`
- `_ => {}` catch-all in websocket.rs line 58 should be `if let`
