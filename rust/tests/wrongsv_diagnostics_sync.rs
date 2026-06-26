use std::fs;
use std::path::PathBuf;

use wrongcl_native::adapter::{BaseCarrier, PayloadNetwork};
use wrongcl_native::inspect_wrongsv_config;

#[test]
fn wrongcl_inspection_matches_wrongsv_endpoint_resolution_for_repo_fixtures() {
    let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../wrongsv/configs");
    let mut fixture_paths = fs::read_dir(&config_dir)
        .expect("wrongsv config directory should exist")
        .map(|entry| entry.expect("dir entry should load").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("toml"))
        .collect::<Vec<_>>();
    fixture_paths.sort();

    assert!(
        !fixture_paths.is_empty(),
        "expected wrongsv config fixtures in {}",
        config_dir.display()
    );
    assert!(
        fixture_paths
            .iter()
            .any(|path| path.file_name().and_then(|value| value.to_str()) == Some("naive.toml")),
        "expected wrongsv Naive fixture to keep protocols.md coverage visible"
    );

    for path in fixture_paths {
        let report = inspect_wrongsv_config(&path).unwrap_or_else(|error| {
            panic!("wrongcl inspect failed for {}: {error}", path.display())
        });
        let inspection = wrongsv::inspect_server_config_path(&path).unwrap_or_else(|error| {
            panic!(
                "wrongsv endpoint inspection failed for {}: {error}",
                path.display()
            )
        });
        let payload_networks = inspection
            .payload_networks
            .into_iter()
            .map(map_payload_network)
            .collect::<Vec<_>>();
        let base_carriers = inspection
            .base_carriers
            .into_iter()
            .map(map_base_carrier)
            .collect::<Vec<_>>();
        let fixture_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("fixture");

        assert_eq!(
            report.active_profile, inspection.active_profile,
            "{fixture_name}: active profile drifted from wrongsv"
        );
        assert_eq!(
            report.payload_networks, payload_networks,
            "{fixture_name}: payload networks drifted from wrongsv"
        );
        assert_eq!(
            report.base_carriers, base_carriers,
            "{fixture_name}: base carriers drifted from wrongsv"
        );
    }
}

fn map_payload_network(payload: wrongsv::PayloadNetworkId) -> PayloadNetwork {
    match payload {
        wrongsv::PayloadNetworkId::Tcp => PayloadNetwork::Tcp,
        wrongsv::PayloadNetworkId::Udp => PayloadNetwork::Udp,
        wrongsv::PayloadNetworkId::Ip => PayloadNetwork::Ip,
    }
}

fn map_base_carrier(carrier: wrongsv::BaseCarrierId) -> BaseCarrier {
    match carrier {
        wrongsv::BaseCarrierId::Tcp => BaseCarrier::Tcp,
        wrongsv::BaseCarrierId::Udp => BaseCarrier::Udp,
    }
}
