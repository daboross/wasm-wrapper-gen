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

use wasm_wrapper_gen_shared::{extract_func_info, get_argument_types, get_ret_type,
                              transform_macro_input_to_items, SupportedArgumentType,
                              SupportedRetType, TransformedRustIdent};


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
    let token_trees = syn::parse_token_trees(input)
        .map_err(|e| format_err!("failed to parse macro input as an item: {}", e))?;

    let ast = transform_macro_input_to_items(token_trees)?;

    let mut full_out = quote::Tokens::new();
    for item in &ast {
        let output = process_item(item)
            .with_context(|e| format!("failed to process function '{:?}': {}", item, e))?;

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
    let ret_ty = get_ret_type(decl)?;

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
        let result: #ret_ty = (#callable_body)(#arg_names_as_argument_list);
    });

    function_body.append(return_handling(&ret_ty)?);

    let func_ident = TransformedRustIdent::new(&item.ident);

    let mut real_arguments_list = quote::Tokens::new();
    for (ty, arg_name) in argument_types.iter().zip(&argument_names) {
        expand_argument_into(arg_name, ty, &mut real_arguments_list)?;
    }

    let ret_def = WrittenReturnType(ret_ty);

    let full_definition = quote! {
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn #func_ident (#real_arguments_list) #ret_def {
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
    type_type: &SupportedArgumentType,
    tokens: &mut quote::Tokens,
) -> Result<(), Error> {
    match *type_type {
        SupportedArgumentType::IntegerSliceRef(int_ty) => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            tokens.append(quote! {
                #ptr_arg_name: *const #int_ty,
                #length_arg_name: usize,
            });
        }
        SupportedArgumentType::IntegerSliceMutRef(int_ty)
        | SupportedArgumentType::IntegerVec(int_ty) => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            tokens.append(quote! {
                #ptr_arg_name: *mut #int_ty,
                #length_arg_name: usize,
            });
        }
        SupportedArgumentType::Integer(int_ty) => tokens.append(quote! {
            #arg_name: #int_ty,
        }),
    }

    Ok(())
}

struct WrittenReturnType(SupportedRetType);

impl quote::ToTokens for WrittenReturnType {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        match self.0 {
            SupportedRetType::Unit => (),
            SupportedRetType::Integer(int_ty) => {
                tokens.append(quote! { -> #int_ty });
            }
            SupportedRetType::IntegerVec(_) => {
                tokens.append(quote! { -> *const usize });
            }
        }
    }
}

fn setup_for_argument(
    arg_name: &ConstructedArgIdent,
    ty: &SupportedArgumentType,
) -> Result<quote::Tokens, Error> {
    let tokens = match *ty {
        SupportedArgumentType::IntegerSliceRef(int_ty) => {
            // TODO: coordinate _ptr / _len suffixes
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            quote! {
                let #arg_name: &[#int_ty] = unsafe {
                    ::std::slice::from_raw_parts(#ptr_arg_name, #length_arg_name)
                };
            }
        }
        SupportedArgumentType::IntegerSliceMutRef(int_ty) => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            quote! {
                let #arg_name: &mut [#int_ty] = unsafe {
                    ::std::slice::from_raw_parts_mut(#ptr_arg_name, #length_arg_name)
                };
            }
        }
        SupportedArgumentType::IntegerVec(int_ty) => {
            let ptr_arg_name = arg_name.with_suffix("_ptr");
            let length_arg_name = arg_name.with_suffix("_len");
            quote! {
                let #arg_name: Vec<#int_ty> = unsafe {
                    ::std::vec::Vec::from_raw_parts(#ptr_arg_name,
                        #length_arg_name, #length_arg_name)
                };
            }
        }
        SupportedArgumentType::Integer(_) => quote::Tokens::new(), // no setup for simple integers
    };

    Ok(tokens)
}

fn return_handling(ty: &SupportedRetType) -> Result<quote::Tokens, Error> {
    let tokens = match *ty {
        SupportedRetType::Unit | SupportedRetType::Integer(_) => quote! { result },
        SupportedRetType::IntegerVec(int_ty) => {
            quote! {
                {
                    let result_ptr = result.as_slice().as_ptr() as *mut #int_ty;
                    let result_len = result.len();
                    let result_cap = result.capacity();
                    let to_return = Box::new([result_ptr as usize, result_len, result_cap]);
                    ::std::mem::forget(result);
                    ::std::boxed::Box::into_raw(to_return) as *const usize
                }
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
