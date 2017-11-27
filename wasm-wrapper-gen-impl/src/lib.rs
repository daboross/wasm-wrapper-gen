extern crate arrayvec;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate proc_macro_hack;
#[macro_use]
extern crate quote;
extern crate syn;

extern crate wasm_wrapper_gen_shared;

use failure::{Error, ResultExt};

use wasm_wrapper_gen_shared::{extract_func_info, get_argument_types,
                              transform_macro_input_to_items, KnownArgumentType,
                              TransformedRustIdent};


#[derive(Debug, Clone)]
/// Small constructed ident struct supporting up to four suffixes.
struct ConstructedArgIdent {
    base: &'static str,
    number_suffix: u32,
    suffixes: arrayvec::ArrayVec<[&'static str; 4]>,
}

impl ConstructedArgIdent {
    fn new(base: &'static str, number_suffix: u32) -> Self {
        ConstructedArgIdent {
            base,
            number_suffix,
            suffixes: arrayvec::ArrayVec::new(),
        }
    }

    fn with_suffix(&self, suffix: &'static str) -> Self {
        let mut cloned = self.clone(); // this is a cheap copy
        cloned.suffixes.push(suffix);
        cloned
    }
}

impl quote::ToTokens for ConstructedArgIdent {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        let mut ident = format!("{}{}", self.base, self.number_suffix);
        for suffix in &self.suffixes {
            ident.push_str(suffix);
        }
        tokens.append(ident);
    }
}

proc_macro_item_impl! {
    pub fn __js_fn_impl(input: &str) -> String {
        match process_all_functions(input) {
            Ok(v) => v,
            Err(e) => {
                panic!("js_fn macro failed: {}", e);
            }
        }
    }
}

fn process_all_functions(input: &str) -> Result<String, Error> {
    let token_trees = syn::parse_token_trees(input).map_err(|e| {
        format_err!("failed to parse macro input as an item: {}", e)
    })?;

    let ast = transform_macro_input_to_items(token_trees)?;

    let mut full_out = quote::Tokens::new();
    for item in &ast {
        let output = process_item(item).with_context(|e| {
            format!("failed to process function '{:?}': {}", item, e)
        })?;

        full_out.append(output);
    }
    Ok(full_out.to_string())
}

fn process_item(item: &syn::Item) -> Result<quote::Tokens, Error> {
    let (item, decl, block) = extract_func_info(item)?;

    let out = generate_function_wrapper(item, decl, block)?;

    Ok(out)
}

fn generate_function_wrapper(
    item: &syn::Item,
    decl: &syn::FnDecl,
    code: &syn::Block,
) -> Result<quote::Tokens, Error> {
    let callable_body = generate_callable_body(item, decl, code)?;

    let argument_types = get_argument_types(decl)?;

    let argument_names = (0..argument_types.len() as u32)
        .map(|index| ConstructedArgIdent::new("__arg", index))
        .collect::<Vec<_>>();

    let mut function_body = quote::Tokens::new();

    for (ty, arg_name) in argument_types.iter().zip(&argument_names) {
        function_body.append(setup_for_argument(&arg_name, ty)?);
    }

    let mut arg_names_as_argument_list = quote::Tokens::new();
    for arg_name in &argument_names {
        arg_names_as_argument_list.append(quote! { #arg_name, });
    }

    function_body.append(quote! {
        // TODO: handle results as well...
        let __result: () = (#callable_body)(#arg_names_as_argument_list);
    });

    let func_ident = TransformedRustIdent::new(&item.ident);

    let mut real_arguments_list = quote::Tokens::new();
    for (ty, arg_name) in argument_types.iter().zip(&argument_names) {
        expand_argument_into(arg_name, ty, &mut real_arguments_list)?;
    }

    let full_definition = quote! {
        extern "C" fn #func_ident (#real_arguments_list) {
            #function_body
        }
    };

    Ok(full_definition)
    // let temp_ident = &item.ident;
    // let temp_str = full_definition.to_string();
    // Ok(quote! {
    //     fn #temp_ident() -> &'static str {
    //         #temp_str
    //     }
    // })
}

fn expand_argument_into(
    arg_name: &ConstructedArgIdent,
    ty: &KnownArgumentType,
    tokens: &mut quote::Tokens,
) -> Result<(), Error> {
    match *ty {
        KnownArgumentType::U8SliceRef => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            tokens.append(quote! {
                #ptr_arg_name: *const u8,
                #length_arg_name: usize,
            });
        }
        KnownArgumentType::U8SliceMutRef => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            tokens.append(quote! {
                #ptr_arg_name: *mut u8,
                #length_arg_name: usize,
            });
        }
    }

    Ok(())
}

fn setup_for_argument(
    arg_name: &ConstructedArgIdent,
    ty: &KnownArgumentType,
) -> Result<quote::Tokens, Error> {
    let tokens = match *ty {
        KnownArgumentType::U8SliceRef => {
            // TODO: coordinate _ptr / _len suffixes
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            quote! {
                let #arg_name = unsafe {
                    ::std::slice::from_raw_parts(#ptr_arg_name, #length_arg_name)
                };
            }
        }
        KnownArgumentType::U8SliceMutRef => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            quote! {
                let #arg_name = unsafe {
                    ::std::slice::from_raw_parts_mut(#ptr_arg_name, #length_arg_name)
                };
            }
        }
    };

    Ok(tokens)
}

fn generate_callable_body(
    _item: &syn::Item,
    decl: &syn::FnDecl,
    code: &syn::Block,
) -> Result<quote::Tokens, Error> {
    // we'll see what works best here.
    // This set of if statements is for if we've been given a path to the implementing function.
    //
    // In this case, we want to just call the function at that path with the same arguments the
    // function declaration takes.
    if let Some(statement) = code.stmts.first() {
        if let syn::Stmt::Expr(ref inner_expr) = *statement {
            if let syn::ExprKind::Path(_, _) = inner_expr.node {
                return Ok(quote! {
                    // output the path alone so that it can be called like (path::to::func)(args)
                    (#inner_expr)
                });
            }
        }
    }

    // if it isn't our special case of a path, we can assume the full code
    // to call the inner function has been written out. We'll give the code
    // then a copy of the inputs and call it
    let mut arguments = quote::Tokens::new();
    for input in &decl.inputs {
        arguments.append(quote! {
            #input,
        });
    }
    Ok(quote! {
        // syn::Block ToTokens includes '{}' always already.
        (|#arguments| #code )
    })
}
