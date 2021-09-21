use std::str::Chars;

use uuid::Uuid;

pub struct XMLWriter {
	data: String,
	indent: usize,
	element_stack: Vec<String>,
}

impl XMLWriter {
	pub fn start() -> Self {
		Self {
			data: format!(
				r#"<?xml version="1.0" encoding="utf-8"?>

<!-- 
	This XML file was generated by the behave compiler.
			
	Manual changes to this file may cause unexpected behavior.
	Manual changes will be lost if the behave project is recompiled.
-->
			
<ModelInfo version="1.0" guid="{{{}}}">
"#,
				Uuid::new_v4().to_hyphenated()
			),
			indent: 1,
			element_stack: Vec::new(),
		}
	}

	pub fn start_element(&mut self, name: impl AsRef<str>) {
		self.indent();
		self.data.push('<');
		self.element_stack
			.push(String::from_iter(EscapeIterator::new(name.as_ref())));
		self.data.extend(EscapeIterator::new(name.as_ref()));
		self.data += ">\n";

		self.indent += 1;
	}

	pub fn start_element_attrib<'a>(
		&mut self, name: impl AsRef<str>,
		attributes: impl IntoIterator<Item = (impl AsRef<str> + 'a, impl AsRef<str> + 'a)>,
	) {
		self.indent();
		self.data.push('<');
		self.element_stack
			.push(String::from_iter(EscapeIterator::new(name.as_ref())));
		self.data.extend(EscapeIterator::new(name.as_ref()));

		for attribute in attributes {
			self.data += " ";
			self.data.extend(EscapeIterator::new(attribute.0.as_ref()));
			self.data += "=\"";
			self.data.extend(EscapeIterator::new(attribute.1.as_ref()));
			self.data += "\"";
		}

		self.data += ">\n";

		self.indent += 1;
	}

	pub fn element(
		&mut self, name: impl AsRef<str>, attributes: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
	) {
		self.indent();
		self.data.push('<');
		self.data.extend(EscapeIterator::new(name.as_ref()));

		for attribute in attributes {
			self.data += " ";
			self.data.extend(EscapeIterator::new(attribute.0.as_ref()));
			self.data += "=\"";
			self.data.extend(EscapeIterator::new(attribute.1.as_ref()));
			self.data += "\"";
		}

		self.data += "/>\n";
	}

	pub fn data(&mut self, data: impl AsRef<str>) {
		self.indent();
		self.data.push_str(data.as_ref());
		self.data.push('\n');
	}

	pub fn end_element(&mut self) {
		self.indent -= 1;
		self.indent();

		self.data += "</";
		self.data.push_str(&self.element_stack.pop().unwrap());
		self.data += ">\n";
	}

	pub fn end(self) -> String { self.data + "</ModelInfo>\n" }

	fn indent(&mut self) {
		self.data.extend(IndentIterator {
			indentation: self.indent,
		});
	}
}

pub struct IndentIterator {
	pub indentation: usize,
}

impl Iterator for IndentIterator {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		if self.indentation > 0 {
			self.indentation -= 1;
			Some('\t')
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) { (self.indentation, Some(self.indentation)) }
}

enum EscapeMode {
	Normal,
	LessThan(u8),
	GreaterThan(u8),
	Ampersand(u8),
	Quote(u8),
}

struct EscapeIterator<'a> {
	chars: Chars<'a>,
	mode: EscapeMode,
}

impl<'a> EscapeIterator<'a> {
	fn new(str: &'a str) -> Self {
		Self {
			chars: str.chars(),
			mode: EscapeMode::Normal,
		}
	}
}

impl Iterator for EscapeIterator<'_> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		match self.mode {
			EscapeMode::Normal => {
				let next = self.chars.next()?;
				Some(match next {
					'<' => {
						self.mode = EscapeMode::LessThan(0);
						'&'
					},
					'>' => {
						self.mode = EscapeMode::GreaterThan(0);
						'&'
					},
					'&' => {
						self.mode = EscapeMode::Ampersand(0);
						'&'
					},
					'"' => {
						self.mode = EscapeMode::Quote(0);
						'&'
					},
					_ => next,
				})
			},
			EscapeMode::LessThan(ref mut val) => {
				*val += 1;
				Some(match val {
					1 => 'l',
					2 => 't',
					3 => {
						self.mode = EscapeMode::Normal;
						';'
					},
					_ => unreachable!(),
				})
			},
			EscapeMode::GreaterThan(ref mut val) => {
				*val += 1;
				Some(match val {
					1 => 'g',
					2 => 't',
					3 => {
						self.mode = EscapeMode::Normal;
						';'
					},
					_ => unreachable!(),
				})
			},
			EscapeMode::Ampersand(ref mut val) => {
				*val += 1;
				Some(match val {
					1 => 'a',
					2 => 'm',
					3 => 'p',
					4 => {
						self.mode = EscapeMode::Normal;
						';'
					},
					_ => unreachable!(),
				})
			},
			EscapeMode::Quote(ref mut val) => {
				*val += 1;
				Some(match val {
					1 => 'q',
					2 => 'u',
					3 => 'o',
					4 => 't',
					5 => {
						self.mode = EscapeMode::Normal;
						';'
					},
					_ => unreachable!(),
				})
			},
		}
	}
}
