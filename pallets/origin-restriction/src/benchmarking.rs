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

//! Benchmarks for pallet origin restriction.

use super::*;
use frame_benchmarking::{v2::*, BenchmarkError};
use sp_runtime::traits::DispatchTransaction;

fn assert_last_event<T: Config>(generic_event: <T as frame_system::Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
mod benches {
	use super::*;

	#[benchmark]
	fn clean_usage() -> Result<(), BenchmarkError> {
		let origin = T::RestrictedEntity::benchmarked_restricted_origin();
		let entity = T::RestrictedEntity::restricted_entity(&origin)
			.expect("The origin from `benchmarked_restricted_origin` must be restricted");

		Usages::<T>::insert(&entity, Usage { used: 1u32.into(), at_block: 0u32.into() });

		frame_system::Pallet::<T>::set_block_number(1_000u32.into());

		#[extrinsic_call]
		_(frame_system::RawOrigin::Root, entity.clone());

		assert_last_event::<T>(Event::UsageCleaned { entity }.into());

		Ok(())
	}

	// This benchmark may miss the cost for `OperationAllowedOneTimeExcess::contains`.
	#[benchmark]
	fn restrict_origin_tx_ext() -> Result<(), BenchmarkError> {
		let tx_ext = RestrictOrigin::<T>::new(true);
		let origin = T::RestrictedEntity::benchmarked_restricted_origin();
		let call = frame_system::Call::remark { remark: alloc::vec![] }.into();

		#[block]
		{
			tx_ext
				.test_run(origin.into(), &call, &Default::default(), 0, 0, |_| {
					Ok(Default::default())
				})
				.expect("Failed to allow the cheapest call, benchmark needs to be improved")
				.expect("inner call successful");
		}

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
