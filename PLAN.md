# wrongcl Remaining Work

Last reviewed: 2026-06-15. Source repos: `wrongsv/` (server + 15 crates), `wrongcl/` (Rust core + Flutter UI + FFI).

## Standing constraints (apply to everything below)

- **Privacy:** never commit infra IPs. Reference the tencentde remote by SSH alias only. Local-only configs (`config.local.toml`, etc.) must stay out of git.
- **GFW-only failures are acceptable.** A profile that passes locally but shows GFW characteristics through tencentde is not a defect — log and move on.
- **VMess is a custom dialect.** wrongsv's VMess KDF diverges from v2fly/xray. Internal `wrongcl ↔ wrongsv-server` interop is the only goal; do NOT claim third-party compatibility.
- **VLESS Kyber-via-addons is dead.** Already removed on 2026-06-14 — do not reintroduce the addons field or related code paths.
- **Borrow, do not re-derive.** All client transport code already exists in `wrongsv/crates/`. Add the relative path dep, import the public API, wrap behind the `Tunnel` trait. Re-deriving frame formats has historically caused divergence bugs (VMess KDF, WebSocket framing).

## Current state (implemented + tested)

Adapter recognizes 22 wrongsv profiles (`rust/src/adapter.rs:13-36`); 11 are wired end-to-end:

| Profile      | Proxy        | Transport   | Outer security | Borrow source                                 |
| ---          | ---          | ---         | ---            | ---                                           |
| `raw`        | VLESS        | raw TCP     | none           | `wrongsv-vless`, `wrongsv-vless-encoding`     |
| `tls`        | VLESS        | raw TCP     | TLS            | + rustls                                      |
| `reality`    | VLESS        | raw TCP     | REALITY        | `wrongsv-reality` auth primitives + inline RFC 8446 TLS 1.3 record layer |
| `anytls`     | VLESS        | raw TCP     | AnyTLS         | `tls::wrap` reused, SHA256(password)+padding frame in `rust/src/anytls.rs`, server side `wrongsv-anytls::accept_anytls` |
| `websocket`  | VLESS        | WebSocket   | none / TLS     | `wrongsv-websocket`                           |
| `httpupgrade`| VLESS        | HTTPUpgrade | none / TLS     | `evaluator-client/src/transport/httpupgrade.rs` |
| `xhttp`      | VLESS        | XHTTP (h2)  | none / TLS     | `evaluator-client/src/transport/xhttp.rs`: per-connection tokio + h2 + tokio-rustls, sync↔async via mpsc bridges |
| `grpc`       | VLESS        | gRPC (h2 + Hunk frames) | none / TLS | `wrongsv-grpc` (Hunk encode/decode via prost) + per-connection tokio + h2 + tokio-rustls |
| `trojan`     | Trojan       | raw         | TLS            | `wrongsv-protocol` (trojan header)            |
| `mixed`      | SOCKS5/HTTP  | raw         | none           | (inbound only)                                |
| `shadowsocks`| Shadowsocks classic AEAD + AEAD-2022 | raw | none | `wrongsv-shadowsocks`                         |

VLESS Vision flow (`xtls-rprx-vision`) is wired on top of any of the VLESS rows — adapter parses `users[0].flow`, Flutter exposes a checkbox, the client wraps the post-handshake stream in `wrongsv-vless::vision`.

Tests: 34 Rust unit tests + 7 integration tests pass (probe loopback + adapter unit tests + 3 adapter error-path tests + endpoint stack summary + REALITY config parsing + VLESS Vision round-trip + AnyTLS auth-frame round-trip against a self-signed rustls server + VLESS-over-WebSocket-over-TLS combined probe + VLESS-over-XHTTP-over-TLS combined probe + VLESS-over-gRPC-over-TLS combined probe + Shadowsocks classic AEAD and AEAD-2022 round-trips against a fake server). Flutter UI exposes proxy/transport/outer-security pickers, a Vision flow checkbox for VLESS, and locks transport+outer when proxy is mixed or shadowsocks; REALITY and AnyTLS each lock transport to raw and hide themselves for non-VLESS proxies; XHTTP and gRPC transports are restricted to VLESS + (none|TLS) and reset to raw when proxy switches away from VLESS.

## Phase 1 — high-value GFW bypass (done)

### 1a. VLESS + REALITY ✅ (done 2026-06-15)

- ~~Borrow `evaluator-client/src/transport/reality.rs:332 connect_reality(...)` and the `wrongsv-reality` crate.~~ Borrowed auth primitives only (`compute_cert_hmac`, `derive_client_auth_key`, `build_reality_client_hello`, `build_session_id`); inline RFC 8446 TLS 1.3 record layer in `rust/src/reality.rs` to avoid evaluator-client's tokio/quinn/kcp dependency footprint.
- ~~Add `RealityOptions`~~ done. `OuterSecurity::Reality(RealityOptions { server_name, public_key, short_id, raw_pubkey })`.
- ~~Adapter: when `[reality]` table present, parse~~ done. Accepts wrongsv server-style `short_ids = [...]` + `dest = "host:port"` and client-style `short-id` / `server-name`. `public-key` must be supplied by the user (server config holds only `private_key`).
- ~~Flutter: add `reality` to `OuterSecurityKind`~~ done. Reality kind is hidden in dropdown for non-VLESS proxies; selecting it forces transport to raw.
- Probe loopback test deferred (would require a fake REALITY responder driven by rcgen + the wrongsv-reality server handshake — accept remote-only GFW failure per standing rule).

### 1b. VLESS Vision flow ✅ (done 2026-06-15)

- ~~Verify the client side sets `flow` on the VLESS request and performs the inner TLS handshake unmodified.~~ done. `protocol::encode_raw_vless_header` now takes a `flow: &str` and hand-rolls the single-string-field protobuf for `Addons` (avoids the `prost-build` dep that `wrongsv-vless-encoding` would pull in).
- ~~Adapter: `users[0].flow` is already wired through to `VlessOptions.flow`, but `Endpoint::validate` currently rejects~~ done. Validation now accepts `xtls-rprx-vision` and still rejects any other non-empty flow.
- ~~Add `wrongsv-vless` path dep and a `VisionTunnel`~~ done. `rust/src/vision.rs` wraps `Box<dyn Tunnel>` with the public `xtls_padding` / `xtls_unpadding` / `xtls_filter_tls` / `is_complete_record` from `wrongsv-vless::vision`. `try_clone_box` returns `Unsupported` (same cap as REALITY / Shadowsocks — the SOCKS5 relay path needs a future `split()` refactor before Vision works through a SOCKS5 proxy).
- ~~Add `adapts_vless_vision_config` adapter test~~ done. Replaced the rejection test with `adapts_reality_config_with_vision_flow` (Vision accepted) and `rejects_unknown_vless_flow` (other flows still rejected). End-to-end `vision_integration.rs` round-trips "ping-vision" against a wrongsv-vless `VisionReader` + `VisionWriter` server.
- Flutter: VLESS section now has a "XTLS Vision flow" checkbox that toggles the request flow string.

## Phase 2 — transport carriers + outer-security alternates

| Item              | Borrow from                                                          | Notes                                                    |
| ---               | ---                                                                  | ---                                                      |
| gRPC transport ✅ | `evaluator-client/src/transport/grpc.rs:187`, `wrongsv-grpc`         | Done 2026-06-15 — see Phase 2 — gRPC note below.         |
| XHTTP transport ✅ | `evaluator-client/src/transport/xhttp.rs:158`                        | Done 2026-06-15 — see Phase 2 — XHTTP note below.        |
| AnyTLS outer ✅   | `evaluator-client/src/transport/anytls.rs:14`, `wrongsv-anytls`      | Done 2026-06-15 — see Phase 2 — AnyTLS note below.       |
| ShadowTLS outer   | `wrongsv/crates/server/src/handler/shadowtls.rs` (v3 wire protocol)   | `OuterSecurity::ShadowTls(...)` — see ShadowTLS note below |
| WebSocket + TLS ✅ | already implemented; added probe test for the combined stack                 | `tests/ws_tls_integration.rs` exercises VLESS+WS+TLS end-to-end (done 2026-06-15) |

For each: add the `*Options` struct, an adapter branch, a Flutter form section, and a probe test against a wrongsv-server-driven loopback.

### Phase 2/3 async-runtime gate — resolved 2026-06-15 (embed tokio)

User authorized "make decision by yourself, by following existing doc and project target" on 2026-06-15. Decision: embed `tokio` (current_thread runtime, per-connection thread) inside the wrongcl cdylib. Justification: the standing "Keep the Rust part fully capable and establish a complete headless service even without Flutter" target + the "Borrow, do not re-derive" rule both point at reusing the evaluator-client's tokio+h2 transport stack rather than re-deriving it.

Deps added: `tokio` (`rt`, `net`, `sync`, `macros`, `time`, `io-util`), `tokio-rustls 0.26` (`default-features = false`, `logging` + `tls12` + `ring` only, to stay on the existing single crypto provider), `h2 0.4`, `http 1`, `bytes 1`, `prost 0.14`. XHTTP and gRPC both ship now (see notes below). VMess still blocked on transitive wrongsv-server bloat (cleanest fix is upstream `wrongsv-vmess` crate extraction). Phase 4 (QUIC/WebTransport/Hysteria2/TUIC) is **not** unblocked — quinn/h3/wtransport are separate adds.

### Phase 2 — XHTTP ✅ (done 2026-06-15)

- Added `Transport::Xhttp(XhttpOptions { path, host })`. Validation restricts XHTTP to VLESS + (None | TLS) outer — the transport owns the full TCP+TLS+h2 stack internally, so REALITY/AnyTLS would conflict.
- `rust/src/xhttp.rs` spawns a per-connection std thread running a current-thread tokio runtime; bridges sync `mpsc::SyncSender`/`Receiver` ↔ `tokio::sync::mpsc` via `blocking_send`. The runtime owns: `tokio::net::TcpStream`, optional `tokio_rustls::TlsConnector` (clones the user's `rustls::ClientConfig` and injects `h2` ALPN if empty), `h2::client::Builder` with 1 MiB initial window, POST `{scheme}://{authority}{path}` request with `content-type: application/octet-stream`. 10 s handshake timeout.
- `client.rs` gains `open_proxy_stack()` — VLESS / Trojan now dispatch through it, which routes XHTTP to `xhttp::connect_xhttp` (skipping the sync `wrap_outer_then_transport` path) and falls back to TCP+wrap for everything else. `wrap_transport` rejects `Transport::Xhttp` defensively.
- Adapter maps `[xhttp]` table → `WrongsvXhttp { path: "/xhttp", host, tls? }`, generates `OuterSecurity::Tls(...)` when `[xhttp.tls]` is present, else `None`. `active_profile` already selected `xhttp` via the existing `else if cfg.xhttp.is_some()` branch.
- Integration test `tests/xhttp_integration.rs` spawns a tokio listener with a self-signed rustls cert + `h2` ALPN, runs `h2::server::handshake`, accepts a stream, replies 200, reads the VLESS handshake from chunked DATA frames, sends `[0x00, 0x00]`, then echoes the payload. End-to-end VLESS+XHTTP+TLS round-trip passes.
- Flutter: `TransportKind.xhttp` with path + optional host header fields. Proxy onChanged resets transport to raw when proxy moves away from VLESS so the UI can't hold an invalid XHTTP+non-VLESS combo.
- FFI: `wrongcl_native_version` advertises `transports: ["raw", "websocket", "httpupgrade", "xhttp", "grpc"]`.

### Phase 2 — gRPC ✅ (done 2026-06-15)

- Added `Transport::Grpc(GrpcOptions { service_name })`. Validation restricts gRPC to VLESS + (None | TLS) outer — same rationale as XHTTP: the transport owns the full TCP+TLS+h2 stack internally.
- `rust/src/grpc.rs` mirrors the XHTTP per-connection tokio bridge pattern. POST `{scheme}://{authority}/{service_name}/Tun` with `content-type: application/grpc`, `te: trailers`, `grpc-accept-encoding: identity`. Each direction wraps payloads in V2Ray Hunk frames (1B compression flag + 4B BE length + protobuf `Hunk { data: bytes }`) using `wrongsv_grpc::encode_hunk_frame` and `GrpcFrameReader`. 10 s handshake timeout. Read loop calls `feed(&data)` once then `feed(&[])` repeatedly to drain any further buffered frames out of the same h2 chunk.
- Borrows `wrongsv-grpc` (path dep) — `prost 0.14` + the crate's `build.rs`/`hunk.proto` get pulled in transitively. The crate exports `encode_hunk_frame` / `decode_hunk_frame` / `GrpcFrameReader`, so no protobuf or framing logic is re-derived here.
- `client.rs` dispatches `Transport::Grpc` through `open_proxy_stack` to `grpc::connect_grpc`, bypassing the sync `wrap_outer_then_transport` path (same pattern as XHTTP). `wrap_transport` rejects `Transport::Grpc` defensively.
- Adapter maps `[grpc]` table → `WrongsvGrpc { service_name: Option<String>, tls: Option<WrongsvTls> }` (accepts both `service_name` and `service-name` keys). Default service name is `"GunService"`. `OuterSecurity::Tls` generated when `[grpc.tls]` is present, else `None`.
- Integration test `tests/grpc_integration.rs` spawns a tokio h2 server with a self-signed rustls cert + `h2` ALPN, accepts `POST /GunService/Tun`, replies 200 with `content-type: application/grpc`, decodes the VLESS handshake from gRPC Hunk frames, writes `[0x00, 0x00]` wrapped in a gRPC frame, and echoes subsequent gRPC frames. End-to-end VLESS+gRPC+TLS round-trip passes.
- Flutter: `TransportKind.grpc('grpc', 'gRPC')` + `GrpcConfig { serviceName }` (default `GunService`). Trojan onChanged now also resets gRPC→raw alongside the XHTTP downgrade.

### Phase 2 — ShadowTLS note (investigated 2026-06-15)

The original PLAN row pointed at `evaluator-client/src/transport/shadowtls.rs:133` as the borrow source, but on inspection this is a **stub that speaks a different protocol** from the wrongsv server:

- evaluator-client connects with rustls, calls `export_keying_material(b"shadow_tls", ...)`, derives a SHA256 proof from a hardcoded `"eval-stls-pass"` password, then reads/writes 8-byte challenge/response through the TLS plaintext stream.
- The actual wrongsv server (`wrongsv/crates/server/src/handler/shadowtls.rs`, ShadowTLS v3) expects an HMAC-SHA1 in the last 4 bytes of the TLS ClientHello's `session_id`, relays the TLS handshake to a backend TLS server for cover, then switches to APPLICATION_DATA records carrying HMAC-SHA1-tagged payloads.

These are incompatible: a wrongcl client built on the evaluator-client borrow could not connect to the wrongsv server. Real implementation requires lifting (or sharing a crate for) the wire format from `wrongsv/crates/server/src/handler/shadowtls.rs`. Deferred until either (a) the wrongsv repo extracts a `wrongsv-shadowtls` crate from the server handler, or (b) we accept duplicating the encode/decode/ClientHello helpers in `wrongcl/rust/src/shadowtls.rs` and gate the implementation behind a probe test against the real wrongsv server.

### Phase 2 — AnyTLS ✅ (done 2026-06-15)

- Added `OuterSecurity::AnyTls(AnyTlsOptions { server_name, password, insecure_skip_verify, alpn })` plus validation restricting AnyTLS to VLESS + raw transport (mirrors what `wrongsv-anytls::accept_anytls` expects on the server side).
- `rust/src/anytls.rs` borrows `tls::wrap` for the handshake, then writes `SHA256(password) || 0x00 0x00` (matches `evaluator-client/src/transport/anytls.rs` — zero-length padding header). No re-derivation of the TLS record layer.
- Adapter maps `[anytls]` table → `AnyTlsOptions`, defaulting `insecure_skip_verify=true` because wrongsv anytls auto-generates a self-signed cert when no key/cert is supplied.
- Integration test `tests/anytls_integration.rs` spawns a rustls server with a self-signed cert for `localhost`, verifies the SHA256 password hash + padding header on the wire, consumes a VLESS request, replies `0x00 0x00`, and echoes the payload.
- Flutter: AnyTLS appears in the outer-security dropdown only when proxy is VLESS; selecting it forces transport to raw (mirrors REALITY); password + SNI fields + insecure-skip-verify checkbox (default true).
- FFI: `wrongcl_native_version` advertises `outer_security: ["none", "tls", "reality", "anytls"]`. All 9 FFI symbols verified intact.

## Phase 3 — deferred proxy dialects

### 3a. Shadowsocks AEAD-2022 ✅ (done 2026-06-15)

- `wrongsv-shadowsocks::ServerConfig::new` already routes `2022-blake3-aes-128-gcm` / `2022-blake3-aes-256-gcm` and decodes a base64-encoded PSK internally (4 alphabets via `decode_aead_2022_psk`), so `ShadowsocksOptions { method, password }` works as-is — no new base64-PSK field needed.
- `endpoint::validate` now accepts the two AEAD-2022 method names alongside the classic AEAD methods (`rust/src/endpoint.rs:274`).
- `rust/src/shadowsocks.rs` now threads the request salt from `ShadowsocksWriter::new_request` into `ShadowsocksReader::new_response`, which AEAD-2022 requires for response-salt verification. Classic AEAD still works because `new_response` falls through to `new()` when the method is not AEAD-2022.
- In-source fake server handler (`client.rs handle_fake_shadowsocks`) switched from `ShadowsocksWriter::new` to `ShadowsocksWriter::new_response(stream, &config, reader.request_salt())` — same fallback semantics; correct for both ciphers.
- Probe test `probe_works_against_fake_shadowsocks_aead_2022_server` exercises `2022-blake3-aes-128-gcm` end-to-end with a 16-byte base64-zero PSK; existing classic AEAD test remains untouched.
- Flutter: `_shadowsocksMethodDropdown()` now includes the two `2022-*` method names. Password field doubles as the base64 PSK string (no UI distinction needed since the crate auto-decodes).

### 3b. VMess AEAD (custom dialect)

- Borrow `evaluator-client/src/transport/vmess.rs:14 VmessStream` and `:85 connect_vmess(...)`.
- Add `ProxyProtocol::Vmess(VmessOptions { uuid, alter_id, security })`.
- **Constraint:** integration tests prove `wrongcl ↔ wrongsv-server` only. Do NOT add v2fly/xray interop assertions. Mark the Flutter dropdown entry with a "wrongsv-only" hint.
- **Blocked on:** `wrongsv-server::vmess` transitively pulls h3/quinn/kcp/wtransport. The 2026-06-15 async-runtime decision unblocked `tokio` + `h2` + `tokio-rustls` but does NOT cover quinn/h3/kcp/wtransport — cleanest unblock is still an upstream `wrongsv-vmess` crate extraction.

## Phase 4 — QUIC family (needs separate async-runtime work)

These all run over UDP and won't fit the current sync `Tunnel` trait without a per-connection runtime. The 2026-06-15 tokio decision covered HTTP/2-family transports (h2, tokio-rustls) but NOT quinn/h3/wtransport. Adding these is still a significant dep + design ask.

- QUIC carrier — `evaluator-client/src/transport/quic.rs:78`
- WebTransport — `evaluator-client/src/transport/webtransport.rs:81`
- Hysteria2 — separate crate work (no evaluator-client module)
- TUIC — separate crate work (no evaluator-client module)

## Phase 5 — specialized / low-priority

mKCP (`kcp.rs:111`), Meek, gdocsviewer, WireGuard, Naive. Adapter already recognizes the profile keys; flesh them out only on request. `protocol-coverage.md` and `deferred-work.md` track what's blocked upstream.

## Test coverage gaps

- Trojan probe coverage: ✅ `tests/tls_integration.rs::probe_works_against_trojan_over_tls_server` (already in place).
- Combined-stack probes: ✅ `tests/ws_tls_integration.rs` (VLESS+WS+TLS, Phase 2), `tests/xhttp_integration.rs` (VLESS+XHTTP+TLS, Phase 2), and `tests/grpc_integration.rs` (VLESS+gRPC+TLS, Phase 2).
- Adapter error-path coverage: now exercised by `rejects_unparseable_listen_string`, `rejects_unimplemented_profile`, and `rejects_trojan_with_empty_password` in `adapter::tests`.
- Flutter widget test now exercises both the "Start proxy" button and the "Run probe" button (FakeWrongclClient tracks `startCount` + `probeCount`, asserts the "probe succeeded" message renders). Probe-button assertion was the last documented coverage gap.

## Design constraints (from `wrongsv/docs/`)

- `design_constraint.md` — three-layer separation: proxy ↔ transport ↔ outer-security. Adapter must keep these orthogonal; do not collapse e.g. "Trojan + TLS" into a single `Trojan` variant.
- `protocol-coverage.md` — tier table for which protocols are first-class vs. parity-only. Phase 1/2 target tier-1; Phase 5 is tier-3.
- `deferred-work.md` — known upstream blockers; check before promising any Phase 5 item.

## Repo layout reminder

- `wrongcl/rust/src/{client,adapter,endpoint,config,error}.rs` — Rust core
- `wrongcl/rust/src/ffi.rs` (or equivalent) — 9 exported FFI symbols, verified via `nm -D --defined-only target/debug/libwrongcl_native.so`
- `wrongcl/lib/{app,wrongcl_client}.dart` — Flutter UI + FFI shim
- `wrongcl/test/widget_test.dart` — Flutter widget test (Dart SDK requirement is pre-existing infra mismatch; run Rust tests for verification instead)
- `wrongsv/crates/` — 15 crates; relative path deps from `wrongcl/rust/Cargo.toml`
