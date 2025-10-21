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

use crate::*;
use codec::Encode;
use frame_support::{
	derive_impl,
	dispatch::{DispatchErrorWithPostInfo, GetDispatchInfo},
	storage::with_transaction,
};
use frame_system::EnsureRoot;
use sp_runtime::{
	testing::UintAuthorityId,
	traits::{Applyable, Checkable},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
	BuildStorage, DispatchError, TransactionOutcome,
};
use verifiable::demo_impls::Simple;

pub type Header = sp_runtime::generic::Header<u64, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, Extrinsic>;
pub type Extrinsic = sp_runtime::generic::UncheckedExtrinsic<
	u64,
	RuntimeCall,
	UintAuthorityId,
	PeopleLiteAuth<Test>,
>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		PeopleLite: crate,
		Balances: pallet_balances,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

impl crate::Config for Test {
	type WeightInfo = ();
	type AttestationAllowanceManager = EnsureRoot<Self::AccountId>;
	type Crypto = Simple;
	type AttestationSignature = UintAuthorityId;
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

/// Execute a signed extrinsic with the given call.
#[allow(unused)]
pub fn exec_signed_tx(
	account: u64,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	let x = Extrinsic::new_signed(
		call.into(),
		account,
		UintAuthorityId(account),
		PeopleLiteAuth::<Test>::new(None),
	);

	exec_tx(x)
}

#[allow(unused)]
pub fn exec_as_lite_person_registration_tx(
	account: u64,
	call: RuntimeCall,
	nonce: u32,
) -> Result<(), TransactionExecutionError> {
	let x = Extrinsic::new_signed(
		call,
		account,
		UintAuthorityId(account),
		PeopleLiteAuth::<Test>::new(Some(crate::PeopleLiteAuthData::AsLitePersonRegistration {
			nonce,
		})),
	);
	exec_tx(x)
}

#[allow(unused)]
pub fn exec_as_lite_person_tx(
	account: u64,
	call: RuntimeCall,
	nonce: u32,
) -> Result<(), TransactionExecutionError> {
	let x = Extrinsic::new_signed(
		call,
		account,
		UintAuthorityId(account),
		PeopleLiteAuth::<Test>::new(Some(crate::PeopleLiteAuthData::AsLitePerson { nonce })),
	);
	exec_tx(x)
}
