use egui_plot::Bar;
use merde::{CowStr, Value};
use ordered_float::OrderedFloat;
use smallvec::SmallVec;

use crate::settings::{AccumulatableType, Inclusion, Settings, ValueBound, YAxisKey};

pub fn make_bars(data: &[merde::Map], settings: &Settings) -> Vec<Bar> {
	if settings.selected_keys.is_empty() {
		return Vec::new();
	}

	// go through every single item in our data, removing the ones that our filter section
	// has filtered out
	let filtered = data.iter().filter(|val| {
		// Here we want to filter out the ones that we've set in our `bounds`
		// field of `settings`

		let exclude = settings
			.bounds
			.iter()
			.filter_map(|(key, bound)| val.get(key).map(|field| (field, bound)))
			.any(|(field, bound)| match (field, bound) {
				(Value::I64(val), ValueBound::I64(bound)) => bound.excludes(val),
				(Value::U64(val), ValueBound::U64(bound)) => bound.excludes(val),
				(Value::Float(val), ValueBound::F64(bound)) => bound.excludes(&val.into_inner()),
				(Value::Bool(val), ValueBound::Bool(bound)) => val != bound,
				(Value::Str(val), ValueBound::AnyStr { include, values }) => match include {
					Inclusion::Include => !values.iter().any(|s| s == &**val),
					Inclusion::Exclude => values.iter().any(|s| s == &**val)
				},
				(Value::Str(val), ValueBound::EnumStr { values }) => values
					.iter()
					.filter(|(_, incl)| *incl == Inclusion::Exclude)
					.any(|(s, _)| s == val),
				(Value::Bytes(_), _) => false,
				// Let's just say that having any bound at all excludes nulls
				(Value::Null, _) => true,
				_ => unreachable!(
					"The rest of the system should make sure we don't have this situation"
				)
			});

		!exclude
	});

	let mut bars = match &settings.y_axis {
		YAxisKey::Count => make_bars_with_accumulator(filtered, settings, || CountAccumulator(0)),
		YAxisKey::SumKey(key, ty) => match ty {
			AccumulatableType::F64 => make_bars_with_accumulator(filtered, settings, || {
				KeyAccumulator { key, value: 0.0 }
			}),
			AccumulatableType::U64 => make_bars_with_accumulator(filtered, settings, || {
				KeyAccumulator { key, value: 0u64 }
			}),
			AccumulatableType::I64 => make_bars_with_accumulator(filtered, settings, || {
				KeyAccumulator { key, value: 0i64 }
			})
		}
	};

	bars.sort_unstable_by_key(|b| OrderedFloat(b.value));
	bars.reverse();

	bars.into_iter()
		.enumerate()
		.map(|(idx, mut b)| {
			b.argument = idx as f64;
			b
		})
		.collect()
}

trait Accumulator {
	fn add(&mut self, value: &merde::Map);
	fn finish(self) -> f64;
}

struct CountAccumulator(usize);

impl Accumulator for CountAccumulator {
	fn add(&mut self, _: &merde::Map) {
		self.0 += 1;
	}

	fn finish(self) -> f64 {
		self.0 as f64
	}
}

struct KeyAccumulator<'a, T> {
	key: &'a CowStr<'static>,
	value: T
}

impl Accumulator for KeyAccumulator<'_, u64> {
	fn add(&mut self, value: &merde::Map) {
		match value.get(self.key) {
			None => (),
			Some(merde::Value::U64(v)) => self.value += v,
			_ => unreachable!()
		}
	}

	fn finish(self) -> f64 {
		self.value as f64
	}
}

impl Accumulator for KeyAccumulator<'_, f64> {
	fn add(&mut self, value: &merde::Map) {
		match value.get(self.key) {
			None => (),
			Some(merde::Value::Float(f)) => self.value += **f,
			_ => unreachable!()
		}
	}

	fn finish(self) -> f64 {
		self.value
	}
}

impl Accumulator for KeyAccumulator<'_, i64> {
	fn add(&mut self, value: &merde::Map) {
		match value.get(self.key) {
			None => (),
			Some(merde::Value::I64(i)) => self.value += i,
			_ => unreachable!()
		}
	}

	fn finish(self) -> f64 {
		self.value as f64
	}
}

fn make_bars_with_accumulator<'a, A, I>(
	mut filtered: I,
	settings: &Settings,
	make_acc: impl Fn() -> A
) -> Vec<Bar>
where
	A: Accumulator,
	I: Iterator<Item = &'a merde::Map<'a>>
{
	let mut bars = Vec::new();

	let mut recent_read = None;
	while let Some(val) = recent_read.take().or_else(|| filtered.next()) {
		// for each item that we know we want to look at, go through all the keys that we
		// are grouping this data by, and get their values on this item
		let old_vals = settings
			.selected_keys
			.iter()
			.map(|key| &val[key])
			.collect::<SmallVec<[&merde::Value; 3]>>();

		let mut accumulator = make_acc();
		accumulator.add(val);

		for next in filtered.by_ref() {
			// then continue going through each item in the filtered list and see if its
			// value is the same as the original one in this group
			let matches = settings
				.selected_keys
				.iter()
				.zip(old_vals.iter())
				.all(|(next_key, old_val)| &next[next_key] == *old_val);

			// if it does belong in this group (i.e. matches the value that we're using for
			// this grouping), increase the count!
			if matches {
				accumulator.add(next);
			} else {
				// otherwise, store it as the starting value for the next group and break.
				recent_read = Some(next);
				break;
			}
		}

		// then add the bar
		bars.push(
			Bar::new(bars.len() as f64, accumulator.finish()).name(
				old_vals
					.iter()
					.map(|s| format!("{s:?}"))
					.collect::<Vec<_>>()
					.join(",")
			)
		);
	}

	bars
}
