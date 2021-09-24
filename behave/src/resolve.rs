use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::ast::{
	ASTPass,
	ASTTree,
	ASTType,
	Access,
	EnumAccess,
	EnumType,
	FunctionAccess,
	GlobalAccess,
	ImportType,
	InbuiltEnum,
	InbuiltFunction,
	ItemType,
	MouseEvent,
	Path,
	ResolvedAccess,
	ResolvedType,
	StructType,
	Type,
	TypeType,
	Use,
	AST,
};
use crate::diagnostic::{Diagnostic, Label, Level};
use crate::items::{FunctionId, ItemMap, TemplateId};

fn insert_event(h: &mut HashMap<Vec<&'static str>, EnumAccess>, event: MouseEvent) {
	h.insert(
		vec!["MouseEvent", event.to_string()],
		EnumAccess {
			id: EnumType::Inbuilt(InbuiltEnum::MouseEvent),
			value: event as usize,
		},
	);
}

lazy_static! {
	static ref INBUILT_FUNCTION_MAP: HashMap<Vec<&'static str>, InbuiltFunction> = {
		let mut h = HashMap::new();
		h.insert(vec!["format"], InbuiltFunction::Format);
		h
	};
	static ref INBUILT_ENUM_ACCESS_MAP: HashMap<Vec<&'static str>, EnumAccess> = {
		let mut h = HashMap::new();

		insert_event(&mut h, MouseEvent::RightSingle);
		insert_event(&mut h, MouseEvent::MiddleSingle);
		insert_event(&mut h, MouseEvent::LeftSingle);
		insert_event(&mut h, MouseEvent::RightDouble);
		insert_event(&mut h, MouseEvent::MiddleDouble);
		insert_event(&mut h, MouseEvent::LeftDouble);
		insert_event(&mut h, MouseEvent::RightDrag);
		insert_event(&mut h, MouseEvent::MiddleDrag);
		insert_event(&mut h, MouseEvent::LeftDrag);
		insert_event(&mut h, MouseEvent::RightRelease);
		insert_event(&mut h, MouseEvent::MiddleRelease);
		insert_event(&mut h, MouseEvent::LeftRelease);
		insert_event(&mut h, MouseEvent::Lock);
		insert_event(&mut h, MouseEvent::Unlock);
		insert_event(&mut h, MouseEvent::Move);
		insert_event(&mut h, MouseEvent::Leave);
		insert_event(&mut h, MouseEvent::WheelUp);
		insert_event(&mut h, MouseEvent::WheelDown);

		h
	};
	static ref INBUILT_ENUM_MAP: HashMap<Vec<&'static str>, EnumType> = {
		let mut h = HashMap::new();

		h.insert(vec!["MouseEvent"], EnumType::Inbuilt(InbuiltEnum::MouseEvent));

		h
	};
	static ref INBUILT_STRUCT_MAP: HashMap<Vec<&'static str>, StructType> = {
		let mut h = HashMap::new();
		h
	};
}

#[derive(Debug)]
struct Resolver<'a> {
	types: HashMap<Vec<String>, ResolvedType>,
	templates: HashMap<Vec<String>, TemplateId>,
	functions: HashMap<Vec<String>, FunctionId>,
	enum_variants: HashMap<Vec<String>, EnumAccess>,
	diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
	fn new<'b>(
		diagnostics: &'a mut Vec<Diagnostic>, this: &'b AST<'b>, root_tree: &'b ASTTree<'b>, item_map: &'b ItemMap<'b>,
		imports: impl Iterator<Item = &'b Path<'b>>,
	) -> Result<Resolver<'a>, ()> {
		let mut roots = vec![root_tree];
		for import in imports {
			match root_tree.get_ast(&import.0) {
				Ok(tree) => roots.push(tree),
				Err(diag) => diagnostics.push(diag.add_label(Label::primary("here", import.1.clone()))),
			}
		}

		if diagnostics.len() == 0 {
			let mut resolver = Self {
				types: HashMap::new(),
				templates: HashMap::new(),
				functions: HashMap::new(),
				enum_variants: HashMap::new(),
				diagnostics,
			};

			resolver.add_inbuilt_types();
			resolver.add_items(this, item_map, &[]);

			for root in roots {
				resolver.add_items_recursive(root, item_map, &[]);
			}

			Ok(resolver)
		} else {
			Err(())
		}
	}

	fn add_inbuilt_types(&mut self) {
		for e in INBUILT_ENUM_MAP.iter() {
			self.types.insert(
				e.0.into_iter().map(|f| f.to_string()).collect(),
				ResolvedType::Enum(*e.1),
			);
		}

		for s in INBUILT_STRUCT_MAP.iter() {
			self.types.insert(
				s.0.into_iter().map(|f| f.to_string()).collect(),
				ResolvedType::Struct(*s.1),
			);
		}
	}

	fn add_items(&mut self, ast: &AST, item_map: &ItemMap, path: &[String]) {
		if let ASTType::Secondary(ref items) = ast.ast_data {
			for item in items {
				match item.0 {
					ItemType::Enum(e) => {
						let mut path = path.to_vec();
						let en = item_map.get_enum(e);
						path.push(en.name.0.clone());
						if let Some(_) = self.types.get(&path) {
							self.diagnostics
								.push(
									Diagnostic::new(Level::Error, "type redeclaration").add_label(Label::primary(
										"a type with the same name is already in scope",
										en.name.1.clone(),
									)),
								)
						} else {
							self.types.insert(path.clone(), ResolvedType::Enum(EnumType::User(e)));
						}

						for variant in en.variants.iter() {
							let mut path = path.clone();
							path.push(variant.name.0.clone());
							self.enum_variants.insert(
								path,
								EnumAccess {
									id: EnumType::User(e),
									value: variant.value,
								},
							);
						}
					},
					ItemType::Struct(s) => {
						let mut path = path.to_vec();
						let st = item_map.get_struct(s);
						path.push(st.name.0.clone());

						if let Some(_) = self.types.get(&path) {
							self.diagnostics
								.push(
									Diagnostic::new(Level::Error, "type redeclaration").add_label(Label::primary(
										"a type with the same name is already in scope",
										st.name.1.clone(),
									)),
								)
						} else {
							self.types.insert(path, ResolvedType::Struct(StructType::User(s)));
						}
					},
					ItemType::Template(t) => {
						let mut path = path.to_vec();
						let te = item_map.get_template(t);
						path.push(te.name.0.clone());
						if let Some(t) = self.templates.get(&path) {
							self.diagnostics.push(
								Diagnostic::new(Level::Error, "template redefinition")
									.add_label(Label::primary(
										"a template with the same name is already in scope",
										te.name.1.clone(),
									))
									.add_label(Label::secondary(
										"template previously defined here",
										item_map.get_template(*t).name.1.clone(),
									)),
							)
						} else {
							self.templates.insert(path, t);
						}
					},
					ItemType::Function(ref name, f) => {
						let mut path = path.to_vec();
						path.push(name.0.clone());

						if let Some(f) = self.functions.get(&path) {
							self.diagnostics
								.push(
									Diagnostic::new(Level::Error, "function redefinition").add_label(Label::primary(
										"a function with the same name is already in scope",
										name.1.clone(),
									)),
								)
						} else {
							self.functions.insert(path, f);
						}
					},
				}
			}
		}
	}

	fn add_items_recursive<'b>(&mut self, root: &'b ASTTree<'b>, item_map: &'b ItemMap<'b>, path: &[String]) {
		match root {
			ASTTree::Branch(ref map) => {
				for pair in map {
					let mut path = path.to_vec();
					path.push(pair.0.clone());
					self.add_items_recursive(pair.1, item_map, &path);
				}
			},
			ASTTree::Leaf(ref ast) => self.add_items(ast, item_map, path),
		}
	}
}

impl ASTPass for Resolver<'_> {
	fn ty<'b>(&mut self, ty: &mut Type<'b>) {
		match ty.0 {
			TypeType::Other(ref mut user) => {
				if let Some(resolved) = self
					.types
					.get(&user.path.0.iter().map(|s| s.0.clone()).collect::<Vec<_>>())
				{
					user.resolved = Some(*resolved);
				} else {
					self.diagnostics.push(
						Diagnostic::new(Level::Error, "type does not exist")
							.add_label(Label::primary("here", user.path.1.clone())),
					)
				}
			},
			TypeType::Array(ref mut ty) => self.ty(ty.as_mut()),
			TypeType::Map(ref mut key, ref mut value) => {
				self.ty(key.as_mut());
				self.ty(value.as_mut());
			},
			TypeType::Sum(ref mut tys) => {
				for ty in tys.iter_mut() {
					self.ty(ty);
				}
			},
			TypeType::Function(ref mut f) => {
				for ty in f.args.iter_mut() {
					self.ty(ty);
				}
				f.ret.as_mut().map(|ty| self.ty(ty.as_mut()));
			},
			_ => {},
		}
	}

	fn access(&mut self, access: &mut Access) {
		access.resolved = Some(
			if let Some(inbuilt) =
				INBUILT_ENUM_ACCESS_MAP.get(&access.path.0.iter().map(|s| s.0.as_str()).collect::<Vec<_>>())
			{
				ResolvedAccess::Global(GlobalAccess::Enum(*inbuilt))
			} else if let Some(inbuilt) =
				INBUILT_FUNCTION_MAP.get(&access.path.0.iter().map(|s| s.0.as_str()).collect::<Vec<_>>())
			{
				ResolvedAccess::Global(GlobalAccess::Function(FunctionAccess::Inbuilt(*inbuilt)))
			} else if let Some(resolved) = self
				.functions
				.get(&access.path.0.iter().map(|s| s.0.clone()).collect::<Vec<_>>())
			{
				ResolvedAccess::Global(GlobalAccess::Function(FunctionAccess::User(*resolved)))
			} else if let Some(resolved) = self
				.enum_variants
				.get(&access.path.0.iter().map(|s| s.0.clone()).collect::<Vec<_>>())
			{
				ResolvedAccess::Global(GlobalAccess::Enum(*resolved))
			} else {
				ResolvedAccess::Local
			},
		);
	}

	fn template_use<'b>(&mut self, us: &mut Use<'b>) {
		if let Some(resolved) = self
			.templates
			.get(&us.template.path.0.iter().map(|s| s.0.clone()).collect::<Vec<_>>())
		{
			us.template.resolved = Some(*resolved);
		} else {
			self.diagnostics.push(
				Diagnostic::new(Level::Error, "template does not exist")
					.add_label(Label::primary("here", us.template.path.1.clone())),
			)
		}

		for arg in us.args.iter_mut() {
			self.expression(&mut arg.1 .0);
		}
	}
}

pub fn resolve(main: &mut AST, others: &mut ASTTree, item_map: &mut ItemMap) -> Result<(), Vec<Diagnostic>> {
	let mut errors = Vec::new();

	let copied_for_imported = others.clone();
	if let Err(diag) = resolve_imported(&copied_for_imported, others, item_map) {
		errors.extend(diag);
	}

	let imports = main.imports.iter().filter_map(|import| {
		if let ImportType::Normal(p) = &import.0 {
			Some(p)
		} else {
			None
		}
	});
	let mut resolver = if let Ok(res) = Resolver::new(&mut errors, main, others, item_map, imports) {
		res
	} else {
		return Err(errors);
	};

	if let ASTType::Main(ref mut lods, ref mut behavior) = main.ast_data {
		resolver.lods(lods);
		resolver.behavior(behavior);
	} else {
		unreachable!();
	}

	if errors.len() == 0 {
		Ok(())
	} else {
		Err(errors)
	}
}

fn resolve_imported(root: &ASTTree, tree: &mut ASTTree, item_map: &mut ItemMap) -> Result<(), Vec<Diagnostic>> {
	let mut errors = Vec::new();

	match tree {
		ASTTree::Branch(ref mut map) => {
			for pair in map.iter_mut() {
				if let Err(diag) = resolve_imported(root, pair.1, item_map) {
					errors.extend(diag);
				}
			}
		},
		ASTTree::Leaf(ref mut ast) => {
			let mut resolver = if let Ok(res) = Resolver::new(
				&mut errors,
				ast,
				root,
				item_map,
				ast.imports.iter().filter_map(|import| {
					if let ImportType::Normal(p) = &import.0 {
						Some(p)
					} else {
						None
					}
				}),
			) {
				res
			} else {
				return Err(errors);
			};

			if let ASTType::Secondary(ref mut items) = ast.ast_data {
				for item in items {
					resolver.item(item, item_map)
				}
			} else {
				unreachable!()
			}
		},
	}

	if errors.len() == 0 {
		Ok(())
	} else {
		Err(errors)
	}
}
