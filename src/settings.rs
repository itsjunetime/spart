use std::ops::Range;

use fxhash::FxHashMap;
use merde::{CowStr, ValueType};

pub struct Settings<'keys> {
	pub bounds: FxHashMap<CowStr<'static>, ValueBound>,
	pub x_axis: Vec<CowStr<'static>>,
	pub y_axis: YAxisKey<'keys>,
	pub max_shown: usize
}

impl Default for Settings<'_> {
	fn default() -> Self {
		Self {
			bounds: FxHashMap::default(),
			x_axis: Vec::new(),
			y_axis: YAxisKey::default(),
			max_shown: usize::MAX
		}
	}
}

#[derive(Default)]
pub enum YAxisKey<'keys> {
	#[default]
	Count,
	Key(&'keys str)
}

#[derive(Clone, PartialEq, Debug)]
pub enum Bound<T> {
	Range(Range<T>),
	Specifics { include: Inclusion, values: Vec<T> }
}

impl<T> Bound<T> {
	const fn specifics(include: Inclusion) -> Self {
		Self::Specifics {
			include,
			values: vec![]
		}
	}

	pub fn excludes(&self, val: &T) -> bool
	where
		T: PartialEq + PartialOrd
	{
		match self {
			Self::Range(range) => !range.contains(val),
			Self::Specifics {
				include: Inclusion::Exclude,
				values
			} => values.contains(val),
			Self::Specifics {
				include: Inclusion::Include,
				values
			} => !values.contains(val)
		}
	}
}

impl<T> Default for Bound<T> {
	fn default() -> Self {
		Self::specifics(Inclusion::Exclude)
	}
}

#[derive(Clone, PartialEq, Debug)]
pub enum Inclusion {
	Include,
	Exclude
}

#[derive(Clone, PartialEq, Debug)]
pub enum ValueBound {
	I64(Bound<i64>),
	U64(Bound<u64>),
	F64(Bound<f64>),
	Str {
		include: Inclusion,
		values: Vec<CowStr<'static>>
	},
	Bool(bool)
}

impl ValueBound {
	pub fn base_options_for(ty: ValueType) -> &'static [Self] {
		static I64_ARR: &[ValueBound] = &[
			ValueBound::I64(Bound::Range(0..i64::MAX)),
			ValueBound::I64(Bound::specifics(Inclusion::Exclude)),
			ValueBound::I64(Bound::specifics(Inclusion::Include))
		];
		static U64_ARR: &[ValueBound] = &[
			ValueBound::U64(Bound::Range(0..u64::MAX)),
			ValueBound::U64(Bound::specifics(Inclusion::Exclude)),
			ValueBound::U64(Bound::specifics(Inclusion::Include))
		];
		static F64_ARR: &[ValueBound] = &[
			ValueBound::F64(Bound::Range(0.0..f64::MAX)),
			ValueBound::F64(Bound::specifics(Inclusion::Exclude)),
			ValueBound::F64(Bound::specifics(Inclusion::Include))
		];
		static STR_ARR: &[ValueBound] = &[
			ValueBound::Str {
				include: Inclusion::Include,
				values: vec![]
			},
			ValueBound::Str {
				include: Inclusion::Exclude,
				values: vec![]
			}
		];

		match ty {
			ValueType::I64 => I64_ARR,
			ValueType::U64 => U64_ARR,
			ValueType::Float => F64_ARR,
			ValueType::String => STR_ARR,
			ValueType::Bool => &[ValueBound::Bool(true), ValueBound::Bool(false)],
			ValueType::Bytes => &[],
			ValueType::Null => &[],
			_ => unreachable!("These values should've been checked by this point")
		}
	}

	pub fn ui_descriptor(&self) -> &'static str {
		match self {
			ValueBound::I64(Bound::Range(_))
			| ValueBound::U64(Bound::Range(_))
			| ValueBound::F64(Bound::Range(_)) => "Range",
			ValueBound::I64(Bound::Specifics {
				include: Inclusion::Exclude,
				..
			})
			| ValueBound::U64(Bound::Specifics {
				include: Inclusion::Exclude,
				..
			})
			| ValueBound::F64(Bound::Specifics {
				include: Inclusion::Exclude,
				..
			})
			| ValueBound::Str {
				include: Inclusion::Exclude,
				..
			} => "Exclude Values",
			ValueBound::I64(Bound::Specifics {
				include: Inclusion::Include,
				..
			})
			| ValueBound::U64(Bound::Specifics {
				include: Inclusion::Include,
				..
			})
			| ValueBound::F64(Bound::Specifics {
				include: Inclusion::Include,
				..
			})
			| ValueBound::Str {
				include: Inclusion::Include,
				..
			} => "Include Values",
			ValueBound::Bool(true) => "true",
			ValueBound::Bool(false) => "false"
		}
	}
}
