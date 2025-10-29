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

#![allow(unused)]

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::cell::RefCell;
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
use indiv_pallet_people::RevisedContextualAlias;
use indiv_support::traits::{ContextualAlias, RingIndex};
use scale_info::TypeInfo;
use sp_runtime::{
	testing::H256,
	traits::{Applyable, BlakeTwo256, Checkable, ConstU32, ConstU64, ConstUint, IdentityLookup},
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

/// Helper function to create a bounded vec username
pub fn username<T: Config>(s: &[u8]) -> Username {
	s.to_vec().try_into().unwrap()
}

/// Helper to create a communication identifier
pub fn comm_id(s: &[u8]) -> CommunicationIdentifier {
	let mut buf = Vec::new();
	while buf.len() < 65 {
		buf.extend_from_slice(s);
	}
	if buf.len() > 65 {
		buf.truncate(65);
	}
	buf.try_into().unwrap()
}

/// Helper to create a valid signature for a lite identity proof
pub fn mock_lite_proof(lite_account: AccountId32) -> AccountAuthority {
	AccountAuthority(lite_account)
}

/// Helper to mock the LitePerson origin
pub fn lite_person_origin(account: u64) -> RuntimeOrigin {
	RuntimeOrigin::from(OriginCaller::PeopleLite(pallet_people_lite::Origin::LitePerson(
		id_to_account(account),
	)))
}

/// Helper to mock the Person origin
pub fn person_origin_for(alias_id: u64, ring: RingIndex, revision: u32) -> RuntimeOrigin {
	let alias = id_to_alias(alias_id);
	let contextual_alias = ContextualAlias { context: RESOURCES_CONTEXT, alias };
	let revised_alias = RevisedContextualAlias { ca: contextual_alias, ring, revision };
	RuntimeOrigin::from(OriginCaller::People(indiv_pallet_people::Origin::PersonalAlias(
		revised_alias,
	)))
}

/// Helper to advance time (seconds)
pub fn advance_time_sec(secs: u64) {
	let current_time = TestClock::now().as_secs();
	TestClock::set_time(Duration::from_secs(current_time + secs));
}

/// Helper to set current time (seconds)
pub fn set_time_sec(secs: u64) {
	TestClock::set_time(Duration::from_secs(secs));
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
		People: indiv_pallet_people,
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstUint<42>;
	type OnSetCode = ();
}

thread_local! {
	pub static MOCK_UNIX_TIME: RefCell<Duration> = RefCell::new(Default::default());
}

pub struct TestClock;

impl UnixTime for TestClock {
	fn now() -> Duration {
		MOCK_UNIX_TIME.with(|mock| *mock.borrow())
	}
}

impl TestClock {
	fn set_time(now: Duration) {
		MOCK_UNIX_TIME.with(|mock| *mock.borrow_mut() = now);
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
			OriginCaller::People(indiv_pallet_people::Origin::PersonalAlias(contextual_alias)) =>
				Ok(contextual_alias.ca.alias),
			_ => Err(origin),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_context: &Context) -> Result<RuntimeOrigin, ()> {
		unimplemented!()
	}
}

impl indiv_support::traits::CountedMembers for MockPerson {
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
	type LiteConsumerRegistrar = Resources;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = Helper;
}

impl indiv_pallet_people::Config for Test {
	type WeightInfo = ();
	type Crypto = Simple;
	type AccountContexts = ();
	type ChunkPageSize = ConstU32<8>;
	type MaxRingSize = ConstU32<255>;
	type OnboardingQueuePageSize = ConstU32<512>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub LitePersonStatementLimit: ValidStatement = ValidStatement {
		max_size: 4 * 1024, // 4 KiB
		max_count: 10,
	};
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl benchmarking::BenchmarkHelper<Test> for BenchmarkHelper {
	fn set_time(now: Duration) {
		MOCK_UNIX_TIME.with(|mock| *mock.borrow_mut() = now);
	}

	fn sign_message(message: &[u8]) -> (AccountId32, AccountAuthority) {
		let account = AccountId32::new([42u8; 32]);
		let signature = AccountAuthority(account.clone());
		(account, signature)
	}
}

impl Config for Test {
	type WeightInfo = ();
	type Crypto = Simple;
	type MaxUsernameLength = ConstU32<32>;
	type MinUsernameLength = ConstU32<7>;
	type PersonAuthDuration = ConstU32<20>;
	type MinPersonAuthUpdateInterval = ConstU32<10>;
	type EnsurePerson = indiv_pallet_people::EnsurePersonalAliasInContext<Test>;
	type EnsureLitePerson = pallet_people_lite::EnsureLitePerson<Test>;
	type Clock = TestClock;
	type OffchainSignature = AccountAuthority;
	type UsernameReservationDuration = ConstU64<40>;
	type LitePersonStatementLimit = LitePersonStatementLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let c = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
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
