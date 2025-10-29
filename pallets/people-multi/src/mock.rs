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

use crate::{
	extension::{AsPerson, AsPersonInfo},
	*,
};
use frame_support::{
	assert_ok, derive_impl, dispatch::DispatchErrorWithPostInfo, match_types, parameter_types,
	storage::with_transaction, weights::RuntimeDbWeight,
};
use frame_system::{offchain::CreateTransactionBase, ChainContext};
use sp_core::{ConstU16, ConstU32, ConstU64, H256};
use sp_runtime::{
	testing::UintAuthorityId,
	traits::{Applyable, BlakeTwo256, Checkable, IdentityLookup},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
	BuildStorage, DispatchError, Weight,
};
use verifiable::{demo_impls::Simple, GenerateVerifiable};

// First ring, used in testing.
pub const RI_ZERO: RingIndex = 0;

const EXTENSION_VERSION: u8 = 0;
pub type TransactionExtension = (AsPerson<Test>, frame_system::CheckNonce<Test>);
pub type Header = sp_runtime::generic::Header<u64, sp_runtime::traits::BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<
	u64,
	RuntimeCall,
	sp_runtime::testing::UintAuthorityId,
	TransactionExtension,
>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		PeoplePallet: crate,
	}
);

parameter_types! {
	pub const MockDbWeight: RuntimeDbWeight = RuntimeDbWeight {
		read: 10,
		write: 20,
	};
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = MockDbWeight;
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

pub type Extrinsic = sp_runtime::testing::TestXt<RuntimeCall, ()>;

impl CreateTransactionBase<Call<Self>> for Test {
	type Extrinsic = Extrinsic;
	type RuntimeCall = RuntimeCall;
}

parameter_types! {
	pub static MaxRingSize: u32 = 10;
}

pub const MOCK_CONTEXT: Context = *b"pop:polkadot.network/mock       ";
match_types! {
	pub type TestAccountContexts: impl Contains<Context> = {
		&MOCK_CONTEXT
	};
}

pub struct MockWeights;
impl crate::WeightInfo for MockWeights {
	fn under_alias() -> sp_runtime::Weight {
		Weight::from_parts(3, 3)
	}

	fn set_alias_account() -> sp_runtime::Weight {
		Weight::from_parts(4, 4)
	}

	fn unset_alias_account() -> sp_runtime::Weight {
		Weight::from_parts(5, 5)
	}

	fn force_recognize_personhood(_n: u32) -> sp_runtime::Weight {
		Weight::from_parts(7, 7)
	}

	fn set_personal_id_account() -> sp_runtime::Weight {
		Weight::from_parts(8, 8)
	}

	fn unset_personal_id_account() -> sp_runtime::Weight {
		Weight::from_parts(9, 9)
	}

	fn set_onboarding_size() -> sp_runtime::Weight {
		Weight::from_parts(10, 10)
	}

	fn merge_rings() -> sp_runtime::Weight {
		Weight::from_parts(11, 11)
	}

	fn migrate_included_key() -> sp_runtime::Weight {
		Weight::from_parts(12, 12)
	}

	fn migrate_onboarding_key() -> sp_runtime::Weight {
		Weight::from_parts(13, 13)
	}

	fn should_build_ring(n: u32) -> sp_runtime::Weight {
		Weight::from_parts(n as u64 * 14, n as u64 * 14)
	}

	fn build_ring(n: u32) -> sp_runtime::Weight {
		Weight::from_parts(n as u64 * 14, n as u64 * 14)
	}

	fn onboard_people() -> sp_runtime::Weight {
		Weight::from_parts(15, 15)
	}

	fn remove_suspended_keys(n: u32) -> sp_runtime::Weight {
		Weight::from_parts(n as u64 * 16, n as u64 * 16)
	}

	fn pending_suspensions_iteration() -> Weight {
		Weight::from_parts(1, 1)
	}

	fn migrate_keys_single_included_key() -> sp_runtime::Weight {
		Weight::from_parts(17, 17)
	}

	fn merge_queue_pages() -> sp_runtime::Weight {
		Weight::from_parts(18, 18)
	}

	fn on_poll_base() -> sp_runtime::Weight {
		Weight::from_parts(19, 19)
	}

	fn on_idle_base() -> sp_runtime::Weight {
		Weight::from_parts(20, 20)
	}

	fn as_person_alias_with_account() -> Weight {
		Weight::from_parts(20, 20)
	}

	fn as_person_identity_with_account() -> Weight {
		Weight::from_parts(21, 21)
	}

	fn as_person_alias_with_proof() -> Weight {
		Weight::from_parts(22, 22)
	}

	fn as_person_identity_with_proof() -> Weight {
		Weight::from_parts(23, 23)
	}

	fn as_person_alias_with_account_revised() -> Weight {
		Weight::from_parts(24, 24)
	}
}

pub const INVALID_MEMBER: [u8; 32] = [
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
	27, 28, 29, 30, 31, 32,
];

/// A mock crypto implementation that implements same as `Simple` but `INVALID_MEMBER` is not a
/// valid member.
pub struct MockCrypto;

impl GenerateVerifiable for MockCrypto {
	type Proof = <Simple as GenerateVerifiable>::Proof;
	type Member = <Simple as GenerateVerifiable>::Member;
	type Secret = <Simple as GenerateVerifiable>::Secret;
	type Members = <Simple as GenerateVerifiable>::Members;
	type Signature = <Simple as GenerateVerifiable>::Signature;
	type Commitment = <Simple as GenerateVerifiable>::Commitment;
	type StaticChunk = <Simple as GenerateVerifiable>::StaticChunk;
	type Intermediate = <Simple as GenerateVerifiable>::Intermediate;
	fn open(
		member: &Self::Member,
		members_iter: impl Iterator<Item = Self::Member>,
	) -> Result<Self::Commitment, ()> {
		Simple::open(member, members_iter)
	}
	fn sign(secret: &Self::Secret, message: &[u8]) -> Result<Self::Signature, ()> {
		Simple::sign(secret, message)
	}
	fn create(
		commitment: Self::Commitment,
		secret: &Self::Secret,
		context: &[u8],
		message: &[u8],
	) -> Result<(Self::Proof, Alias), ()> {
		Simple::create(commitment, secret, context, message)
	}
	fn is_valid(
		proof: &Self::Proof,
		members: &Self::Members,
		context: &[u8],
		alias: &Alias,
		message: &[u8],
	) -> bool {
		Simple::is_valid(proof, members, context, alias, message)
	}
	fn validate(
		proof: &Self::Proof,
		members: &Self::Members,
		context: &[u8],
		message: &[u8],
	) -> Result<Alias, ()> {
		Simple::validate(proof, members, context, message)
	}
	fn new_secret(entropy: verifiable::Entropy) -> Self::Secret {
		Simple::new_secret(entropy)
	}
	fn push_members(
		intermediate: &mut Self::Intermediate,
		members: impl Iterator<Item = Self::Member>,
		lookup: impl Fn(Range<usize>) -> Result<Vec<Self::StaticChunk>, ()>,
	) -> Result<(), ()> {
		Simple::push_members(intermediate, members, lookup)
	}
	fn start_members() -> Self::Intermediate {
		Simple::start_members()
	}
	fn finish_members(inter: Self::Intermediate) -> Self::Members {
		Simple::finish_members(inter)
	}
	fn is_member_valid(member: &Self::Member) -> bool {
		if *member == INVALID_MEMBER {
			return false;
		}

		Simple::is_member_valid(member)
	}
	fn alias_in_context(secret: &Self::Secret, context: &[u8]) -> Result<Alias, ()> {
		Simple::alias_in_context(secret, context)
	}
	fn verify_signature(
		signature: &Self::Signature,
		message: &[u8],
		member: &Self::Member,
	) -> bool {
		Simple::verify_signature(signature, message, member)
	}
	fn member_from_secret(secret: &Self::Secret) -> Self::Member {
		Simple::member_from_secret(secret)
	}
}

impl crate::Config for Test {
	type WeightInfo = MockWeights;
	type Crypto = MockCrypto;
	type AccountContexts = TestAccountContexts;
	type ChunkPageSize = ConstU32<5>;
	type MaxRingSize = MaxRingSize;
	type OnboardingQueuePageSize = ConstU32<40>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchHelper {}

#[cfg(feature = "runtime-benchmarks")]
impl<Chunk> BenchmarkHelper<Chunk> for BenchHelper
where
	Chunk: From<<MockCrypto as verifiable::GenerateVerifiable>::StaticChunk>,
{
	fn valid_account_context() -> Context {
		MOCK_CONTEXT
	}
	fn initialize_chunks() -> Vec<Chunk> {
		vec![]
	}
}

#[allow(dead_code)]
pub fn advance_to(b: u64) {
	while System::block_number() < b {
		System::set_block_number(System::block_number() + 1);
	}
}

pub struct ConfigRecord;

pub fn new_config() -> ConfigRecord {
	ConfigRecord
}

pub struct TestExt(ConfigRecord);
#[allow(dead_code)]
impl TestExt {
	pub(crate) fn max_ring_size(self, size: u32) -> Self {
		MaxRingSize::set(size);
		self
	}

	pub fn new() -> Self {
		Self(new_config())
	}

	pub fn execute_with<R>(self, f: impl Fn() -> R) -> R {
		new_test_ext().execute_with(f)
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let chunks: Vec<<MockCrypto as GenerateVerifiable>::StaticChunk> = [(); 512].to_vec();
	let encoded_chunks = chunks.encode();

	RuntimeGenesisConfig {
		system: Default::default(),
		people_pallet: crate::GenesisConfig::<Test> {
			encoded_chunks: encoded_chunks.clone(),
			..Default::default()
		},
	}
	.build_storage()
	.unwrap()
	.into()
}

/// We gather both error into a single type in order to do `assert_ok` and `assert_err` safely.
/// Otherwise, we can easily miss the inner error in a `Resut<Resut<_, _>, _>`.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TransactionExecutionError {
	Validity(TransactionValidityError),
	// This ignores the post info.
	Dispatch(DispatchErrorWithPostInfo),
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

/// Execute a transaction with the given origin, call and transaction extension.
pub fn exec_tx(
	who: Option<u64>,
	tx_ext: TransactionExtension,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	let tx = match who {
		Some(who) => UncheckedExtrinsic::new_signed(call.into(), who, UintAuthorityId(who), tx_ext),
		None => UncheckedExtrinsic::new_transaction(call.into(), tx_ext),
	};

	let info = tx.get_dispatch_info();
	let len = tx.encoded_size();

	// Check and validate the extrinsic.
	let checked = Checkable::check(tx, &ChainContext::<Test>::default())?;
	with_transaction(|| {
		let valid = checked.validate::<Test>(TransactionSource::External, &info, len);
		sp_runtime::TransactionOutcome::Rollback(Result::<_, DispatchError>::Ok(valid))
	})
	.unwrap()?;
	// Finally, apply the extrinsic.
	checked.apply::<Test>(&info, len)??;

	Ok(())
}

pub fn exec_as_alias_tx(
	who: u64,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	let nonce = frame_system::Account::<Test>::get(who).nonce;
	let tx_ext = (
		AsPerson::new(Some(AsPersonInfo::AsPersonalAliasWithAccount(nonce))),
		frame_system::CheckNonce::from(nonce),
	);

	exec_tx(Some(who), tx_ext, call)
}

/// Execute a transaction with the revised contextual alias origin with a revision update.
pub fn exec_as_alias_with_updated_revision_tx(
	who: u64,
	key: &<MockCrypto as GenerateVerifiable>::Member,
	secret: &<MockCrypto as GenerateVerifiable>::Secret,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	let nonce = frame_system::Account::<Test>::get(who).nonce;
	let id = crate::Keys::<Test>::get(key).expect("id not found");
	let rev_ca = crate::AccountToAlias::<Test>::get(who).expect("alias account not found");
	let record = crate::People::<Test>::get(id).expect("record not found");
	let ring_index = record.position.ring_index().unwrap();
	let commitment = {
		let all_keys = crate::RingKeys::<Test>::get(ring_index);
		MockCrypto::open(key, all_keys.into_iter()).unwrap()
	};
	let call: RuntimeCall = call.into();
	let other_tx_ext = (frame_system::CheckNonce::<Test>::from(0),);
	// Here we simply ignore implicit as they are null.
	let inherited_implication = (&EXTENSION_VERSION, &call, &other_tx_ext);
	let msg =
		(inherited_implication, "revise", &who, nonce).using_encoded(sp_io::hashing::blake2_256);
	let (proof, _alias) = MockCrypto::create(commitment, secret, &rev_ca.ca.context, &msg)
		.expect("proof creation failed");
	let tx_ext = (
		AsPerson::new(Some(AsPersonInfo::AsPersonalAliasWithAccountRevised(
			nonce,
			proof,
			ring_index,
			rev_ca.ca.context,
		))),
		frame_system::CheckNonce::from(0),
	);

	exec_tx(Some(who), tx_ext, call)
}

/// Call `set_alias_account` for the given personal id and account.
pub fn setup_alias_account(
	key: &<MockCrypto as GenerateVerifiable>::Member,
	secret: &<MockCrypto as GenerateVerifiable>::Secret,
	context: Context,
	account: u64,
) {
	let id = crate::Keys::<Test>::get(key).expect("id not found");
	let record = crate::People::<Test>::get(id).expect("record not found");
	let ring_index = record.position.ring_index().expect("person not included in a ring");
	let commitment = {
		let all_keys = crate::RingKeys::<Test>::get(ring_index);
		MockCrypto::open(key, all_keys.into_iter()).unwrap()
	};
	let call = RuntimeCall::PeoplePallet(crate::Call::set_alias_account {
		account,
		call_valid_at: frame_system::Pallet::<Test>::block_number(),
	});
	let other_tx_ext = (frame_system::CheckNonce::<Test>::from(0),);
	// Here we simply ignore implicit as they are null.
	let msg = (&EXTENSION_VERSION, &call, &other_tx_ext).using_encoded(sp_io::hashing::blake2_256);
	let (proof, _alias) =
		MockCrypto::create(commitment, secret, &context, &msg).expect("proof creation failed");
	let tx_ext = (
		AsPerson::<Test>::new(Some(AsPersonInfo::AsPersonalAliasWithProof(
			proof, ring_index, context,
		))),
		other_tx_ext.0,
	);
	assert_ok!(exec_tx(None, tx_ext.clone(), call.clone()));
}
