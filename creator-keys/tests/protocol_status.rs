//! Integration tests for `get_protocol_status` view function.

mod contract_test_env;

use contract_test_env::{register_creator_keys, test_env_with_auths};
use creator_keys::constants::storage;
use creator_keys::{CreatorKeysContractClient, ProtocolStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};

#[test]
fn test_protocol_status_defaults() {
    let env = test_env_with_auths();
    let (client, _) = register_creator_keys(&env);

    let status = client.get_protocol_status();
    assert_eq!(
        status,
        ProtocolStatus {
            global_trading_paused: false,
            protocol_fee_bps: 0,
            treasury_address: None,
            lockup_duration_seconds: 86_400,
            min_investment_amount: None,
        }
    );
}

#[test]
fn test_protocol_status_configured() {
    let env = test_env_with_auths();
    let (client, _) = register_creator_keys(&env);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.set_protocol_admin(&admin, &admin);

    client.set_protocol_fee(&admin, &250, &treasury);
    client.set_lockup_duration(&admin, &120);
    client.set_min_investment_amount(&admin, &5000);

    let status = client.get_protocol_status();
    assert_eq!(
        status,
        ProtocolStatus {
            global_trading_paused: false,
            protocol_fee_bps: 250,
            treasury_address: Some(treasury.clone()),
            lockup_duration_seconds: 120,
            min_investment_amount: Some(5000),
        }
    );
    assert_eq!(client.get_min_investment_amount(), Some(5000));
}

#[test]
fn test_protocol_status_global_pause_toggle() {
    let env = test_env_with_auths();
    let (client, _) = register_creator_keys(&env);

    let admin = Address::generate(&env);
    client.set_protocol_admin(&admin, &admin);

    let signers = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    client.set_global_pause_admins(
        &admin,
        &vec![
            &env,
            signers[0].clone(),
            signers[1].clone(),
            signers[2].clone(),
        ],
    );

    // Initially unpaused
    assert_eq!(client.get_protocol_status().global_trading_paused, false);

    // Admin 1 votes to pause: still not paused (threshold is 2)
    client.global_pause(&signers[0]);
    assert_eq!(client.get_protocol_status().global_trading_paused, false);

    // Admin 2 votes to pause: pause activates
    client.global_pause(&signers[1]);
    assert_eq!(client.get_protocol_status().global_trading_paused, true);

    // Admin 1 votes to resume: still paused
    client.global_resume(&signers[0]);
    assert_eq!(client.get_protocol_status().global_trading_paused, true);

    // Admin 2 votes to resume: unpaused
    client.global_resume(&signers[1]);
    assert_eq!(client.get_protocol_status().global_trading_paused, false);
}

#[test]
fn test_protocol_status_bumps_ttl_on_existing_entries() {
    let env = test_env_with_auths();
    let (client, _) = register_creator_keys(&env);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.set_protocol_admin(&admin, &admin);

    client.set_protocol_fee(&admin, &100, &treasury);
    client.set_lockup_duration(&admin, &3600);
    client.set_min_investment_amount(&admin, &1000);

    // Fast forward ledger sequence
    env.ledger().set_sequence_number(100_000);

    // Calling get_protocol_status bumps TTL on all present entries without error
    let status = client.get_protocol_status();
    assert_eq!(status.protocol_fee_bps, 100);
    assert_eq!(status.treasury_address, Some(treasury));
    assert_eq!(status.lockup_duration_seconds, 3600);
    assert_eq!(status.min_investment_amount, Some(1000));
}
