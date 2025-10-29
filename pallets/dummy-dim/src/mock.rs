// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Individuality.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.

// Individuality is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Individuality.  If not, see <http://www.gnu.org/licenses/>.

use frame_support::derive_impl;
use frame_system::{
	offchain::{CreateBare, CreateTransactionBase},
	pallet_prelude::ExtrinsicFor,
	EnsureRoot,
};
use sp_core::{ConstU16, ConstU32, ConstU64, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		People: indiv_pallet_people,
		DummyDim: crate
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
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl indiv_pallet_people::Config for Test {
	type WeightInfo = ();
	type Crypto = verifiable::demo_impls::Simple;
	type AccountContexts = ();
	type ChunkPageSize = ConstU32<8>;
	type MaxRingSize = ConstU32<255>;
	type OnboardingQueuePageSize = ConstU32<512>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl crate::Config for Test {
	type WeightInfo = ();
	type UpdateOrigin = EnsureRoot<Self::AccountId>;
	type MaxPersonBatchSize = ConstU32<1000>;
	type People = People;
}

#[allow(dead_code)]
pub fn advance_to(b: u64) {
	while System::block_number() < b {
		System::set_block_number(System::block_number() + 1);
	}
}

impl CreateBare<indiv_pallet_people::Call<Self>> for Test {
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		Self::Extrinsic::new_bare(call)
	}
}

impl CreateTransactionBase<indiv_pallet_people::Call<Self>> for Test {
	type Extrinsic = ExtrinsicFor<Test>;
	type RuntimeCall = RuntimeCall;
}

pub struct ConfigRecord;

pub fn new_config() -> ConfigRecord {
	ConfigRecord
}

pub struct TestExt(ConfigRecord);
#[allow(dead_code)]
impl TestExt {
	pub fn new() -> Self {
		Self(new_config())
	}

	pub fn execute_with<R>(self, f: impl Fn() -> R) -> R {
		new_test_ext().execute_with(f)
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let c = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	sp_io::TestExternalities::from(c)
}
