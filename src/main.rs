use app::App;
use eframe::egui;
use merde::{IntoStatic, json::from_str};

mod app;
mod bars;
mod settings;
mod song;
mod sort;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = std::env::args().skip(1);

	let json_data = args
		.map(std::fs::read_to_string)
		.collect::<Result<Vec<_>, _>>()?
		.join("\n");

	let deserialized: Vec<merde::Map<'static>> = from_str::<Vec<merde::Map>>(&json_data)
		.unwrap()
		.into_static();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([600., 400.]),
		..Default::default()
	};
	eframe::run_native(
		"Spart",
		options,
		Box::new(move |_| {
			App::new(deserialized)
				.map(|a| Box::new(a) as _)
				.map_err(|e| Box::new(e) as _)
		})
	)?;

	Ok(())
}
