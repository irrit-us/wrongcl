# wrongcl Single-Screen Reorg + Core Expansion Plan

Last reviewed: 2026-06-21.

## Goal

Reorganize the main screen into a compact 3-section single-view layout
(no scrolling on the primary surface) and grow the Rust core so the UI
controls describe real, working features rather than truthful placeholders.

Pre-production status: the existing single-endpoint config and any persisted
profiles can be rewritten directly. No backward-compatibility shims.

## Truthful-controls rule (unchanged)

- Unsupported controls never look enabled.
- Disabled controls always show a reason.
- No UI feature implies backend support that does not exist.
- Until a phase ships its backend, the affected controls remain visibly
  disabled with an explicit reason.

## Concept model

- `Profile` — a server connection (endpoint, transport, outer security, auth).
  Already exists in `ProfileStore`.
- `Script` — a routing rule list (new). Clash-subset rules:
  `DOMAIN`, `DOMAIN-SUFFIX`, `DOMAIN-KEYWORD`, `IP-CIDR`, `GEOIP`, `MATCH`,
  each mapping to `DIRECT | PROXY(<group>) | REJECT`. No JavaScript engine.
- `Mode` — a named `(profile, script)` pair (new). Built-in modes:
  - `Global` = (active profile, `MATCH -> PROXY(default-group)`)
  - `Rule` = (active profile, user's default script)
  - `Direct` = (no profile, `MATCH -> DIRECT`)
    Slots 4-6: user-defined modes created via the Add Mode dialog.

Persistence: `ProfileStore` JSON gains `scripts[]`, `modes[]`, `endpoints[]`,
`groups[]`, `active_mode`. Schema is rewritten on first launch.

## Single-screen layout (adaptive, 1024x640 floor)

```
+---------------------------------------------------------------+
| [icon] | Global | Rule | Direct | +Add  |  ...   |  ...      |  Row 1: icon + 6 mode slots,
+--------+----------------+------------+--------------+--------+   modes share dark panel
|                |                  |                          |
|  Up chart      |   [SysProxy o-]  |   Up   total / s         |  Row 2: charts | controls |
|  Down chart    |   [   > Runtime] |   Down total / s         |   stats (3 equal columns;
|                |   [TUN     o-]   |                          |   center column has dark
|                |                  |                          |   bottom border + thinner
|                |                  |                          |   border under Runtime)
+----------------+------------------+--------------------------+
|  [ Proxies                ]  [ Profiles                    ] |  Row 3a
|  [ Connections ]  [ Requests ]  [ Logs                     ] |  Row 3b
|  [ Basic ] [ Network ] [ DNS ] [ Advanced                  ] |  Row 3c
+---------------------------------------------------------------+
```

- Top row distributes 7 cells evenly across full width; each cell is
  center-aligned within its slot. Once all 6 mode slots are occupied, the
  Add Mode button is removed.
- Middle row: charts (left), control column (center, dark bordered),
  stats (right), all equal width.
- Bottom row: 3 sub-rows of equal-height entry chips.
- All subpages are full-screen overlays with `< close | title` header.

## Rust core additions (`wrongcl/rust/src/`)

1. **Connection registry** — `proxy.rs` replaces aggregate `AtomicU64`
   counters with an `Arc<DashMap<u64, ConnInfo>>`:
   `{id, started_at, peer_addr, target_host_port, source_pid?, source_app?,
   url?, bytes_up, bytes_down, state}`.
   Source app lookup:
   - Linux: `/proc/net/tcp` + `/proc/<pid>/comm`
   - macOS: `proc_pidinfo` / `audit_token`
   - Windows: `GetExtendedTcpTable`
2. **HTTP CONNECT/Host capture** — `proxy/request.rs` records the request
   line and `Host` header for plain HTTP; CONNECT and SOCKS record
   `host:port` only. No HTTPS interception.
3. **Tracing log capture** — new `logs.rs` installs a
   `tracing_subscriber::Layer` writing to a bounded ring buffer
   (2000 entries). Entry shape: `{ts, level, target, message, fields}`.
   Add `tracing` instrumentation to `accept_loop`, `handle_socks_client`,
   `client.rs::connect`, router decisions, DNS lookups, TUN packets.
4. **Rule engine** — new `router.rs`. Pure-Rust matcher over the rule
   subset above; embedded GEOIP country lookup. Decision type:
   `Direct | Proxy(endpoint_id) | Reject`. Default action when no rule
   matches is configurable per script.
5. **Multi-endpoint + proxy groups** — `config.rs` rewrites to:

   ```
   ClientConfig {
     endpoints: Map<String, Endpoint>,
     groups: Vec<ProxyGroup>,                 // select | fallback | url-test
     scripts: Vec<Script>,
     modes: Vec<Mode>,
     active_mode: String,
     local: LocalProxyConfig,
     dns: DnsSettings,
   }
   ```

   The router resolves rule decisions through `groups` to a concrete
   endpoint at connect time.
6. **DNS resolver** — new `dns.rs`. Backends: system, `udp://1.1.1.1:53`,
   `https://...` (DoH). Per-rule resolution happens before matching when
   the rule needs an IP.
7. **TUN driver** — new `tun.rs` plus per-OS modules. Linux/macOS via the
   `tun` crate, Windows via `wintun`. Packet plane via `smoltcp`. Routes
   each connection through the router. Privilege model is surfaced as a
   `needs privileges` disabled reason; no silent `sudo`.
8. **FFI surface additions**:
   - `wrongcl_connections_list_json`
   - `wrongcl_connection_close(id)`
   - `wrongcl_connections_close_matching(filter_json)`
   - `wrongcl_logs_since(cursor) -> {entries, cursor}`
   - `wrongcl_router_active_mode_json` / `_set(name)`
   - `wrongcl_router_set_script_json(json)`
   - `wrongcl_proxy_groups_json`
   - `wrongcl_proxy_group_select(group, member)`
   - `wrongcl_dns_settings_json` / `_set(json)`
   - `wrongcl_tun_status_json`
   - `wrongcl_tun_enable_json(config)` / `wrongcl_tun_disable`

## Flutter additions (`wrongcl/lib/`)

- `main_view.dart` — new single-screen `Column` with the 3 sections.
- `widgets/mode_strip.dart` — top row icon + mode chips + add-mode button.
- `widgets/control_column.dart` — pill / play / pill stack with the
  dark-border + thinner-border-under-Runtime treatment.
- `widgets/traffic_chart.dart` — `CustomPainter` line graph driven by the
  existing in-controller `uploadedBytesHistory` / `downloadedBytesHistory`
  derived to bytes-per-second.
- `widgets/traffic_stats.dart` — cumulative byte totals (right column).
- `widgets/subpage_scaffold.dart` — close + title header used by every
  subpage overlay.
- `subviews/`:
  - `proxies_view.dart` — group list at top, members below, selection
    controls and group-kind (select/fallback/url-test) actions.
  - `connections_view.dart` — search/filter by URL or source app, per-row
    close, close-all-matching.
  - `requests_view.dart` — list of captured CONNECT/Host targets with
    `source_app` and either URL (HTTP) or `ip:port` (everything else).
  - `logs_view.dart` — colored rows by level, expand affordance for
    entries longer than two rendered lines.
  - `mode_picker_view.dart` — add-mode dialog: name + profile + script.
  - `settings/basic_view.dart`
  - `settings/network_view.dart`
  - `settings/dns_view.dart`
  - `settings/advanced_view.dart`
- `client_home_controller.dart` stays the orchestrator. Feature-specific
  state moves to `lib/controllers/{router,connections,logs,proxies,tun,
  dns}_controller.dart` to keep `client_home_controller.dart` from
  ballooning.
- Old `dashboard_view.dart` and `home_widgets.dart` are removed once
  Phase 1 replaces them.

## Phasing

Each phase ships independently. Unsupported features remain visible-but-
disabled with explicit reasons until their phase ships.

| Phase | Scope                                                                                                                                                                                                                                    | Backend?  | Visible result                                    |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------- |
| 1     | New single-screen shell. Middle row real-time charts (client-side polling of existing `wrongcl_proxy_status`). System Proxy and Runtime toggles wired. TUN toggle visible but disabled. Bottom-row entries open empty subpage scaffolds. | no        | "Fits in one screen" deliverable.                 |
| 2     | Connection registry + tracing log capture + their FFI.                                                                                                                                                                                   | yes       | Connections and Logs pages working.               |
| 3     | HTTP CONNECT/Host capture + source-app lookup per OS.                                                                                                                                                                                    | yes       | Requests page working.                            |
| 4     | Multi-endpoint + proxy groups (config rewrite + group FFI).                                                                                                                                                                              | yes       | Proxies page working: groups, members, selection. |
| 5     | Rule engine + script/mode storage + Add Mode dialog wired.                                                                                                                                                                               | yes       | Global / Rule / Direct actually route traffic.    |
| 6     | DNS resolver + DNS settings page.                                                                                                                                                                                                        | yes       | DNS page working.                                 |
| 7     | TUN driver: Linux first, then Windows (wintun), then macOS.                                                                                                                                                                              | yes       | TUN toggle real.                                  |
| 8     | Settings content split into Basic/Network/Advanced (Network includes proxy listen host/port, mixed protocol toggles; Basic covers autostart, language, theme; Advanced covers diagnostics, log level, raw config editor).                | mostly UI | Settings parity grows incrementally.              |
| 9     | Release gate: `flutter analyze`, `flutter test`, Windows + Linux builds, Rust `cargo test`, desktop platform smoke.                                                                                                                      | -         | Ship.                                             |

## Implementation decisions taken without further asking

- Rule engine: Clash-subset matcher; GEOIP via a small embedded country
  database.
- Logs ring buffer: 2000 entries.
- Mode / script / endpoint / group storage: extends `ProfileStore` JSON
  with additive top-level keys; pre-production schema rewrite is allowed.
- TUN privileges: surface a `needs privileges` disabled reason and a
  one-screen setup explainer per OS. Never silent `sudo`.
- Test coverage: at least one widget test per new subpage; Rust unit
  tests for the connection registry, router, DNS resolver, and rule
  matcher.
- Build hygiene: no schema-migration code, no compat shims, no
  feature-flagged "old vs new" code paths.

## File targets

- replace: `wrongcl/lib/dashboard_view.dart`, `wrongcl/lib/home_widgets.dart`
- add: `wrongcl/lib/main_view.dart`, `wrongcl/lib/widgets/...`,
  `wrongcl/lib/controllers/...`, `wrongcl/lib/subviews/{proxies,
  connections,requests,logs,mode_picker}_view.dart`,
  `wrongcl/lib/subviews/settings/{basic,network,dns,advanced}_view.dart`
- extend: `wrongcl/lib/client_home_controller.dart`,
  `wrongcl/lib/control_state.dart`, `wrongcl/lib/profile_store.dart`,
  `wrongcl/lib/wrongcl_client.dart`
- add: `wrongcl/rust/src/{logs,router,dns,tun}.rs`,
  `wrongcl/rust/src/tun/{linux,macos,windows}.rs`
- rewrite: `wrongcl/rust/src/{config,proxy,proxy/request,ffi}.rs`

## Release gate

Before this work is called complete:

- `flutter analyze`
- `flutter test`
- `cargo test --manifest-path wrongcl/rust/Cargo.toml`
- `flutter build windows`
- existing `scripts/verify-local.sh linux` on Linux or CI
- existing `scripts/verify-macos-host.sh` on macOS or CI
- Manual desktop check:
  - the main screen fits at 1024x640 without scrolling
  - top row distributes 7 cells evenly; Add Mode disappears at 6 modes
  - middle row charts update in real time when traffic flows
  - System Proxy and Runtime toggles act on real backends
  - TUN toggle either flips a real driver or shows its disabled reason
  - Proxies, Profiles, Connections, Requests, Logs, and the four
    Settings subpages open as full-screen overlays with close+title

## Non-goals

- No HTTPS interception (no MITM CA, no decrypted URL paths).
- No JavaScript rule script engine (Clash-subset rules only).
- No mobile (Android/iOS) parity in this milestone.
- No Clash/Mihomo YAML import in this milestone.
