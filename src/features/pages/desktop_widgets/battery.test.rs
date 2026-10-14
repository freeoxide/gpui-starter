use gpui_kit::component::IconName;

use super::battery_model;
use crate::system_controls::{BatteryChargeState, BatteryStatus, ControlState};

fn ready(charge: f32, state: BatteryChargeState) -> ControlState<BatteryStatus> {
    ControlState::Ready(BatteryStatus {
        charge,
        state,
        low_power_mode: None,
    })
}

#[test]
fn ready_models_all_six_charge_states() {
    let cases = [
        (
            BatteryChargeState::Charging,
            Some("widgets_battery_charging"),
        ),
        (BatteryChargeState::Full, Some("widgets_battery_full")),
        (
            BatteryChargeState::Discharging,
            Some("widgets_battery_discharging"),
        ),
        (BatteryChargeState::Empty, None),
        (BatteryChargeState::Paused, None),
        (BatteryChargeState::Unknown, None),
    ];
    for (state, state_key) in cases {
        let model = battery_model(&ready(0.8, state));
        assert_eq!(model.state_key, state_key, "{state:?}");
        assert_eq!(model.pct, "80%");
        assert!(model.no_data.is_none());
    }
    assert!(matches!(
        battery_model(&ready(0.8, BatteryChargeState::Charging)).icon,
        IconName::BatteryCharging
    ));
    assert!(matches!(
        battery_model(&ready(0.8, BatteryChargeState::Full)).icon,
        IconName::BatteryFull
    ));
    assert!(matches!(
        battery_model(&ready(0.8, BatteryChargeState::Empty)).icon,
        IconName::BatteryLow
    ));
    assert!(matches!(
        battery_model(&ready(0.8, BatteryChargeState::Paused)).icon,
        IconName::Battery
    ));
    assert!(matches!(
        battery_model(&ready(0.8, BatteryChargeState::Unknown)).icon,
        IconName::Battery
    ));
}

#[test]
fn discharging_charge_tiers_pick_the_tiered_icons() {
    let warning = battery_model(&ready(0.05, BatteryChargeState::Discharging));
    assert!(matches!(warning.icon, IconName::BatteryWarning));
    let low = battery_model(&ready(0.2, BatteryChargeState::Discharging));
    assert!(matches!(low.icon, IconName::BatteryLow));
    let medium = battery_model(&ready(0.5, BatteryChargeState::Discharging));
    assert!(matches!(medium.icon, IconName::BatteryMedium));
}

#[test]
fn unsupported_sets_no_data_with_the_reason_key() {
    let model = battery_model(&ControlState::Unsupported {
        reason_key: "system_controls_reason_web",
    });
    assert!(matches!(model.icon, IconName::Battery));
    assert_eq!(model.pct, "--");
    assert_eq!(model.state_key, None);
    assert_eq!(model.no_data.as_deref(), Some("system_controls_reason_web"));
}

#[test]
fn unavailable_carries_the_error_text() {
    let model = battery_model(&ControlState::Unavailable {
        error: "no battery".into(),
    });
    assert_eq!(model.pct, "--");
    assert_eq!(model.no_data.as_deref(), Some("no battery"));
}
