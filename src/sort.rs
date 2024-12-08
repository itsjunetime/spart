use std::{cmp::Ordering, ops::Deref};

use merde::Value;

use crate::settings::Settings;

pub fn sort_arr(vec: &mut [merde::Map], settings: &Settings) {
	vec.sort_unstable_by(|a, b| {
		for key in &settings.x_axis {
			let key = &key.as_str().into();
			let a = &a[key];
			let b = &b[key];

			macro_rules! if_not_equal {
				($a:expr, $b:expr) => {
					match std::cmp::Ord::cmp($a, $b) {
						ord @ (Ordering::Less | Ordering::Greater) => return ord,
						Ordering::Equal => ()
					}
				};
			}

			match (a, b) {
				(Value::I64(a), Value::I64(b)) => if_not_equal!(&a, &b),
				(Value::U64(a), Value::U64(b)) => if_not_equal!(&a, &b),
				(Value::Float(a), Value::Float(b)) => if_not_equal!(&a, &b),
				(Value::Str(a), Value::Str(b)) => if_not_equal!(&a.deref(), &b.deref()),
				(Value::Bool(a), Value::Bool(b)) => if_not_equal!(&a, &b),
				(Value::Bytes(a), Value::Bytes(b)) => if_not_equal!(&a.deref(), &b.deref()),
				(Value::Null, Value::Null) => return Ordering::Equal,
				(Value::Null, _) => return Ordering::Less,
				(_, Value::Null) => return Ordering::Greater,
				_ => unreachable!("We have already checked that types match nicely above this fn")
			}
		}

		Ordering::Equal
	});
}
