use std::fmt::Write;

use failure::Error;

use wasm_wrapper_gen_shared::{JsFnInfo, SupportedCopyTy, SupportedArgumentType, SupportedRetType,
                              TransformedRustIdent};

use self::indented_write::WriteExt;

pub fn generate_javascript<'a, 'b, I>(js_class_name: &str, iter: &'a I) -> Result<String, Error>
where
    &'a I: IntoIterator<Item = &'b JsFnInfo> + 'a,
{
    let mut output_buffer = String::new();
    {
        let buf = &mut output_buffer;

        write_class_definition_up_to_exports_grabbing(buf, js_class_name)?;

        {
            let buf = &mut buf.indented(12);
            for info in iter {
                write_func_unexport(buf, info)?;
            }
        }

        write_class_definition_post_exports_grabbing_up_to_methods(buf)?;

        {
            let buf = &mut buf.indented(4);
            for info in iter {
                write_method(buf, info)?;
            }
        }

        write_class_definition_finish(buf, js_class_name)?;
    }
    Ok(output_buffer)
}

fn write_class_definition_up_to_exports_grabbing<T>(
    buf: &mut T,
    js_class_name: &str,
) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"class {} {{
    constructor(wasm_module) {{
        this._mod = new WebAssembly.Instance(wasm_module, {{
            // TODO: imports
        }});
        this._mem = new Uint8Array(this._mod.exports["memory"].buffer);

        this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
        this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];

        this._funcs = {{
"#,
        js_class_name
    )?;
    Ok(())
}

fn write_func_unexport<T>(buf: &mut T, info: &JsFnInfo) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        "['{}']: this._mod.exports[\"{}\"],\n",
        info.rust_name,
        TransformedRustIdent::new(&info.rust_name)
    )?;

    Ok(())
}

fn write_class_definition_post_exports_grabbing_up_to_methods<T>(buf: &mut T) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"        }};
    }}
"#
    )?;
    Ok(())
}

fn write_method<T>(buf: &mut T, info: &JsFnInfo) -> Result<(), Error>
where
    T: Write,
{
    write!(buf, "\n{}(", info.rust_name)?;
    let mut first_iteration = true;
    for i in 0..info.args_ty.len() {
        if !first_iteration {
            write!(buf, ", ")?;
        }
        write!(buf, "arg{}", i)?;
        first_iteration = false;
    }
    write!(buf, ") {{\n")?;

    {
        let buf = &mut buf.indented(4);
        // argument testing
        for (i, ty) in info.args_ty.iter().enumerate() {
            match *ty {
                SupportedArgumentType::IntegerSliceRef(_)
                | SupportedArgumentType::IntegerSliceMutRef(_)
                | SupportedArgumentType::IntegerVec(_) => {
                    write!(
                        buf,
                        r#"if (arg{0} == null || isNaN(arg{0}.length)) {{
    throw new Error();
}}
"#,
                        i
                    )?;
                }
                SupportedArgumentType::Integer(_) => {
                    write!(
                        buf,
                        r#"if (isNaN(arg{0})) {{
    throw new Error();
}}
"#,
                        i
                    )?;
                }
            }
        }
        // allocation
        for (i, ty) in info.args_ty.iter().enumerate() {
            match *ty {
                SupportedArgumentType::IntegerSliceRef(int_ty)
                | SupportedArgumentType::IntegerSliceMutRef(int_ty)
                | SupportedArgumentType::IntegerVec(int_ty) => {
                    match int_ty {
                        SupportedCopyTy::U8 => {
                            // store length in temp variable
                            write!(buf, "let arg{0}_len = arg{0}.length;\n", i)?;
                            // allocate memory
                            write!(buf, "let arg{0}_ptr = this._alloc(arg{0}_len);\n", i)?;
                            write!(
                                buf,
                                "let arg{0}_view = this._mem.subarray\
                                 (arg{0}_ptr, arg{0}_ptr + arg{0}_len);\n",
                                i
                            )?;
                            // get a 'view' into the allocated memory
                            // and copy the argument's data into it
                            write!(buf, "arg{0}_view.set(arg{0});\n", i)?;
                        }
                        _ => {
                            write!(
                                buf,
                                r#"let arg{0}_len = arg{0}.length;
let arg{0}_byte_len = arg{0}_len * {1};
let arg{0}_ptr = this._alloc(arg{0}_byte_len);
let arg{0}_view = new {2}(this._mem.buffer, arg{0}_ptr, arg{0}_byte_len);
arg{0}_view.set(arg{0});
"#,
                                i,
                                int_ty.size_in_bytes(),
                                javascript_typed_array_for_int(int_ty)
                            )?;
                        }
                    }
                }
                SupportedArgumentType::Integer(_) => {} // no allocation needed for integers.
            }
        }

        // actual function call
        write!(buf, "let result = this._funcs['{}'](", info.rust_name)?;
        let mut first_iteration = true;
        for (i, ty) in info.args_ty.iter().enumerate() {
            if !first_iteration {
                write!(buf, ", ")?;
            }

            match *ty {
                SupportedArgumentType::IntegerSliceRef(_)
                | SupportedArgumentType::IntegerSliceMutRef(_)
                | SupportedArgumentType::IntegerVec(_) => {
                    write!(buf, "arg{0}_ptr, arg{0}_len", i)?;
                }
                SupportedArgumentType::Integer(_) => {
                    write!(buf, "arg{0}", i)?;
                }
            }
            first_iteration = false;
        }

        write!(buf, ");\n")?;

        // cleanup (deallocation)
        for (i, ty) in info.args_ty.iter().enumerate() {
            // copy changes back for mutable references
            match *ty {
                SupportedArgumentType::IntegerSliceMutRef(int_ty) => {
                    // propagate modifications outwards.
                    write!(
                        buf,
                        r#"if (typeof arg{0}.set == 'function') {{
    arg{0}.set(arg{0}_view);
}} else {{
    for (var i = 0; i < arg{0}_len; i++) {{
        arg{0}[i] = "#,
                        i
                    )?;
                    if int_ty == SupportedCopyTy::Bool {
                        write!(buf, "Boolean(arg{0}_view[i])", i)?;
                    } else {
                        write!(buf, "arg{0}_view[i]", i)?;
                    }
                    write!(buf, ";\n    }}\n}}")?;
                }
                SupportedArgumentType::IntegerSliceRef(_)
                | SupportedArgumentType::IntegerVec(_)
                | SupportedArgumentType::Integer(_) => {}
            }
            // deallocate
            match *ty {
                SupportedArgumentType::Integer(_) | SupportedArgumentType::IntegerVec(_) => {}
                SupportedArgumentType::IntegerSliceRef(int_ty)
                | SupportedArgumentType::IntegerSliceMutRef(int_ty) => match int_ty {
                    SupportedCopyTy::U8 => {
                        write!(buf, "this._dealloc(arg{0}_ptr, arg{0}_len);\n", i)?;
                    }
                    _ => {
                        write!(buf, "this._dealloc(arg{0}_ptr, arg{0}_byte_len);\n", i)?;
                    }
                },
            }
        }

        match info.ret_ty {
            SupportedRetType::Unit => {
                write!(buf, "return;\n")?;
            }
            SupportedRetType::Integer(SupportedCopyTy::Bool) => {
                write!(buf, "return Boolean(result);\n")?;
            }
            SupportedRetType::Integer(_) => {
                write!(buf, "return result;\n")?;
            }
            SupportedRetType::IntegerVec(int_ty) => {
                write!(
                    buf,
                    r#"let result_temp_ptr = result;
let result_temp_len = 3;
let result_temp_byte_len = result_temp_len * {0};
let result_temp_view = new {1}(this._mem.buffer, result, result_temp_byte_len);
let return_ptr = result_temp_view[0];
let return_len = result_temp_view[1];
let return_cap = result_temp_view[2];
let return_byte_len = return_len * {2};
let return_byte_cap = return_cap * {2};"#,
                    SupportedCopyTy::USize.size_in_bytes(),
                    javascript_typed_array_for_int(SupportedCopyTy::USize),
                    int_ty.size_in_bytes()
                )?;
                match int_ty {
                    SupportedCopyTy::Bool => {
                        write!(
                            buf,
                            r#"
let return_view = new {0}(this._mem.buffer, return_ptr, return_byte_len);
let return_value_copy = [];
for (var i = 0; i < return_len; i++) {{
    return_value_copy.push(Boolean(return_view[i]));
}}
return return_value_copy;
"#,
                            javascript_typed_array_for_int(int_ty)
                        )?;
                    }
                    _ => {
                        write!(
                            buf,
                            r#"
let return_value_copy = {0}.from(new {0}(this._mem.buffer, return_ptr, return_byte_len));
this._dealloc(return_ptr, return_byte_cap);
this._dealloc(result_temp_ptr, result_temp_byte_len);
return return_value_copy;
"#,
                            javascript_typed_array_for_int(int_ty)
                        )?;
                    }
                }
            }
        }

        // TODO: handle return values (we can eventually do this by allocating
        // a Box<(*const ptr, usize)> of memory to store a (ptr, len) for the &[u8] returned)
    }

    write!(buf, "}}\n")?;

    Ok(())
}


fn write_class_definition_finish<T>(buf: &mut T, js_class_name: &str) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"}}

exports = module.exports = {}"#,
        js_class_name
    )?;
    Ok(())
}


mod indented_write {
    use std::fmt::{self, Write};

    pub struct IdentedWriter<T: Write> {
        inner: T,
        indent: u32,
        /// True if this writer hasn't written since the last new line.
        is_primed: bool,
    }
    impl<T: Write> IdentedWriter<T> {
        pub fn new(inner: T, indent: u32) -> Self {
            IdentedWriter {
                inner,
                indent,
                is_primed: true,
            }
        }
    }

    impl<T: Write> Write for IdentedWriter<T> {
        fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
            let mut iter = s.split("\n");

            // split always returns at least one character
            let first_part = iter.next().unwrap();
            if !first_part.is_empty() {
                if self.is_primed {
                    self.is_primed = false;
                    for _ in 0..self.indent {
                        self.inner.write_str(" ")?;
                    }
                }
                self.inner.write_str(first_part)?;
            }
            // if there were any newlines, there will be more than one item.
            //
            // write each newline and then write the indent before writing.
            for part in iter {
                self.inner.write_str("\n")?;
                if part.is_empty() {
                    self.is_primed = true;
                } else {
                    self.is_primed = false;
                    for _ in 0..self.indent {
                        self.inner.write_str(" ")?;
                    }
                    self.inner.write_str(part)?;
                }
            }

            Ok(())
        }
    }

    pub trait WriteExt: Write {
        fn indented(&mut self, indent: u32) -> IdentedWriter<&mut Self> {
            IdentedWriter::new(self, indent)
        }
    }
    impl<T: Write> WriteExt for T {}
}

fn javascript_typed_array_for_int(ty: SupportedCopyTy) -> &'static str {
    use self::SupportedCopyTy::*;
    match ty {
        U8 => "Uint8Array",
        U16 => "Uint16Array",
        U32 => "Uint32Array",
        I8 => "Int8Array",
        I16 => "Int16Array",
        I32 => "Int32Array",
        USize => "Uint32Array",
        ISize => "Int32Array",
        F32 => "Float32Array",
        F64 => "Float64Array",
        Bool => "Uint8Array", // additional code needed to handle this case
    }
}
