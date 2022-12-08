use codec::{Decode, Encode};
use frame_support::ensure;
use sp_core::crypto::AccountId32;
use sp_runtime::DispatchError;
pub use sp_std::{vec::{Vec},vec,str};
use scale_info::prelude::string::String;
use crate::log;

// This function converts a 32 byte AccountId to its byte-array equivalent form.
pub fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], DispatchError>
	where
		AccountId: Encode+?Sized,
{
	let account_vec = account.encode();
	ensure!(account_vec.len() == 32, "AccountId must be 32 bytes.");
	let mut bytes = [0u8; 32];
	bytes.copy_from_slice(&account_vec);
	Ok(bytes)
}

pub fn hex_string_to_vec(str:String) -> Vec<u8> {
	let hex_string =str.replace("0x", "");
	let split_string = hex_string.as_bytes()
		.chunks(2)
		.map(str::from_utf8)
		.collect::<Result<Vec<&str>, _>>()
		.unwrap();
	let mut bytes :Vec<u8> = Vec::new();
	for part in split_string.into_iter() {
		bytes.push(hex_to_deci(part));
	}
	bytes
}

fn hex_to_deci(str:&str) -> u8 {
	let mut deci: u8 = 0;
	let mut i: u32 = 0;
	let hex_vec: Vec<char> = str.trim_end().chars().rev().collect();

	for hex in hex_vec {
		let temp: u8 = match hex {
			'0'=>0,
			'1'=>1,
			'2'=>2,
			'3'=>3,
			'4'=>4,
			'5'=>5,
			'6'=>6,
			'7'=>7,
			'8'=>8,
			'9'=>9,
			'a'=>10,
			'b'=>11,
			'c'=>12,
			'd'=>13,
			'e'=>14,
			'f'=>15,
			_=> 16
		};
		if temp == 16 {
			return u8::MAX;
		}
		deci += temp * u8::pow(16, i);
		i += 1;
	}

	return deci;
}

// pub fn convert_bytes_to_hex(bytes: [u8;32])-> String{
// 	let to_address = convert_bytes_to_accountid(bytes);
// 	let mut res = String::new();
// 	write!(&mut res, "{:?}",to_address);
// 	res
// }

pub fn convert_bytes_to_accountid<AccountId>(bytes: [u8;32])-> AccountId
	where AccountId:Encode+?Sized+Decode
{
	let account32: AccountId32 = bytes.into();
	let mut to32:&[u8] = AccountId32::as_ref(&account32);
	let to_address = AccountId::decode(&mut to32).unwrap();
	to_address
}

pub fn convert_string_to_accountid<AccountId>(account_str: &str)-> AccountId
	where AccountId:Encode+?Sized+Decode
{
	let mut output = vec![0xFF; 35];
	bs58::decode(account_str).into(&mut output).unwrap();
	let cut_address_vec:Vec<u8> = output.drain(1..33).collect();
	let mut array = [0; 32];
	let bytes = &cut_address_vec[..array.len()];
	array.copy_from_slice(bytes);
	let account32: AccountId32 = array.into();
	let mut to32 = AccountId32::as_ref(&account32);
	let to_address = AccountId::decode(&mut to32).unwrap();
	to_address
}
