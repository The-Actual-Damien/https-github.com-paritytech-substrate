// Copyright 2017 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Support functions.

#[macro_use]
extern crate environmental;
extern crate polkadot_state_machine;
extern crate polkadot_primitives as primitives;

use std::fmt;
use primitives::ed25519;

pub use std::vec::Vec;
pub use std::rc::Rc;
pub use std::cell::RefCell;
pub use std::boxed::Box;
pub use std::slice;
pub use std::mem::{size_of, transmute, swap, uninitialized};

pub use polkadot_state_machine::Externalities;

// TODO: use the real error, not NoError.

#[derive(Debug)]
pub struct NoError;
impl fmt::Display for NoError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "") }
}

environmental!(ext : Externalities<Error=NoError> + 'static);

pub fn storage(key: &[u8]) -> Vec<u8> {
	ext::with(|ext| ext.storage(key).ok().map(|s| s.to_vec()))
		.unwrap_or(None)
		.unwrap_or_else(|| vec![])
}

pub fn read_storage(key: &[u8], value_out: &mut [u8]) -> usize {
	ext::with(|ext| {
		if let Ok(value) = ext.storage(key) {
			let written = ::std::cmp::min(value.len(), value_out.len());
			value_out[0..written].copy_from_slice(&value[0..written]);
			value.len()
		} else {
			0
		}
	}).unwrap_or(0)
}

pub fn storage_into<T: Sized>(_key: &[u8]) -> Option<T> {
	let size = size_of::<T>();

	ext::with(|ext| {
		if let Ok(value) = ext.storage(_key) {
			if value.len() == size {
				unsafe {
					let mut result: T = std::mem::uninitialized();
					std::slice::from_raw_parts_mut(transmute::<*mut T, *mut u8>(&mut result), size)
						.copy_from_slice(&value);
					return Some(result);
				}
			}
		}
		None
	}).unwrap_or(None)
}

pub fn set_storage(key: &[u8], value: &[u8]) {
	ext::with(|ext|
		ext.set_storage(key.to_vec(), value.to_vec())
	);
}

/// The current relay chain identifier.
pub fn chain_id() -> u64 {
	ext::with(|ext|
		ext.chain_id()
	).unwrap_or(0)
}

/// Conduct a Keccak-256 hash of the given data.
pub use primitives::{blake2_256, twox_128, twox_256};

/// Verify a ed25519 signature.
pub fn ed25519_verify(sig: &[u8; 64], msg: &[u8], pubkey: &[u8; 32]) -> bool {
	ed25519::verify(&sig[..], msg, &pubkey[..])
}

/// Execute the given closure with global function available whose functionality routes into the
/// externalities `ext`. Forwards the value that the closure returns.
pub fn with_externalities<R, F: FnOnce() -> R>(ext: &mut (Externalities<Error=NoError> + 'static), f: F) -> R {
	ext::using(ext, f)
}

#[macro_export]
macro_rules! impl_stubs {
	($( $name:ident ),*) => {}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[derive(Debug, Default)]
	struct TestExternalities {
		storage: HashMap<Vec<u8>, Vec<u8>>,
	}
	impl Externalities for TestExternalities {
		type Error = NoError;

		fn storage(&self, key: &[u8]) -> Result<&[u8], NoError> {
			Ok(self.storage.get(&key.to_vec()).map_or(&[] as &[u8], Vec::as_slice))
		}

		fn set_storage(&mut self, key: Vec<u8>, value: Vec<u8>) {
			self.storage.insert(key, value);
		}

		fn chain_id(&self) -> u64 { 42 }
	}

	macro_rules! map {
		($( $name:expr => $value:expr ),*) => (
			vec![ $( ( $name, $value ) ),* ].into_iter().collect()
		)
	}

	#[test]
	fn storage_works() {
		let mut t = TestExternalities { storage: map![], };
		assert!(with_externalities(&mut t, || {
			assert_eq!(storage(b"hello"), b"".to_vec());
			set_storage(b"hello", b"world");
			assert_eq!(storage(b"hello"), b"world".to_vec());
			assert_eq!(storage(b"foo"), b"".to_vec());
			set_storage(b"foo", &[1, 2, 3][..]);
			assert_eq!(storage_into::<[u8; 3]>(b"foo"), Some([1, 2, 3]));
			assert_eq!(storage_into::<[u8; 3]>(b"hello"), None);
			true
		}));

		t.storage = map![b"foo".to_vec() => b"bar".to_vec()];

		assert!(!with_externalities(&mut t, || {
			assert_eq!(storage(b"hello"), b"".to_vec());
			assert_eq!(storage(b"foo"), b"bar".to_vec());
			false
		}));
	}
}
