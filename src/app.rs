use std::{
	collections::hash_map::Entry,
	ops::{Deref, Range}
};

use eframe::{
	egui::{self, Align, ComboBox, Key, Layout, Slider, UiBuilder, Vec2b},
	emath::Numeric
};
use egui_plot::{Bar, BarChart, Plot};
use fxhash::FxHashMap;
use merde::ValueType;

use crate::{
	bars::make_bars,
	settings::{Bound, Settings, ValueBound},
	sort::sort_arr
};

pub struct App {
	// We could try to do zero-copy deserialization, but it'll be much easier to work with if we
	// just copy it all - plus, the data will normally just be loaded in once and then manipulated
	// a bunch, so we don't need to optimize the loading-in phase very much. Also it may not
	// actually be super possible to do `yoke`-like stuff 'cause we don't want to have a set
	// schema, meaning that we have to use `merde::Value`, and we can't necessarily
	// `derive(Yokeable)` on that. So.
	//
	// Invariant: Each `Map` inside this vec has the same schema, and contains no nested data
	// structures - no inner `Map`s or `Array`s. It is also not empty.
	data: Vec<merde::Map<'static>>,
	keys: Vec<(String, ValueType)>,
	settings: Settings<'static>,
	pub bars: Vec<Bar>
}

#[derive(thiserror::Error, Debug)]
pub enum AppCreationErr {
	#[error("The provided data is empty")]
	NoData,
	#[error("Differing types were found for the key '{key}': {expected:?} and {found:?}")]
	DifferentTypes {
		key: String,
		expected: ValueType,
		found: ValueType
	},
	#[error(
		"Nested types (like '{0:?}') are not allowed here (we just can't make a bar graph with them)"
	)]
	NestedTypes(ValueType)
}

impl App {
	pub fn new(data: Vec<merde::Map<'static>>) -> Result<Self, AppCreationErr> {
		let Some(first) = data.first() else {
			return Err(AppCreationErr::NoData);
		};

		for map in data.iter().skip(1) {
			for (key, value) in map.iter() {
				match (first[key].value_type(), value.value_type()) {
					// we don't want nested types
					(t @ (ValueType::Map | ValueType::Array), _)
					| (_, t @ (ValueType::Map | ValueType::Array)) => {
						return Err(AppCreationErr::NestedTypes(t));
					}
					// and we're ok with type differences if one is null and the other is a
					// different type - everything's Option around here
					(ValueType::Null, _) | (_, ValueType::Null) => (),
					// But if they're two different types otherwise, that's an error.
					(a, b) if a != b => {
						return Err(AppCreationErr::DifferentTypes {
							key: key.to_string(),
							expected: a,
							found: b
						});
					}
					_ => ()
				}
			}
		}

		let mut keys: Vec<(String, _)> = first
			.iter()
			.map(|(k, v)| (k.to_string(), v.value_type()))
			.collect();

		// sort_by_key requires returning a &str that borrows from the passed-in CowStr and the
		// lifetimes aren't friendly with that.
		#[allow(clippy::unnecessary_sort_by)]
		keys.sort_unstable_by(|(a, _), (b, _)| (**a).cmp(&**b));

		Ok(Self {
			data,
			keys,
			settings: Settings::default(),
			bars: Vec::new()
		})
	}

	pub fn add_key(
		key: String,
		bars: &mut Vec<Bar>,
		data: &mut [merde::Map<'static>],
		settings: &mut Settings
	) {
		settings.x_axis.push(key);
		Self::rebuild_bars(bars, data, settings);
	}

	pub fn remove_key(
		key: &String,
		bars: &mut Vec<Bar>,
		data: &mut [merde::Map<'static>],
		settings: &mut Settings
	) {
		if let Some(idx) = settings.x_axis.iter().position(|k| k == key) {
			settings.x_axis.remove(idx);
		}
		Self::rebuild_bars(bars, data, settings);
	}

	fn rebuild_bars(
		bars: &mut Vec<Bar>,
		data: &mut [merde::Map<'static>],
		settings: &mut Settings
	) {
		let was_empty = bars.is_empty();
		sort_arr(data, &*settings);
		*bars = make_bars(data, &*settings);

		if was_empty {
			settings.max_shown = bars.len();
		}
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			let (id, rect) = ui.allocate_space(ui.available_size());
			let builder = UiBuilder::new()
				.id_salt(id)
				.max_rect(rect)
				.layout(Layout::left_to_right(Align::Center));

			let mut ui = ui.new_child(builder);

			ui.vertical(|ui| {
				ui.heading("Keys");

				for (key, _) in &self.keys {
					let selected = self.settings.x_axis.contains(key);
					if ui.radio(selected, key.deref()).clicked() {
						if selected {
							Self::remove_key(
								key,
								&mut self.bars,
								&mut self.data,
								&mut self.settings
							);
						} else {
							Self::add_key(
								key.clone(),
								&mut self.bars,
								&mut self.data,
								&mut self.settings
							);
						}
					}
				}

				ui.heading("Max shown");

				let num_bars = self.bars.len();
				ui.add(egui::Slider::new(
					&mut self.settings.max_shown,
					0..=num_bars
				));

				ui.heading("Bounds");

				let mut update_bars = false;
				for (key, ty) in &self.keys {
					ComboBox::from_label(&**key)
						.selected_text(
							self.settings
								.bounds
								.get(key)
								.map_or("None", ValueBound::ui_descriptor)
						)
						.show_ui(ui, |ui| {
							update_bars |=
								show_bounds_for_ty(ui, key, *ty, &mut self.settings.bounds)
						});

					if let Some(bound) = self.settings.bounds.get_mut(key) {
						update_bars |= show_bounds_configurations(bound, ui);
					}
				}

				if update_bars {
					Self::rebuild_bars(&mut self.bars, &mut self.data, &mut self.settings);
				}
			});

			if !self.bars.is_empty() {
				Plot::new(id).show(&mut ui, |ui| {
					let bars = self.bars[..self.settings.max_shown.min(self.bars.len())].to_vec();
					ui.set_auto_bounds(Vec2b::TRUE);
					ui.bar_chart(BarChart::new(bars))
				});
			}
		});
	}
}

#[must_use]
fn show_bounds_for_ty(
	ui: &mut egui::Ui,
	key: &String,
	ty: ValueType,
	bounds: &mut FxHashMap<String, ValueBound>
) -> bool {
	let mut current = bounds.get(key).cloned();
	let available_bounds = ValueBound::base_options_for(ty);
	for b in available_bounds {
		ui.selectable_value(&mut current, Some(b.clone()), b.ui_descriptor());
	}
	ui.selectable_value(&mut current, None, "None");

	match (bounds.entry(key.clone()), current) {
		(Entry::Occupied(e), None) => {
			e.remove();
			true
		}
		(Entry::Occupied(mut e), Some(current)) => {
			if current != *e.get() {
				e.insert(current);
				return true;
			}
			false
		}
		(Entry::Vacant(_), None) => false,
		(Entry::Vacant(e), Some(current)) => {
			e.insert(current);
			true
		}
	}
}

fn show_bounds_configurations(bound: &mut ValueBound, ui: &mut egui::Ui) -> bool {
	fn show_slider_for_range<N: Numeric>(range: &mut Range<N>, ui: &mut egui::Ui) {
		ui.add(Slider::new(&mut range.start, N::MIN..=range.end));
		let end = range.end;
		ui.add(Slider::new(&mut range.end, end..=N::MAX));
	}

	match bound {
		ValueBound::I64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::U64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::F64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::Str { include: _, values } => {
			let mut to_remove = None;
			let mut return_rebuild = false;

			for (idx, value) in values.iter_mut().enumerate() {
				ui.horizontal(|ui| {
					return_rebuild |= ui
						.text_edit_singleline(value)
						.ctx
						.input(|state| state.key_pressed(Key::Enter));

					if ui.button("❌").clicked() {
						to_remove = Some(idx);
					}
				});
			}

			if let Some(remove) = to_remove {
				values.remove(remove);
				return_rebuild = true;
			}

			let mut new_val = String::new();
			ui.text_edit_singleline(&mut new_val);
			if !new_val.is_empty() {
				values.push(new_val);
			}

			return return_rebuild;
		}
		_ => ()
	}

	false
}
