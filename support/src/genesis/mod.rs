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

use codec::Encode;
use sp_runtime::Vec;
use verifiable::ring_vrf_impl::StaticChunk;

/// Helper function to get ring verifier builder params.
pub fn ring_verifier_builder_params() -> Vec<StaticChunk> {
	let params = verifiable::ring_vrf_impl::ring_verifier_builder_params();
	let chunks: Vec<StaticChunk> = params.0.iter().map(|c| StaticChunk(*c)).collect();
	chunks
}

/// Helper function to get raw ring verifier builder params.
pub fn ring_verifier_builder_params_raw() -> Vec<u8> {
	ring_verifier_builder_params().encode()
}
