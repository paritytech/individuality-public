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

#![allow(unused)]

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl,
	dispatch::{DispatchErrorWithPostInfo, GetDispatchInfo},
	parameter_types,
	storage::with_transaction,
	traits::OriginTrait,
};
use frame_system::{
	offchain::{CreateBare, CreateTransactionBase},
	EnsureRoot,
};
use scale_info::TypeInfo;
use sp_runtime::{
	testing::H256,
	traits::{Applyable, BlakeTwo256, Checkable, ConstU32, ConstUint, IdentityLookup},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
	AccountId32, BuildStorage, DispatchError, TransactionOutcome,
};
use verifiable::demo_impls::Simple;

pub type Header = sp_runtime::generic::Header<u64, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, Extrinsic>;
pub type Extrinsic =
	sp_runtime::generic::UncheckedExtrinsic<AccountId32, RuntimeCall, AccountAuthority, ()>;

impl<LocalCall> CreateBare<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> Extrinsic {
		Extrinsic::new_bare(call)
	}
}

impl<LocalCall> CreateTransactionBase<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type RuntimeCall = RuntimeCall;
}

/// Convert a `u64` to an `AccountId32`.
pub fn id_to_account(id: u64) -> AccountId32 {
	let mut bytes = [0; 32];
	bytes[..8].copy_from_slice(&id.to_le_bytes());
	AccountId32::new(bytes)
}

/// Convert a `u64` to an `Alias`.
pub fn id_to_alias(id: u64) -> Alias {
	let mut bytes = [0; 32];
	bytes[..8].copy_from_slice(&id.to_le_bytes());
	bytes
}

/// A signature type that is always successful for a given account
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	Hash,
	PartialOrd,
	Ord,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct AccountAuthority(pub AccountId32);

impl IdentifyAccount for AccountAuthority {
	type AccountId = AccountId32;
	fn into_account(self) -> Self::AccountId {
		self.0
	}
}

impl Verify for AccountAuthority {
	type Signer = Self;

	fn verify<L: sp_runtime::traits::Lazy<[u8]>>(
		&self,
		_msg: L,
		signer: &<Self::Signer as IdentifyAccount>::AccountId,
	) -> bool {
		self.0 == *signer
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Resources: crate,
		Balances: pallet_balances,
		People: pallet_people_multi,
		PeopleLite: pallet_people_lite,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstUint<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstUint<42>;
	type OnSetCode = ();
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub static Now: core::time::Duration = core::time::Duration::from_millis(0);
}

pub struct TestClock;
impl UnixTime for TestClock {
	fn now() -> core::time::Duration {
		Now::get()
	}
}

pub struct MockPerson;
impl frame_support::traits::EnsureOriginWithArg<RuntimeOrigin, Context> for MockPerson {
	type Success = Alias;

	fn try_origin(
		origin: RuntimeOrigin,
		_context: &Context,
	) -> Result<Self::Success, RuntimeOrigin> {
		match origin.caller() {
			OriginCaller::People(pallet_people_multi::Origin::PersonalAlias(contextual_alias)) =>
				Ok(contextual_alias.ca.alias),
			_ => Err(origin),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_context: &Context) -> Result<RuntimeOrigin, ()> {
		unimplemented!()
	}
}

impl individuality_support::traits::CountedMembers for MockPerson {
	fn active_count() -> u32 {
		0
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_active_count(_count: u32) {}
}

pub struct MockLitePerson;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for MockLitePerson {
	type Success = AccountId32;

	fn try_origin(origin: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match origin.caller() {
			OriginCaller::PeopleLite(pallet_people_lite::Origin::LitePerson(account)) =>
				Ok(account.clone()),
			_ => Err(origin),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		unimplemented!()
	}
}

parameter_types! {
	pub const MaxUsernameLength: u32 = 32;
	pub const MinUsernameLength: u32 = 7;
	pub const PersonAuthDuration: u32 = 30 * 24 * 60 * 60; // 40 days
	pub const MinPersonAuthUpdateInterval: u32 = 5 * 24 * 60 * 60; // 5 days
	pub const UsernameReservationDuration: u64 = 20 * 60; // 20 minutes
	pub LitePersonStatementLimit: ValidStatement = ValidStatement {
		max_size: 4 * 1024, // 4 KiB
		max_count: 10,
	};
}

impl crate::Config for Test {
	type WeightInfo = ();
	type Crypto = Simple;
	type MaxUsernameLength = MaxUsernameLength;
	type MinUsernameLength = MinUsernameLength;
	type PersonAuthDuration = PersonAuthDuration;
	type MinPersonAuthUpdateInterval = MinPersonAuthUpdateInterval;
	type EnsurePerson = MockPerson;
	type EnsureLitePerson = MockLitePerson;
	type Clock = TestClock;
	type OffchainSignature = AccountAuthority;
	type UsernameReservationDuration = UsernameReservationDuration;
	type LitePersonStatementLimit = LitePersonStatementLimit;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct Helper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_people_lite::BenchmarkHelper<AccountId32, AccountAuthority> for Helper {
	fn sign_message(_message: &[u8]) -> (AccountId32, AccountAuthority) {
		([0u8; 32].into(), AccountAuthority([0u8; 32].into()))
	}
}

impl pallet_people_lite::Config for Test {
	type WeightInfo = ();
	type AttestationAllowanceManager = EnsureRoot<Self::AccountId>;
	type Crypto = Simple;
	type AttestationSignature = AccountAuthority;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = Helper;
}

impl pallet_people_multi::Config for Test {
	type WeightInfo = ();
	type Crypto = Simple;
	type AccountContexts = ();
	type ChunkPageSize = ConstU32<8>;
	type MaxRingSize = ConstU32<255>;
	type OnboardingQueuePageSize = ConstU32<512>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let c = RuntimeGenesisConfig::default().build_storage().unwrap();
	sp_io::TestExternalities::from(c)
}

/// We gather both error into a single type in order to do `assert_ok` and `assert_err` safely.
/// Otherwise, we can easily miss the inner error in a `Resut<Resut<_, _>, _>`.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TransactionExecutionError {
	Validity(TransactionValidityError),
	// This ignores the post info.
	Dispatch(DispatchErrorWithPostInfo),
}

impl TransactionExecutionError {
	#[allow(unused)]
	pub fn unwrap_dispatch(self) -> DispatchErrorWithPostInfo {
		let Self::Dispatch(error) = self else {
			panic!("validity error unwrapped as dispatch");
		};
		error
	}
}

impl From<DispatchErrorWithPostInfo> for TransactionExecutionError {
	fn from(e: DispatchErrorWithPostInfo) -> Self {
		TransactionExecutionError::Dispatch(e)
	}
}

impl From<TransactionValidityError> for TransactionExecutionError {
	fn from(e: TransactionValidityError) -> Self {
		TransactionExecutionError::Validity(e)
	}
}

impl From<DispatchError> for TransactionExecutionError {
	fn from(e: DispatchError) -> Self {
		TransactionExecutionError::Dispatch(e.into())
	}
}

impl From<InvalidTransaction> for TransactionExecutionError {
	fn from(e: InvalidTransaction) -> Self {
		TransactionExecutionError::Validity(e.into())
	}
}

/// Execute a bare extrinsic with the given call.
pub fn exec_tx(x: Extrinsic) -> Result<(), TransactionExecutionError> {
	let info = x.get_dispatch_info();
	let len = x.encoded_size();

	let checked = Checkable::check(x, &frame_system::ChainContext::<Test>::default())?;

	// validation is always rollbacked in production.
	with_transaction(|| {
		let valid = checked.validate::<Test>(TransactionSource::External, &info, len);

		TransactionOutcome::Rollback(Result::<_, DispatchError>::Ok(valid))
	})
	.unwrap()?;

	checked.apply::<Test>(&info, len)??;

	Ok(())
}
