use std::fmt::{self, Display, Write};

use failure::Error;

use wasm_wrapper_gen_shared::{JsFnInfo, SupportedArgumentType, SupportedCopyTy, SupportedRetType,
                              TransformedRustIdent};

use style::{AccessStyle, Config};

use self::indented_write::WriteExt;

pub fn generate_javascript<'a, 'b, I>(config: &Config, iter: &'a I) -> Result<String, Error>
where
    &'a I: IntoIterator<Item = &'b JsFnInfo> + 'a,
{
    let mut output_buffer = String::new();
    {
        let buf = &mut output_buffer;

        write_class_definition_up_to_exports_grabbing(config, buf)?;

        {
            let buf = &mut buf.indented(config.indent * 3);
            for info in iter {
                write_func_unexport(buf, info)?;
            }
        }

        write_class_definition_post_exports_grabbing_up_to_methods(config, buf)?;

        {
            let buf = &mut buf.indented(config.indent);
            for info in iter {
                write_method(config, buf, info)?;
            }
        }

        write_class_definition_finish(config, buf)?;
    }
    Ok(output_buffer)
}

fn write_class_definition_up_to_exports_grabbing<T>(
    config: &Config,
    buf: &mut T,
) -> Result<(), Error>
where
    T: Write,
{
    write!(buf, "class {} {{\n", config.class_name)?;
    {
        let mut buf = buf.indented(config.indent);
        write!(buf, "constructor (wasm_module) {{\n")?;
        {
            let mut buf = buf.indented(config.indent);
            write!(
                buf,
                "this._mod = new WebAssembly.Instance(wasm_module, {{}});\n"
            )?;
            match config.access_style {
                AccessStyle::TypedArrays => {
                    write!(buf, "this._mem = this._mod.exports[\"memory\"];\n")?;
                }
                AccessStyle::DataView => {
                    write!(
                        buf,
                        r#"this._raw_mem = this._mod.exports["memory"];
this._mem = new DataView(this._raw_mem.buffer);
"#
                    )?;
                }
            }
            write!(
                buf,
                r#"
this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];

this._funcs = {{
"#
            )?;
        }
    }
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

fn write_class_definition_post_exports_grabbing_up_to_methods<T>(
    config: &Config,
    buf: &mut T,
) -> Result<(), Error>
where
    T: Write,
{
    write!(buf.indented(config.indent * 2), "}};\n")?;
    write!(buf.indented(config.indent), "}}\n")?;
    match config.access_style {
        AccessStyle::TypedArrays => {}
        AccessStyle::DataView => {
            let buf = &mut buf.indented(config.indent);
            write!(buf, "\n_check_mem_realloc() {{\n")?;
            {
                let buf = &mut buf.indented(config.indent);
                write!(
                    buf,
                    "if (this._mem.byteLength != this._raw_mem.byteLength) {{\n"
                )?;
                write!(
                    buf.indented(config.indent),
                    "this._mem = new DataView(this._raw_mem.buffer);\n"
                )?;
                write!(buf, "}}\n")?;
            }
            write!(buf, "}}\n")?;
        }
    }
    Ok(())
}

fn validate_argument<T, U, V>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
    failure: V,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
{
    match ty {
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerSliceMutRef(_)
        | SupportedArgumentType::IntegerVec(_) => {
            write!(buf, "if ({0} == null || isNaN({0}).length) {{\n", arg_name)?;
            write!(buf.indented(config.indent), "{}\n", failure)?;
            write!(buf, "}}\n")?;
        }
        SupportedArgumentType::Integer(_) => {
            write!(buf, "if (isNaN({0})) {{\n", arg_name)?;
            write!(buf.indented(config.indent), "{}\n", failure)?;
            write!(buf, "}}\n")?;
        }
    }

    Ok(())
}

fn prepare_argument_allocation<T, U>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    match ty {
        SupportedArgumentType::IntegerSliceRef(int_ty)
        | SupportedArgumentType::IntegerSliceMutRef(int_ty)
        | SupportedArgumentType::IntegerVec(int_ty) => match config.access_style {
            AccessStyle::TypedArrays => {
                write!(
                    buf,
                    r#"let {0}_len = {0}.length;
let {0}_byte_len = {0}_len * {1};
let {0}_ptr = this._alloc({0}_byte_len);
let {0}_view = new {2}(this._mem.buffer, {0}_ptr, {0}_byte_len);
{0}_view.set({0});
"#,
                    arg_name,
                    int_ty.size_in_bytes(),
                    javascript_typed_array_for_int(int_ty)
                )?;
            }
            AccessStyle::DataView => {
                write!(
                    buf,
                    r#"let {0}_len = {0}.length;
let {0}_byte_len = {0}_len * {1};
let {0}_ptr = this._alloc({0}_byte_len);
this._check_mem_realloc();
for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{
"#,
                    arg_name,
                    int_ty.size_in_bytes()
                )?;
                js_set_ith_ty_at(
                    buf.indented(config.indent),
                    "this._mem",
                    int_ty,
                    format_args!("{0}_ptr", arg_name),
                    format_args!("{0}_i", arg_name),
                    format_args!("{0}[{0}_i]", arg_name),
                )?;
                write!(buf, "}}\n")?;
            }
        },
        SupportedArgumentType::Integer(_) => {} // no allocation needed for integers.
    }

    Ok(())
}

fn deallocate_argument_allocation<T, U>(
    _config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    // deallocate
    match ty {
        SupportedArgumentType::Integer(_) | SupportedArgumentType::IntegerVec(_) => {}
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerSliceMutRef(_) => {
            write!(buf, "this._dealloc({0}_ptr, {0}_byte_len);\n", arg_name)?;
        }
    }

    Ok(())
}

fn propogate_argument_changes_outwards<T, U>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    // copy changes back for mutable references
    match ty {
        SupportedArgumentType::IntegerSliceMutRef(int_ty) => {
            // propagate modifications outwards.
            match config.access_style {
                AccessStyle::TypedArrays => {
                    write!(
                        buf,
                        "if ({0}_view.buffer.byteLength != this._mem.buffer.byteLength) {{\n",
                        arg_name
                    )?;
                    write!(
                        buf.indented(config.indent),
                        "{0}_view = new {1}(this._mem.buffer, {0}_ptr, {0}_byte_len);\n",
                        arg_name,
                        javascript_typed_array_for_int(int_ty)
                    )?;
                    write!(buf, "}}\n")?;
                    write!(buf, "if (typeof {0}.set == 'function') {{", arg_name)?;
                    write!(
                        buf.indented(config.indent),
                        "{0}.set({0}_view);\n",
                        arg_name
                    )?;
                    write!(buf, "}} else {{")?;
                    {
                        let mut buf = buf.indented(config.indent);

                        write!(
                            buf,
                            "for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{\n",
                            arg_name
                        )?;
                        {
                            let mut buf = buf.indented(config.indent);
                            write!(buf, "{0}[{0}_i] = ", arg_name)?;
                            if int_ty == SupportedCopyTy::Bool {
                                write!(buf, "Boolean({0}_view[{0}_i])", arg_name)?;
                            } else {
                                write!(buf, "{0}_view[{0}_i]", arg_name)?;
                            }
                            write!(buf, ";\n")?;
                        }
                        write!(buf, "}}\n")?;
                    }
                    write!(buf, "}}\n")?;
                }
                AccessStyle::DataView => {
                    write!(
                        buf,
                        "for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{\n",
                        arg_name
                    )?;
                    {
                        let mut buf = buf.indented(config.indent);
                        write!(buf, "{0}[{0}_i] = ", arg_name)?;
                        js_get_ith_ty_at(
                            &mut buf,
                            "this._mem",
                            int_ty,
                            format_args!("{0}_ptr", arg_name),
                            format_args!("{0}_i", arg_name),
                        )?;
                        write!(buf, ";\n")?;
                    }
                    write!(buf, "}}\n")?;
                }
            }
        }
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerVec(_)
        | SupportedArgumentType::Integer(_) => {}
    }

    Ok(())
}

fn write_method<T>(config: &Config, buf: &mut T, info: &JsFnInfo) -> Result<(), Error>
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
        let buf = &mut buf.indented(config.indent);
        // argument testing
        for (i, &ty) in info.args_ty.iter().enumerate() {
            validate_argument(
                config,
                buf,
                format_args!("arg{}", i),
                ty,
                "throw new Error();",
            )?;
        }
        // allocation
        for (i, &ty) in info.args_ty.iter().enumerate() {
            prepare_argument_allocation(config, buf, format_args!("arg{}", i), ty)?;
        }

        // actual function call
        write!(buf, "let result = this._funcs['{}'](", info.rust_name)?;
        let mut first_iteration = true;
        for (i, &ty) in info.args_ty.iter().enumerate() {
            if !first_iteration {
                write!(buf, ", ")?;
            }

            match ty {
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
        if config.access_style == AccessStyle::DataView {
            write!(buf, "this._check_mem_realloc();\n")?;
        }

        // cleanup (deallocation)
        for (i, &ty) in info.args_ty.iter().enumerate() {
            propogate_argument_changes_outwards(config, buf, format_args!("arg{}", i), ty)?;
            deallocate_argument_allocation(config, buf, format_args!("arg{}", i), ty)?;
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
            SupportedRetType::IntegerVec(int_ty) => match config.access_style {
                AccessStyle::TypedArrays => {
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
for (var ret_i = 0; ret_i < return_len; ret_i++) {{
"#,
                                javascript_typed_array_for_int(int_ty)
                            )?;
                            write!(
                                buf.indented(config.indent),
                                "return_value_copy.push(Boolean(return_view[ret_i]));\n"
                            )?;
                            write!(
                                buf,
                                r#"}}
"#,
                            )?;
                        }
                        _ => {
                            write!(
                                buf,
                                r#"
let return_value_copy = {0}.from(new {0}(this._mem.buffer, return_ptr, return_byte_len));
"#,
                                javascript_typed_array_for_int(int_ty)
                            )?;
                        }
                    }
                    write!(
                        buf,
                        r#"
this._dealloc(return_ptr, return_byte_cap);
this._dealloc(result_temp_ptr, result_temp_byte_len);
return return_value_copy;
"#
                    )?;
                }
                AccessStyle::DataView => {
                    write!(
                        buf,
                        r#"let result_temp_ptr = result;
let return_ptr = this._mem.getUint32(result_temp_ptr, true);
let return_len = this._mem.getUint32(result_temp_ptr + {0}, true);
let return_cap = this._mem.getUint32(result_temp_ptr + {1}, true);
let return_byte_len = return_len * {2};
let return_byte_cap = return_cap * {2};
let return_value_copy = [];
for (var ret_i = 0; ret_i < return_len; ret_i++) {{
"#,
                        SupportedCopyTy::USize.size_in_bytes(),
                        SupportedCopyTy::USize.size_in_bytes() * 2,
                        int_ty.size_in_bytes(),
                    )?;
                    {
                        let mut buf = buf.indented(config.indent);
                        write!(buf, "return_value_copy.push(")?;
                        js_get_ith_ty_at(&mut buf, "this._mem", int_ty, "return_ptr", "ret_i")?;
                        write!(buf, ");\n")?;
                    }
                    write!(
                        buf,
                        r#"}}
this._dealloc(return_ptr, return_byte_cap);
this._dealloc(result_temp_ptr, {0});
return return_value_copy;
"#,
                        SupportedCopyTy::USize.size_in_bytes() * 3
                    )?;
                }
            },
        }

        // TODO: handle return values (we can eventually do this by allocating
        // a Box<(*const ptr, usize)> of memory to store a (ptr, len) for the &[u8] returned)
    }

    write!(buf, "}}\n")?;

    Ok(())
}


fn write_class_definition_finish<T>(config: &Config, buf: &mut T) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"}}

exports = module.exports = {};
"#,
        config.class_name,
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

fn js_set_ith_ty_at<T, U, V, W, X>(
    mut buf: T,
    data_view_name: U,
    ty: SupportedCopyTy,
    ptr_name: V,
    i_name: W,
    value: X,
) -> Result<(), fmt::Error>
where
    T: fmt::Write,
    U: Display,
    V: Display,
    W: Display,
    X: Display,
{
    use self::SupportedCopyTy::*;

    let value = match ty {
        Bool => format!("Boolean({})", value),
        _ => value.to_string(),
    };

    let set_func_name = match ty {
        Bool | U8 => "setUint8",
        U16 => "setUint16",
        USize | U32 => "setUint32",
        I8 => "setInt8",
        I16 => "setInt16",
        ISize | I32 => "setInt32",
        F32 => "setFloat32",
        F64 => "setFloat64",
    };

    let offset = ty.size_in_bytes();

    write!(
        buf,
        "{}.{}({} + {} * {}, {}, true);\n",
        data_view_name,
        set_func_name,
        ptr_name,
        offset,
        i_name,
        value
    )
}

fn js_get_ith_ty_at<T, U, V, W>(
    mut buf: T,
    data_view_name: U,
    ty: SupportedCopyTy,
    ptr_name: V,
    i_name: W,
) -> Result<(), fmt::Error>
where
    T: fmt::Write,
    U: Display,
    V: Display,
    W: Display,
{
    use self::SupportedCopyTy::*;

    let get_func_name = match ty {
        Bool | U8 => "getUint8",
        U16 => "getUint16",
        USize | U32 => "getUint32",
        I8 => "getInt8",
        I16 => "getInt16",
        ISize | I32 => "getInt32",
        F32 => "getFloat32",
        F64 => "getFloat64",
    };

    let offset = ty.size_in_bytes();

    match ty {
        Bool => write!(
            buf,
            "Boolean({}.{}({} + {} * {}, true))",
            data_view_name,
            get_func_name,
            ptr_name,
            offset,
            i_name
        ),
        _ => write!(
            buf,
            "{}.{}({} + {} * {}, true)",
            data_view_name,
            get_func_name,
            ptr_name,
            offset,
            i_name
        ),
    }
}
