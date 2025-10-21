// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	mock::{
		exec_as_lite_person_registration_tx, exec_as_lite_person_tx, exec_signed_tx, new_test_ext,
		RuntimeCall, Test,
	},
	pallet::{AttestationAllowance, LitePeople, UnclaimedAttestations},
	Pallet, PROOF_OF_OWNERSHIP_PREFIX,
};
use codec::Encode;
use frame_support::assert_ok;
use sp_runtime::{
	transaction_validity::{InvalidTransaction, TransactionValidityError},
	DispatchError,
};
use verifiable::GenerateVerifiable;

type SecretOfTest = <<Test as crate::Config>::Crypto as GenerateVerifiable>::Secret;

fn poo_msg(who: u64) -> Vec<u8> {
	who.using_encoded(|b| [&PROOF_OF_OWNERSHIP_PREFIX[..], b].concat())
}

fn secret_from_seed(seed: u8) -> SecretOfTest {
	<Test as crate::Config>::Crypto::new_secret([seed; 32])
}

fn member_from_secret(sec: &SecretOfTest) -> crate::MemberOf<Test> {
	<Test as crate::Config>::Crypto::member_from_secret(sec)
}

fn sign_poo_with_secret(sec: &SecretOfTest, who: u64) -> crate::SignatureOf<Test> {
	<Test as crate::Config>::Crypto::sign(sec, &poo_msg(who)[..]).expect("sign ok")
}

fn sign_poo_for(
	who: u64,
	member: crate::MemberOf<Test>,
) -> <<Test as crate::Config>::Crypto as GenerateVerifiable>::Signature {
	let msg = who.using_encoded(|bytes| [&PROOF_OF_OWNERSHIP_PREFIX[..], bytes].concat());
	<Test as crate::Config>::Crypto::sign(&member, &msg[..]).unwrap()
}

#[test]
fn increase_attestation_allowance_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 42;
		let count: u32 = 5;

		// Pre-condition: no allowance.
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);

		// Signed (non-root) origin tries to increase allowance -> should fail with BadOrigin.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::increase_attestation_allowance {
			account: verifier,
			count,
		});
		let err = exec_signed_tx(1, call).expect_err("must fail for non-root origin");

		assert_eq!(err.unwrap_dispatch().error, DispatchError::BadOrigin);
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);
	});
}

#[test]
fn increase_attestation_allowance_succeeds() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 7;
		let count: u32 = 10;

		// Root increases allowance -> should succeed.
		assert_ok!(Pallet::<Test>::increase_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			verifier,
			count
		));

		assert_eq!(AttestationAllowance::<Test>::get(verifier), count);
	});
}

#[test]
fn clear_attestation_allowance_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 11;
		let limit: u32 = 50;

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::clear_attestation_allowance {
			account: verifier,
			limit,
		});
		let err = exec_signed_tx(1, call).expect_err("must fail for non-root origin");

		assert_eq!(err.unwrap_dispatch().error, DispatchError::BadOrigin);
	});
}

#[test]
fn clear_attestation_allowance_succeeds() {
	new_test_ext().execute_with(|| {
		let verifier: u64 = 12;

		// Seed allowance and a couple of unclaimed attestations.
		AttestationAllowance::<Test>::insert(verifier, 3);
		UnclaimedAttestations::<Test>::insert(verifier, 200u64, ());
		UnclaimedAttestations::<Test>::insert(verifier, 201u64, ());

		// Root clears everything.
		assert_ok!(Pallet::<Test>::clear_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			verifier,
			100, // high enough to clear all
		));

		// Allowance removed and unclaimed attestations cleared.
		assert_eq!(AttestationAllowance::<Test>::get(verifier), 0);
		assert!(!AttestationAllowance::<Test>::contains_key(verifier));
		assert!(!UnclaimedAttestations::<Test>::contains_key(verifier, 200u64));
		assert!(!UnclaimedAttestations::<Test>::contains_key(verifier, 201u64));
	});
}

#[test]
fn set_attestation_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let attestation: u64 = 300;

		// Root is not allowed for a signed-only call.
		let err =
			Pallet::<Test>::set_attestation(frame_system::RawOrigin::Root.into(), attestation)
				.expect_err("root is not a valid origin for set_attestation");

		assert_eq!(err, DispatchError::BadOrigin);
		assert!(!UnclaimedAttestations::<Test>::contains_key(1u64, attestation));
	});
}

#[test]
fn set_attestation_fails_for_no_attestation_allowance() {
	new_test_ext().execute_with(|| {
		let who: u64 = 13;
		let attestation: u64 = 301;

		// No allowance seeded -> must fail with NoAttestationAllowance.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::set_attestation { attestation });
		let err = exec_signed_tx(who, call).expect_err("must fail without allowance");

		assert_eq!(
			err.unwrap_dispatch().error,
			crate::Error::<Test>::NoAttestationAllowance.into()
		);
		assert!(!UnclaimedAttestations::<Test>::contains_key(who, attestation));
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);
	});
}

#[test]
fn set_attestation_fails_for_already_set_attestation() {
	new_test_ext().execute_with(|| {
		let who: u64 = 14;
		let attestation: u64 = 302;

		// Seed one available allowance and pre-set the attestation.
		AttestationAllowance::<Test>::insert(who, 1);
		UnclaimedAttestations::<Test>::insert(who, attestation, ());

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::set_attestation { attestation });
		let err = exec_signed_tx(who, call).expect_err("must fail for duplicate attestation");

		assert_eq!(err.unwrap_dispatch().error, crate::Error::<Test>::AttestationAlreadySet.into());

		// Storage unchanged (allowance still 1; attestation still present).
		assert_eq!(AttestationAllowance::<Test>::get(who), 1);
		assert!(UnclaimedAttestations::<Test>::contains_key(who, attestation));
	});
}

#[test]
fn set_attestation_succeeds() {
	new_test_ext().execute_with(|| {
		let who: u64 = 15;
		let attestation: u64 = 303;

		// Seed exactly one allowance.
		AttestationAllowance::<Test>::insert(who, 1);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::set_attestation { attestation });
		assert_ok!(exec_signed_tx(who, call));

		// Attestation recorded; allowance consumed (removed).
		assert!(UnclaimedAttestations::<Test>::contains_key(who, attestation));
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);
		assert!(!AttestationAllowance::<Test>::contains_key(who));
	});
}

#[test]
fn cancel_attestation_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let attestation: u64 = 400;

		let err =
			Pallet::<Test>::cancel_attestation(frame_system::RawOrigin::Root.into(), attestation)
				.expect_err("root is not a valid origin for cancel_attestation");

		assert_eq!(err, DispatchError::BadOrigin);
	});
}

#[test]
fn cancel_attestation_fails_for_no_attestation() {
	new_test_ext().execute_with(|| {
		let who: u64 = 16;
		let attestation: u64 = 401;

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::cancel_attestation { attestation });
		let err = exec_signed_tx(who, call).expect_err("must fail when nothing to cancel");

		assert_eq!(err.unwrap_dispatch().error, crate::Error::<Test>::NoAttestation.into());
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);
	});
}

#[test]
fn cancel_attestation_fails_for_not_being_the_attester() {
	new_test_ext().execute_with(|| {
		let attester: u64 = 17;
		let not_attester: u64 = 18;
		let attestation: u64 = 402;

		// Someone else created the attestation.
		UnclaimedAttestations::<Test>::insert(attester, attestation, ());

		// Another user tries to cancel -> behaves as "no attestation" for them.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::cancel_attestation { attestation });
		let err = exec_signed_tx(not_attester, call).expect_err("must fail for wrong signer");

		assert_eq!(err.unwrap_dispatch().error, crate::Error::<Test>::NoAttestation.into());

		// Original record still intact; allowances unchanged.
		assert!(UnclaimedAttestations::<Test>::contains_key(attester, attestation));
		assert_eq!(AttestationAllowance::<Test>::get(attester), 0);
		assert_eq!(AttestationAllowance::<Test>::get(not_attester), 0);
	});
}

#[test]
fn cancel_attestation_succeeds() {
	new_test_ext().execute_with(|| {
		let who: u64 = 19;
		let attestation: u64 = 403;

		// Seed an attestation and zero allowance.
		UnclaimedAttestations::<Test>::insert(who, attestation, ());
		assert_eq!(AttestationAllowance::<Test>::get(who), 0);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::cancel_attestation { attestation });
		assert_ok!(exec_signed_tx(who, call));

		// Attestation removed; allowance incremented.
		assert!(!UnclaimedAttestations::<Test>::contains_key(who, attestation));
		assert_eq!(AttestationAllowance::<Test>::get(who), 1);
	});
}

#[test]
fn register_fails_for_wrong_origin_when_not_using_tx_extension() {
	new_test_ext().execute_with(|| {
		let who: u64 = 30;
		let verifier: u64 = 31;
		let attestation: u64 = 777;
		let ring_vrf_key: crate::MemberOf<Test> = [55u8; 32];
		let special_key = [7u8; 32];
		let proof = sign_poo_for(who, ring_vrf_key);
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		// Try to register via a plain signed tx (no PeopleLiteAuth data) -> BadOrigin.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier,
			attestation,
			attestation_signature,
		});
		let err = exec_signed_tx(who, call).expect_err("must fail without tx extension");
		assert_eq!(err.unwrap_dispatch().error, DispatchError::BadOrigin);
	});
}

#[test]
fn register_fails_for_no_attestation() {
	new_test_ext().execute_with(|| {
		let who: u64 = 40;
		let verifier: u64 = 41;
		let attestation: u64 = 888;
		let ring_vrf_key: crate::MemberOf<Test> = [66u8; 32];
		let special_key = [1u8; 32];
		let proof = sign_poo_for(who, ring_vrf_key);
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		// No (verifier, attestation) present -> validation must reject.
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier,
			attestation,
			attestation_signature,
		});
		let err = exec_as_lite_person_registration_tx(who, call, 0)
			.expect_err("validation must fail when invite is missing");

		match err {
			crate::mock::TransactionExecutionError::Validity(
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code)),
			) => {
				assert_eq!(code, crate::extension::CustomError::NoUnclaimedAttestation as u8);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	});
}

#[test]
fn register_fails_for_invalid_proof_of_ownership() {
	new_test_ext().execute_with(|| {
		let who: u64 = 50;
		let verifier: u64 = 51;
		let attestation: u64 = 889;

		// Seed the unclaimed attestation so we get past the first check.
		UnclaimedAttestations::<Test>::insert(verifier, attestation, ());

		// Valid ring key derived from `secret_ok`, but we sign with `secret_wrong`.
		let secret_ok = secret_from_seed(1);
		let secret_wrong = secret_from_seed(2);
		let ring_ok: crate::MemberOf<Test> = member_from_secret(&secret_ok);
		let special_key = [2u8; 32];

		let bad_proof = sign_poo_with_secret(&secret_wrong, who);
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key: ring_ok,
			special_key,
			proof_of_ownership: bad_proof,
			verifier,
			attestation,
			attestation_signature,
		});
		let err = exec_as_lite_person_registration_tx(who, call, 0)
			.expect_err("validation must fail with invalid proof-of-ownership");

		match err {
			crate::mock::TransactionExecutionError::Validity(
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code)),
			) => {
				assert_eq!(code, crate::extension::CustomError::InvalidProofOfOwnership as u8);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	});
}

#[test]
fn register_fails_for_invalid_attestation_signature() {
	new_test_ext().execute_with(|| {
		let who: u64 = 60;
		let verifier: u64 = 61;
		let attestation: u64 = 890;

		// Seed the unclaimed attestation.
		UnclaimedAttestations::<Test>::insert(verifier, attestation, ());

		let ring_vrf_key: crate::MemberOf<Test> = [99u8; 32];
		let special_key = [3u8; 32];
		let proof = sign_poo_for(who, ring_vrf_key);

		// Deliberately wrong attestation signature (doesn't match `attestation`).
		let bad_attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation + 1);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier,
			attestation,
			attestation_signature: bad_attestation_signature,
		});
		let err = exec_as_lite_person_registration_tx(who, call, 0)
			.expect_err("validation must fail with invalid attestation signature");

		match err {
			crate::mock::TransactionExecutionError::Validity(
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code)),
			) => {
				assert_eq!(code, crate::extension::CustomError::InvalidAttestationSignature as u8);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	});
}

#[test]
fn register_fails_for_already_registered() {
	new_test_ext().execute_with(|| {
		let who: u64 = 70;
		let verifier: u64 = 71;
		let attestation: u64 = 891;

		// Make sure the attestation exists so we get to the "already registered" check.
		UnclaimedAttestations::<Test>::insert(verifier, attestation, ());

		// First register using a valid (member, secret, signature) tuple.
		let secret = secret_from_seed(7);
		let ring_vrf_key: crate::MemberOf<Test> = member_from_secret(&secret);
		let special_key = [4u8; 32];
		let proof = sign_poo_with_secret(&secret, who);
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		// Pre-register the account by calling the pallet directly with the proper origin.
		// (This bypasses the tx extension logic—which is fine for setup.)
		assert_ok!(Pallet::<Test>::register(
			crate::Origin::<Test>::LitePersonRegistration(who).into(),
			ring_vrf_key,
			special_key,
			proof,
			verifier,
			attestation,
			attestation_signature.clone(),
		));
		assert!(LitePeople::<Test>::contains_key(who));

		// Re-seed the invite so the tx-extension proceeds past the invite check to
		// `AlreadyRegistered`.
		UnclaimedAttestations::<Test>::insert(verifier, attestation, ());

		// Try to register again via the tx extension -> must fail "AlreadyRegistered".
		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier,
			attestation,
			attestation_signature,
		});
		let err = exec_as_lite_person_registration_tx(who, call, 1)
			.expect_err("must fail because the account is already registered");

		match err {
			crate::mock::TransactionExecutionError::Validity(
				TransactionValidityError::Invalid(InvalidTransaction::Custom(code)),
			) => {
				assert_eq!(code, crate::extension::CustomError::AlreadyRegistered as u8);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	});
}

#[test]
fn register_succeeds() {
	new_test_ext().execute_with(|| {
		let who: u64 = 80;
		let verifier: u64 = 81;
		let attestation: u64 = 892;

		// Seed the unclaimed attestation required by validation.
		UnclaimedAttestations::<Test>::insert(verifier, attestation, ());

		// Derive a valid key pair for Simple and sign proof-of-ownership with the matching secret.
		let secret = secret_from_seed(9);
		let ring_vrf_key: crate::MemberOf<Test> = member_from_secret(&secret);
		let special_key = [5u8; 32];
		let proof = sign_poo_with_secret(&secret, who);
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		let call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier,
			attestation,
			attestation_signature,
		});
		assert_ok!(exec_as_lite_person_registration_tx(who, call, 0));

		// Effects:
		// - Unclaimed attestation removed,
		// - person stored,
		// - sufficients incremented by the call.
		assert!(!UnclaimedAttestations::<Test>::contains_key(verifier, attestation));
		assert!(LitePeople::<Test>::contains_key(who));
		assert_eq!(frame_system::Account::<Test>::get(who).sufficients, 1);
	});
}

#[test]
fn dispatch_as_signer_fails_for_wrong_origin() {
	new_test_ext().execute_with(|| {
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"nope".to_vec(),
		});

		// Call pallet directly with Root -> must fail (requires LitePerson origin).
		let err = Pallet::<Test>::dispatch_as_signer(
			frame_system::RawOrigin::Root.into(),
			Box::new(nested),
		)
		.expect_err("Root must not be allowed");
		assert_eq!(err.error, DispatchError::BadOrigin);
	});
}

#[test]
fn dispatch_as_signer_succeeds() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1); // to have events.

		let who: u64 = 100;

		// Make the account a lite person (minimal state required by the tx extension).
		LitePeople::<Test>::insert(
			who,
			crate::pallet::LitePersonInfo { ring_vrf_key: [1u8; 32], special_key: [0u8; 32] },
		);

		// Nested call will be executed as Signed(who).
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"hello".to_vec(),
		});

		let outer = RuntimeCall::PeopleLite(crate::Call::<Test>::dispatch_as_signer {
			call: Box::new(nested),
		});

		frame_system::Pallet::<Test>::inc_sufficients(&who);

		// First use of this account -> nonce 0 for AsLitePerson.
		assert_ok!(exec_as_lite_person_tx(who, outer, 0));

		// Verify the System::Remarked event contains `sender = who`.
		let found = frame_system::Pallet::<Test>::events().iter().any(|rec| {
			matches!(
				rec.event,
				crate::mock::RuntimeEvent::System(frame_system::Event::Remarked { ref sender, .. })
				if *sender == who
			)
		});
		assert!(found, "expected System::Remarked event from the lite person signer");
	});
}

#[test]
fn full_flow_manager_attester_user_register_and_dispatch() {
	new_test_ext().execute_with(|| {
		frame_system::Pallet::<Test>::set_block_number(1); // to have events.

		let attester: u64 = 210;
		let user: u64 = 211;
		let attestation: u64 = 12345;

		// 1) Manager increases allowance for attester.
		assert_ok!(Pallet::<Test>::increase_attestation_allowance(
			frame_system::RawOrigin::Root.into(),
			attester,
			1,
		));
		assert_eq!(AttestationAllowance::<Test>::get(attester), 1);

		// 2) Attester sets attestation.
		let set_call =
			RuntimeCall::PeopleLite(crate::Call::<Test>::set_attestation { attestation });
		assert_ok!(exec_signed_tx(attester, set_call));
		assert!(UnclaimedAttestations::<Test>::contains_key(attester, attestation));
		assert_eq!(AttestationAllowance::<Test>::get(attester), 0);
		assert!(!AttestationAllowance::<Test>::contains_key(attester));

		// 3) User registers via the tx extension.
		let secret = secret_from_seed(11);
		let ring_vrf_key: crate::MemberOf<Test> = member_from_secret(&secret);
		let special_key = [9u8; 32];
		let proof =
			<Test as crate::Config>::Crypto::sign(&secret, &poo_msg(user)[..]).expect("sign ok");
		let attestation_signature: <Test as crate::Config>::AttestationSignature =
			sp_runtime::testing::UintAuthorityId(attestation);

		let register_call = RuntimeCall::PeopleLite(crate::Call::<Test>::register {
			ring_vrf_key,
			special_key,
			proof_of_ownership: proof,
			verifier: attester,
			attestation,
			attestation_signature,
		});
		// First tx from `user` -> nonce 0 for registration.
		assert_ok!(exec_as_lite_person_registration_tx(user, register_call, 0));

		assert!(LitePeople::<Test>::contains_key(user));
		assert!(!UnclaimedAttestations::<Test>::contains_key(attester, attestation));

		// 4) User dispatches as signer (now a lite person) - second tx -> nonce 1.
		let nested = RuntimeCall::System(frame_system::Call::<Test>::remark_with_event {
			remark: b"full-flow-ok".to_vec(),
		});
		let outer = RuntimeCall::PeopleLite(crate::Call::<Test>::dispatch_as_signer {
			call: Box::new(nested),
		});
		assert_ok!(exec_as_lite_person_tx(user, outer, 1));

		// Confirm the remark event came from the user.
		let found = frame_system::Pallet::<Test>::events().iter().any(|rec| {
			matches!(
				rec.event,
				crate::mock::RuntimeEvent::System(frame_system::Event::Remarked { ref sender, .. })
				if *sender == user
			)
		});
		assert!(found, "expected System::Remarked event from the registered lite person");
	});
}
