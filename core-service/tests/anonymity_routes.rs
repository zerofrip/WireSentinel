use core_service::anonymity_decoy::AnonymityDecoyService;
use core_service::anonymity_security::AnonymitySecurityPolicy;
use event_bus::EventBus;

#[test]
fn decoy_simulation_works_in_lab_mode() {
    let events = EventBus::new();
    let security = AnonymitySecurityPolicy::new(events.clone());
    security.set_lab_mode(true);
    let decoy = AnonymityDecoyService::new(&security, events);
    let route = decoy.create_route("lab-target").unwrap();
    let hops = decoy.simulate(&route).unwrap();
    assert!(hops > 0);
}

#[test]
fn decoy_simulation_blocked_outside_lab_mode() {
    let events = EventBus::new();
    let security = AnonymitySecurityPolicy::new(events.clone());
    let decoy = AnonymityDecoyService::new(&security, events);
    assert!(decoy.create_route("blocked-target").is_err());
}
