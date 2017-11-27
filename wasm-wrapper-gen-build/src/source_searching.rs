use failure::Error;

use syn;

use wasm_wrapper_gen_shared::transform_mac_to_items;

pub fn walk_crate_for_js_fns(source: &str) -> Result<Vec<syn::Item>, Error> {
    use syn::visit::Visitor;

    let ast = syn::parse_crate(source).map_err(|e| {
        format_err!("failed to parse macro input as an item: {}", e)
    })?;

    let mut v = FindMacrosVisitor::find_js_fn();

    v.visit_crate(&ast);

    // flat_map doesn't work well with Result<Vec<T>, E>.
    let mut func_definition_items = Vec::new();

    for found_macro in v.found {
        func_definition_items.extend(transform_mac_to_items(found_macro)?);
    }

    Ok(func_definition_items)
}

struct FindMacrosVisitor {
    ident_to_find: syn::Path,
    found: Vec<syn::Mac>,
}

impl FindMacrosVisitor {
    fn new(ident_to_find: syn::Path) -> Self {
        FindMacrosVisitor {
            ident_to_find: ident_to_find,
            found: Vec::new(),
        }
    }

    fn find_js_fn() -> Self {
        FindMacrosVisitor::new(syn::Path {
            global: false,
            segments: vec![
                syn::PathSegment {
                    ident: syn::Ident::new("js_fn"),
                    parameters: syn::PathParameters::none(),
                },
            ],
        })
    }
}

impl syn::visit::Visitor for FindMacrosVisitor {
    fn visit_mac(&mut self, mac: &syn::Mac) {
        // TODO: can macros ever have global paths? This would break if that's
        // the case. Right now we require an exact match on non-global 'js_fn!',
        // if there could be another way to invoke it, we might want to use
        // some fuzzy matching.
        if mac.path == self.ident_to_find {
            self.found.push(mac.clone());
        }
    }
}
