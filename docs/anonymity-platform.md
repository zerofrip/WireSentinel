# WireSentinel Anonymity Platform (Phase 13)

Phase 13 integrates the **WireSentinel-Anonymity** workspace with WireSentinel Core.

## Providers

| Provider | Route variant | Description |
|---|---|---|
| Katzenpost | `TrafficRoute::Katzenpost` | Mixnet-style message routing via Katzenpost gateways |
| Loopix | `TrafficRoute::Loopix` | Loopix provider-backed anonymous routing |
| Federated mixnet | `TrafficRoute::FederatedMixnet` | Multi-provider federated anonymity path |

Anonymous nested routes also support `AnonymousRoute::Katzenpost`, `Loopix`, and `FederatedMixnet`.

## REST API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/anonymity` | Provider status snapshot |
| GET | `/api/v1/anonymity/entropy` | Route entropy + anonymity set estimate |
| GET/POST | `/api/v1/anonymity/services` | Anonymous service registry |
| POST | `/api/v1/anonymity/decoy/simulate` | Lab-mode decoy route simulation |
| GET | `/api/v1/privacy/anonymity` | Advanced privacy analytics snapshot |

## Storage

Migration `011_katzenpost_loopix_anonymous_services.sql` adds:

- `katzenpost_profiles`, `katzenpost_gateways`
- `loopix_profiles`, `loopix_providers`
- `anonymous_services`, `anonymous_service_endpoints`
- Extended `privacy_analytics` optional metrics columns

## Kernel / NDIS

`GuardianRouteKind` / `NdisRouteKind` add values `Katzenpost=8`, `Loopix=9`, `FederatedMixnet=10`.

WFP kernel mapping in `wfp/src/kernel.rs` forwards the new `TrafficRoute` variants.

## Plugins

`PluginCapability::AnonymityBackend` and `AnonymityBackendPlugin` allow third-party anonymity backends.
Use `PluginManager::list_anonymity_providers()` to enumerate installed backends.

## Security

- `AnonymitySecurityPolicy` validates gateways, federation configs, and plugin providers.
- Decoy routing requires lab mode (`AnonymitySecurityPolicy::set_lab_mode(true)`).
- Violations emit `AnonymitySecurityViolation` events.

## Workspace dependencies

WireSentinel Core path-depends on `../WireSentinel-Anonymity` crates:

- `anonymity-core`, `katzenpost`, `loopix`, `federation`
- `entropy`, `discovery`, `decoy-routing`, `analytics`, `cover-traffic`, `sdk`
