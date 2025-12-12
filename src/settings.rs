use core::{fmt::Debug, ops::{Not, Range}};

use merde::{CowStr, ValueType};
use smallvec::SmallVec;

use crate::app::{FxHashMap, MAX_ENUM_VARIANTS};

type SelectedKeys = SmallVec<[CowStr<'static>; 3]>;

pub struct Settings {
	pub bounds: FxHashMap<CowStr<'static>, ValueBound>,
	pub selected_keys: SelectedKeys,
	pub y_axis: YAxisKey,
	pub max_shown: usize
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			bounds: FxHashMap::default(),
			selected_keys: const { SelectedKeys::new_const() },
			y_axis: YAxisKey::default(),
			max_shown: usize::MAX
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub enum AccumulatableType {
	F64,
	U64,
	I64
}

#[derive(Default)]
pub enum YAxisKey {
	#[default]
	Count,
	SumKey(CowStr<'static>, AccumulatableType)
}

impl YAxisKey {
	pub fn to_variant(&self) -> YAxisKeyVariant {
		match self {
			Self::Count => YAxisKeyVariant::Count,
			Self::SumKey(_, _) => YAxisKeyVariant::SumKey
		}
	}
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum YAxisKeyVariant {
	Count,
	SumKey
}

impl YAxisKeyVariant {
	pub fn ui_descriptor(&self) -> &'static str {
		match self {
			Self::Count => "Simple Counting",
			Self::SumKey => "Sum values by key",
		}
	}
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
		T: PartialEq + PartialOrd + Debug
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

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Inclusion {
	Include,
	Exclude
}

impl Not for Inclusion {
	type Output = Self;
	fn not(self) -> Self::Output {
		match self {
			Self::Include => Self::Exclude,
			Self::Exclude => Self::Include
		}
	}
}

#[derive(Clone, PartialEq, Debug)]
pub enum ValueBound {
	I64(Bound<i64>),
	U64(Bound<u64>),
	F64(Bound<f64>),
	EnumStr {
		values: SmallVec<[(CowStr<'static>, Inclusion); MAX_ENUM_VARIANTS]>
	},
	AnyStr {
		include: Inclusion,
		values: Vec<String>
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
			ValueBound::AnyStr {
				include: Inclusion::Include,
				values: vec![]
			},
			ValueBound::AnyStr {
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
			ValueType::Bytes | ValueType::Null => &[],
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
			| ValueBound::AnyStr {
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
			| ValueBound::AnyStr {
				include: Inclusion::Include,
				..
			} => "Include Values",
			ValueBound::EnumStr { .. } => "List Filter",
			ValueBound::Bool(true) => "true",
			ValueBound::Bool(false) => "false"
		}
	}
}
