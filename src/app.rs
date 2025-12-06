use std::{borrow::Cow, ops::{Deref, Range}};

use eframe::{
	egui::{self, Align, ComboBox, Key, Layout, Slider, UiBuilder, Vec2b},
	emath::Numeric
};
use egui_plot::{Bar, BarChart, Plot};
use merde::{CowStr, Value, ValueType};
use smallvec::SmallVec;

use crate::{
	bars::make_bars,
	settings::{AccumulatableType, Bound, Inclusion, Settings, ValueBound, YAxisKey, YAxisKeyVariant},
	sort::sort_arr
};

pub type FxHashMap<K, V> = hashbrown::HashMap<K, V, fxhash::FxBuildHasher>;

pub const MAX_ENUM_VARIANTS: usize = 12;
const EXPECTED_SPOTIFY_KEYS: usize = 24;

type EnumValues = SmallVec<[CowStr<'static>; MAX_ENUM_VARIANTS]>;

#[derive(Debug)]
struct KeyData {
	name: CowStr<'static>,
	ty: ValueType,
	enum_values: EnumValues
}

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

	// 24 is based on the number of keys normally in the spotify all-time data
	keys: SmallVec<[KeyData; EXPECTED_SPOTIFY_KEYS]>,
	settings: Settings,
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

		let mut keys: SmallVec<[KeyData; EXPECTED_SPOTIFY_KEYS]> = first
			.iter()
			.map(|(k, v)| {
				let mut enum_values = SmallVec::new();

				let value = v.value_type();

				if value == ValueType::String {
					for val in &data {
						if let Some(Value::Str(s)) = val.get(k) && !enum_values.contains(s) {
							if enum_values.len() == MAX_ENUM_VARIANTS {
								enum_values.clear();
								break;
							}

							enum_values.push(s.to_owned());
						}
					}
				}

				KeyData {
					name: k.to_owned(),
					ty: v.value_type(),
					enum_values
				}
			})
			.collect();

		// sort_by_key requires returning a &str that borrows from the passed-in CowStr and the
		// lifetimes aren't friendly with that.
		#[allow(clippy::unnecessary_sort_by)]
		keys.sort_unstable_by(|a, b| a.name.cmp(&b.name));

		Ok(Self {
			data,
			keys,
			settings: Settings::default(),
			bars: Vec::new()
		})
	}

	pub fn add_key(
		key: CowStr<'static>,
		bars: &mut Vec<Bar>,
		data: &mut [merde::Map<'static>],
		settings: &mut Settings
	) {
		settings.selected_keys.push(key);
		Self::rebuild_bars(bars, data, settings);
	}

	pub fn remove_key(
		key: &CowStr<'static>,
		bars: &mut Vec<Bar>,
		data: &mut [merde::Map<'static>],
		settings: &mut Settings
	) {
		if let Some(idx) = settings.selected_keys.iter().position(|k| k == key) {
			settings.selected_keys.remove(idx);
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

				for KeyData { name, .. } in &self.keys {
					let selected = self.settings.selected_keys.contains(name);
					if ui.radio(selected, name.deref()).clicked() {
						if selected {
							Self::remove_key(
								name,
								&mut self.bars,
								&mut self.data,
								&mut self.settings
							);
						} else {
							Self::add_key(
								name.clone(),
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
				for KeyData { name, ty, enum_values } in &self.keys {
					ComboBox::from_label(&**name)
						.selected_text(
							self.settings
								.bounds
								.get(name)
								.map_or("None", ValueBound::ui_descriptor)
						)
						.show_ui(ui, |ui| {
							update_bars |=
								show_bounds_for_ty(ui, name, *ty, &mut self.settings.bounds, enum_values)
						});

					if let Some(bound) = self.settings.bounds.get_mut(name) {
						update_bars |= show_bounds_configurations(bound, ui);
					}
				}

				if update_bars {
					Self::rebuild_bars(&mut self.bars, &mut self.data, &mut self.settings);
				}

				let first_available_key = match &self.settings.y_axis {
					YAxisKey::SumKey(n, ty) => Some((n, *ty)),
					YAxisKey::Count => self.keys.iter()
						.find_map(|k| {
							println!("looking at key {k:#?}");
							match k.ty {
								ValueType::U64 => Some((&k.name, AccumulatableType::U64)),
								ValueType::Float => Some((&k.name, AccumulatableType::F64)),
								ValueType::I64 => Some((&k.name, AccumulatableType::I64)),
								_ => None
							}
						})
				};

				println!("first_available_key is {first_available_key:#?}");

				if let Some((key_name, key_ty)) = first_available_key {
					ui.heading("Measuring");

					let mut current_y_variant = self.settings.y_axis.to_variant();
					ComboBox::from_label("Method")
						.selected_text(current_y_variant.ui_descriptor())
						.show_ui(ui, |ui| {
							for var in [YAxisKeyVariant::Count, YAxisKeyVariant::SumKey] {
								ui.selectable_value(
									&mut current_y_variant,
									var,
									var.ui_descriptor()
								);
							}
						});

					match (current_y_variant, &self.settings.y_axis) {
						(YAxisKeyVariant::Count, YAxisKey::SumKey(_, _)) => self.settings.y_axis = YAxisKey::Count,
						(YAxisKeyVariant::SumKey, YAxisKey::Count) =>
							self.settings.y_axis = YAxisKey::SumKey(key_name.to_owned(), key_ty),
						_ => ()
					}

					if let YAxisKey::SumKey(y_axis_name, y_axis_ty) = &mut self.settings.y_axis {
						for KeyData { name, ty, enum_values: _ } in &self.keys {
							let mut y_axis_radio = |ty: AccumulatableType| {
								if ui.radio(y_axis_name == name, &**name).clicked() {
									*y_axis_name = name.to_owned();
									*y_axis_ty = ty;
								}
							};

							match ty {
								ValueType::U64 => y_axis_radio(AccumulatableType::U64),
								ValueType::Float => y_axis_radio(AccumulatableType::F64),
								ValueType::I64 => y_axis_radio(AccumulatableType::I64),
								_ => ()
							}
						}
					}
				}
			});

			if !self.bars.is_empty() {
				Plot::new(id).show(&mut ui, |ui| {
					let bars = self.bars[..self.settings.max_shown.min(self.bars.len())].to_vec();
					ui.set_auto_bounds(Vec2b::TRUE);
					ui.bar_chart(BarChart::new("Plot", bars))
				});
			}
		});
	}
}

#[must_use]
fn show_bounds_for_ty(
	ui: &mut egui::Ui,
	key: &CowStr<'static>,
	ty: ValueType,
	bounds: &mut FxHashMap<CowStr<'static>, ValueBound>,
	enum_values: &EnumValues
) -> bool {
	let mut current = bounds.get(key).map(Cow::Borrowed);
	let available_bounds = ValueBound::base_options_for(ty);
	// TODO: switch to using selectable_labels so that we can delay cloning stuff
	for b in available_bounds {
		ui.selectable_value(&mut current, Some(Cow::Owned(b.clone())), b.ui_descriptor());
	}
	if !enum_values.is_empty() && ui.label("Select from list").clicked() {
		current = Some(Cow::Owned(ValueBound::EnumStr {
			values: enum_values.iter().map(|s| (s.clone(), Inclusion::Include)).collect()
		}));
	}

	ui.selectable_value(&mut current, None, "None");

	match current {
		None => bounds.remove(key).is_some(),
		Some(Cow::Borrowed(_)) => false,
		Some(Cow::Owned(b)) => {
			bounds.insert(key.clone(), b);
			true
		}
	}
}

fn show_bounds_configurations(bound: &mut ValueBound, ui: &mut egui::Ui) -> bool {
	fn show_slider_for_range<N: Numeric>(range: &mut Range<N>, ui: &mut egui::Ui) {
		ui.add(Slider::new(&mut range.start, N::MIN..=range.end));
		ui.add(Slider::new(&mut range.end, range.start..=N::MAX));
	}

	match bound {
		ValueBound::I64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::U64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::F64(Bound::Range(range)) => show_slider_for_range(range, ui),
		ValueBound::AnyStr { include: _, values } => {
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
		ValueBound::EnumStr { values } => {
			for (name, inclusion) in values {
				ui.horizontal(|ui| {
					ui.radio_value(inclusion, !*inclusion, &**name);
				});
			}
		}
		_ => ()
	}

	false
}
