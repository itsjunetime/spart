use egui_plot::Bar;
use merde::Value;
use ordered_float::OrderedFloat;

use crate::settings::{Inclusion, Settings, ValueBound, YAxisKey};

pub fn make_bars(data: &[merde::Map], settings: &Settings) -> Vec<Bar> {
	if settings.x_axis.is_empty() {
		return Vec::new();
	}

	let mut bars = match settings.y_axis {
		YAxisKey::Count => {
			let mut bars = Vec::new();

			let mut filtered = data.iter().filter(|val| {
				// Here we want to filter out the ones that we've set in our `bounds`
				// field of `settings`

				let exclude = settings
					.bounds
					.iter()
					.flat_map(|(key, bound)| val.get(key).map(|field| (field, bound)))
					.any(|(field, bound)| match (field, bound) {
						(Value::I64(val), ValueBound::I64(bound)) => bound.excludes(val),
						(Value::U64(val), ValueBound::U64(bound)) => bound.excludes(val),
						(Value::Float(val), ValueBound::F64(bound)) =>
							bound.excludes(&val.into_inner()),
						(Value::Bool(val), ValueBound::Bool(bound)) => val != bound,
						(Value::Str(val), ValueBound::Str { include, values }) => match include {
							Inclusion::Include => !values.contains(val),
							Inclusion::Exclude => values.contains(val)
						},
						(Value::Bytes(_), _) => false,
						// Let's just say that having any bound at all excludes nulls
						(Value::Null, _) => true,
						_ => unreachable!(
							"The rest of the system should make sure we don't have this situation"
						)
					});

				!exclude
			});

			let mut recent_read = None;
			while let Some(val) = recent_read.take().or_else(|| filtered.next()) {
				let old_vals = settings
					.x_axis
					.iter()
					.map(|key| &val[key])
					.collect::<Vec<_>>();

				let mut count = 1;
				for next in filtered.by_ref() {
					let matches = settings
						.x_axis
						.iter()
						.zip(old_vals.iter())
						.all(|(next_key, old_val)| &next[next_key] == *old_val);

					if matches {
						count += 1;
					} else {
						recent_read = Some(next);
						break;
					}
				}

				bars.push(
					Bar::new(bars.len() as f64, count as f64).name(
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
		YAxisKey::Key(_) => todo!()
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