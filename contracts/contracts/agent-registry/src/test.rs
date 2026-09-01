#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

#[test]
fn test_registration_and_revocation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    // Init
    client.init(&admin);

    // Check not authorized initially
    assert!(!client.is_authorized(&agent));

    // Register
    client.register(&agent);
    assert!(client.is_authorized(&agent));

    // Revoke
    client.revoke_agent(&agent);
    assert!(!client.is_authorized(&agent));
}

#[test]
fn test_require_authorized_not_registered() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    client.init(&admin);

    let result = client.try_require_authorized(&agent);
    assert_eq!(result, Err(Ok(Error::NotRegistered)));
}

#[test]
fn test_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    let result = client.try_init(&admin);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_get_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    assert_eq!(client.get_admin(), admin);
    assert!(client.get_pending_admin().is_none());
}

// ── Scope handling ──────────────────────────────────────────────────────────

#[test]
fn test_register_then_grant_and_check_scopes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);

    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::RENEWALS)));
    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS)));

    // Grant Renewals + Approvals.
    let mask = ScopeSet(ScopeSet::RENEWALS | ScopeSet::APPROVALS);
    client.update_scopes(&agent, &mask);

    assert!(client.has_scope(&agent, &ScopeSet(ScopeSet::RENEWALS)));
    assert!(client.has_scope(&agent, &ScopeSet(ScopeSet::APPROVALS)));
    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS)));
}

#[test]
fn test_update_scopes_requires_registered_agent() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);

    // Updating scopes for an unregistered agent is rejected.
    let result = client.try_update_scopes(&agent, &ScopeSet(ScopeSet::RENEWALS));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_require_scope_succeeds_with_scope() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);
    client.update_scopes(&agent, &ScopeSet(ScopeSet::GIFTCARDS));

    // Should not panic.
    let result = client.require_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS));
    assert!(result.is_ok());
}

#[test]
fn test_require_scope_missing_scope() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);
    client.update_scopes(&agent, &ScopeSet(ScopeSet::RENEWALS));

    let result = client.try_require_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS));
    assert_eq!(result, Err(Ok(Error::MissingScope)));
}

// ── Bug condition exploration tests ──────────────────────────────────────────

/// isBugCondition_Authorized: address has no storage entry.
#[test]
fn test_fault_condition_authorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unregistered_agent = Address::generate(&env);

    client.init(&admin);

    let result = client.try_require_authorized(&unregistered_agent);
    assert_eq!(result, Err(Ok(Error::NotRegistered)));
}

/// isBugCondition_Scope (bit not set): agent registered with Renewals, demand GiftCards.
#[test]
fn test_fault_condition_scope_missing_bit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);
    client.update_scopes(&agent, &ScopeSet(ScopeSet::RENEWALS));

    let result = client.try_require_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS));
    assert_eq!(result, Err(Ok(Error::MissingScope)));
}

/// isBugCondition_Scope (stored_mask is None): completely unregistered agent.
#[test]
fn test_fault_condition_scope_unregistered() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unregistered_agent = Address::generate(&env);

    client.init(&admin);

    let result = client.try_require_scope(&unregistered_agent, &ScopeSet(ScopeSet::RENEWALS));
    assert_eq!(result, Err(Ok(Error::MissingScope)));
}

// ── Preservation tests ────────────────────────────────────────────────────────

/// NOT isBugCondition_Authorized: registered agent → require_authorized succeeds.
#[test]
fn test_preservation_authorized_ok() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);

    let result = client.try_require_authorized(&agent);
    assert!(result.is_ok(), "require_authorized should succeed for a registered agent");
}

/// NOT isBugCondition_Scope: agent has the requested scope → require_scope succeeds.
#[test]
fn test_preservation_scope_ok() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);
    client.update_scopes(&agent, &ScopeSet(ScopeSet::RENEWALS));

    let result = client.try_require_scope(&agent, &ScopeSet(ScopeSet::RENEWALS));
    assert!(result.is_ok(), "require_scope should succeed when the agent has the requested scope");
}

/// is_authorized returns the correct bool across the register → revoke lifecycle.
#[test]
fn test_preservation_is_authorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);

    // Unregistered → false.
    assert!(!client.is_authorized(&agent));

    // After register → true.
    client.register(&agent);
    assert!(client.is_authorized(&agent));

    // After revoke → false again.
    client.revoke_agent(&agent);
    assert!(!client.is_authorized(&agent));
}

/// has_scope returns the correct bool before and after update_scopes.
#[test]
fn test_preservation_has_scope() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.register(&agent);

    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::RENEWALS)));
    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS)));

    client.update_scopes(&agent, &ScopeSet(ScopeSet::RENEWALS));

    assert!(client.has_scope(&agent, &ScopeSet(ScopeSet::RENEWALS)));
    assert!(!client.has_scope(&agent, &ScopeSet(ScopeSet::GIFTCARDS)));
}

// ── Two-step admin transfer ───────────────────────────────────────────────────

#[test]
fn test_two_step_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin);

    client.transfer_admin(&new_admin);
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

    client.accept_admin();
    assert_eq!(client.get_admin(), new_admin);
    assert!(client.get_pending_admin().is_none());
}

#[test]
fn test_cancel_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin);
    client.transfer_admin(&new_admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin));

    client.cancel_transfer_admin();
    assert!(client.get_pending_admin().is_none());
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_accept_admin_without_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    let result = client.try_accept_admin();
    assert_eq!(result, Err(Ok(Error::NoPendingAdmin)));
}

#[test]
fn test_new_admin_can_administer_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let agent = Address::generate(&env);

    client.init(&admin);
    client.transfer_admin(&new_admin);
    client.accept_admin();

    client.register(&agent);
    assert!(client.is_authorized(&agent));
}

// ── Additional ScopeSet logic tests ──────────────────────────────────────────

#[test]
fn test_scopeset_logic() {
    let s1 = ScopeSet(ScopeSet::RENEWALS);
    let s2 = ScopeSet(ScopeSet::GIFTCARDS);
    
    assert!(s1.is_valid());
    assert!(s2.is_valid());
    assert!(!ScopeSet(1 << 5).is_valid()); // Invalid bit

    let union_scope = s1.union(&s2);
    assert_eq!(union_scope.0, ScopeSet::RENEWALS | ScopeSet::GIFTCARDS);
    assert!(union_scope.contains(&s1));
    assert!(union_scope.contains(&s2));

    let diff = union_scope.difference(&s1);
    assert_eq!(diff.0, ScopeSet::GIFTCARDS);
}

#[test]
fn test_update_scopes_invalid() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    client.init(&admin);
    client.register(&agent);

    // Invalid scope bits
    let res = client.try_update_scopes(&agent, &ScopeSet(1 << 5));
    assert_eq!(res, Err(Ok(Error::InvalidScope)));
}

#[test]
fn test_require_scopes() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AgentRegistry);
    let client = AgentRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let agent = Address::generate(&env);
    client.init(&admin);
    client.register(&agent);

    let scope = ScopeSet(ScopeSet::RENEWALS | ScopeSet::GIFTCARDS);
    client.update_scopes(&agent, &scope);

    // Should succeed
    let res1 = client.require_scopes(&agent, &scope);
    assert!(res1.is_ok());
    let res2 = client.require_scope(&agent, &ScopeSet(ScopeSet::RENEWALS));
    assert!(res2.is_ok());
}